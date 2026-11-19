use bevy::image::ImageFormat;
use bevy::prelude::*;
use bevy::render::view::screenshot::{Screenshot, ScreenshotCaptured};
use bevy::tasks::futures::check_ready;
use bevy::tasks::{AsyncComputeTaskPool, Task};
use std::io::Cursor;
use std::path::Path;

use super::{PhotoHint, PhotoModeUi, register_photo_hint};
use crate::dev_tools::{
    CAPTURE, CAPTURE_KEY, CAPTURE_LABEL, DEV_TOOLS_Z_INDEX, dev_window, file_stamp, gallery_dir,
    numbered_file_name, register_inner_key, spawn_dev_window,
};
use crate::disk::write_atomic;
use crate::fade::Fade;
use crate::input::{Action, Binding, Page, action_just_pressed, register_action};
use crate::save::unix_now;
use crate::screens::Pause;
use crate::ui::{ACCENT, INK, Icon, SMALL_FONT_SIZE_PX, TEXT, custom_icon, font, muted_text};

// ==== Config =================================================================

// what photo mode's bar says beside Capture's keycap, in English as every dev tool's words
const TAKE_PHOTO_WORDS: &str = "Take Photo";

// ---- the file: a PNG in the gallery, which the file explorer browses --------
const PHOTO_EXTENSION: &str = "png";

// ---- flash and shutter: the moment the photo is taken -----------------------
// quick in and out, as a camera's flash: a thin white frame at the window's very edge,
// faint enough to be felt more than seen
const FLASH: Fade = Fade {
    delay_secs: 0.0,
    fade_in_secs: 0.04,
    hold_secs: 0.06,
    fade_out_secs: 0.45,
};
const FLASH_COLOR: Color = Color::WHITE;
// the frame's alpha at its brightest
const FLASH_PEAK_ALPHA: f32 = 0.35;
// how far in from each edge the light reaches before it is gone
const FLASH_WIDTH_PX: f32 = 10.0;
// a camera's click, "camera click" by stevemotion, made mono OGG: the format Bevy's `audio`
// feature already reads, a third the size of the MP3 it came as
const SHUTTER_SOUND_PATH: &str = "dev_tools/shutter.ogg";

// ---- notice: "Photo saved" in the top right corner --------------------------
const NOTICE_TITLE: &str = "Photo saved";
const NOTICE_TOP_PX: f32 = 16.0;
const NOTICE_RIGHT_PX: f32 = 16.0;
const NOTICE_HOLD_SECS: f32 = 2.0;
const NOTICE_GAP_PX: f32 = 10.0;
const NOTICE_TILE_PX: f32 = 32.0;
const NOTICE_TILE_RADIUS_PX: f32 = 8.0;
const NOTICE_ICON_PX: f32 = 20.0;

// ==== Plugin =================================================================

/// Taking a photo in photo mode: E captures the game alone into the gallery, with a flash
/// at the edges of the screen and a shutter's click, and says where it went once it is
/// written.
pub(super) fn plugin(app: &mut App) {
    // registered here, by the first tool that reads it, so the Dev page lists it right after
    // photo mode's own key; recording reads it too, and registers nothing
    register_action(
        app,
        Action {
            name: CAPTURE,
            default: Binding::Key(CAPTURE_KEY),
            page: Page::Dev,
            label: Some(CAPTURE_LABEL),
        },
    );
    // photo mode's bar shows it once photo mode is on, and it does nothing before
    register_inner_key(app, CAPTURE);
    register_photo_hint(
        app,
        PhotoHint {
            icon: Icon::Camera,
            actions: &[CAPTURE],
            words: TAKE_PHOTO_WORDS,
        },
    );
    app.init_resource::<PhotosThisSecond>();
    // the last three run outside photo mode too: a photo still being written as photo mode
    // ends still says so, and its flash and notice still go
    app.add_systems(
        Update,
        (
            take_photo
                .run_if(action_just_pressed(CAPTURE))
                .run_if(in_state(Pause::Photo)),
            fade_flashes,
            notice_written_photos,
            expire_notices,
        ),
    );
}

// ==== Components =============================================================

/// A photo being encoded and written on another thread. It ends with the file's name, or
/// with `None` when it could not be written.
#[derive(Component)]
struct PhotoWrite(Task<Option<String>>);

/// Marks the root of the flash at the edges of the screen, with how long it has been shown.
#[derive(Component)]
struct PhotoFlash {
    shown_secs: f32,
}

/// Marks one of the flash's four strips, with the way its light fades: from its edge of the
/// window inward, as a `LinearGradient` angle.
#[derive(Component)]
struct FlashStrip {
    inward: f32,
}

/// Marks the root of the "Photo saved" notice, with how long it has been shown.
#[derive(Component)]
struct PhotoNotice {
    shown_secs: f32,
}

// ==== Resources ==============================================================

/// The second the last photo was taken in, and how many were taken in it, so a photo taken
/// in the same second as the one before gets a name of its own instead of overwriting it.
#[derive(Resource, Default)]
struct PhotosThisSecond {
    stamp: String,
    count: u32,
}

// ==== Systems ================================================================

/// Takes a photo of the game alone: hides photo mode's own windows, captures the window in
/// the same frame, so the capture is one where they are hidden, and once it arrives shows
/// them again, flashes, clicks, and writes the photo into the gallery on another thread.
fn take_photo(
    mut commands: Commands,
    mut own_ui: Query<&mut Visibility, With<PhotoModeUi>>,
    mut this_second: ResMut<PhotosThisSecond>,
) {
    let Some(gallery) = gallery_dir() else {
        error!("no data folder to keep photos in");
        return;
    };
    let stamp = file_stamp(unix_now().as_secs());
    if stamp == this_second.stamp {
        this_second.count += 1;
    } else {
        this_second.stamp = stamp;
        this_second.count = 1;
    }
    let path = gallery.join(numbered_file_name(
        &this_second.stamp,
        this_second.count,
        PHOTO_EXTENSION,
    ));
    for mut visibility in &mut own_ui {
        *visibility = Visibility::Hidden;
    }
    commands.spawn(Screenshot::primary_window()).observe(
        move |mut captured: On<ScreenshotCaptured>,
              mut own_ui: Query<&mut Visibility, With<PhotoModeUi>>,
              mut commands: Commands,
              asset_server: Res<AssetServer>| {
            for mut visibility in &mut own_ui {
                *visibility = Visibility::Inherited;
            }
            // after the capture, so neither is in the photo: the flash tells it was taken
            commands.spawn(flash());
            commands.spawn((
                AudioPlayer::new(asset_server.load(SHUTTER_SOUND_PATH)),
                PlaybackSettings::DESPAWN,
            ));
            // taken out of the event, not cloned: it is every pixel of the window, and
            // this is the only observer of its screenshot
            let image = std::mem::take(&mut captured.event_mut().image);
            let path = path.clone();
            let write = AsyncComputeTaskPool::get().spawn(async move { write_photo(image, &path) });
            commands.spawn(PhotoWrite(write));
        },
    );
}

/// Brightens each flash in and out with `FLASH`, in real time since the game is frozen, and
/// despawns it once it is over.
fn fade_flashes(
    mut commands: Commands,
    time: Res<Time<Real>>,
    mut flashes: Query<(Entity, &mut PhotoFlash, &Children)>,
    mut strips: Query<(&FlashStrip, &mut BackgroundGradient)>,
) {
    for (entity, mut flash, children) in &mut flashes {
        flash.shown_secs += time.delta_secs();
        if flash.shown_secs >= FLASH.total_secs() {
            commands.entity(entity).despawn();
            continue;
        }
        let progress = FLASH.alpha(flash.shown_secs);
        for child in children {
            if let Ok((strip, mut gradient)) = strips.get_mut(*child) {
                *gradient = strip_light(strip.inward, progress);
            }
        }
    }
}

/// Shows "Photo saved" for each photo whose file is written, in place of the one before.
fn notice_written_photos(
    mut commands: Commands,
    mut writes: Query<(Entity, &mut PhotoWrite)>,
    notices: Query<Entity, With<PhotoNotice>>,
) {
    for (entity, mut write) in &mut writes {
        let Some(written) = check_ready(&mut write.0) else {
            continue;
        };
        commands.entity(entity).despawn();
        if let Some(file_name) = written {
            for notice in &notices {
                commands.entity(notice).despawn();
            }
            spawn_dev_window(
                &mut commands,
                (
                    PhotoModeUi,
                    PhotoNotice { shown_secs: 0.0 },
                    DespawnOnExit(Pause::Photo),
                ),
                notice_window(file_name),
            );
        }
    }
}

/// Despawns each notice once it has been read, counting real time.
fn expire_notices(
    mut commands: Commands,
    time: Res<Time<Real>>,
    mut notices: Query<(Entity, &mut PhotoNotice)>,
) {
    for (entity, mut notice) in &mut notices {
        notice.shown_secs += time.delta_secs();
        if notice.shown_secs >= NOTICE_HOLD_SECS {
            commands.entity(entity).despawn();
        }
    }
}

// ==== Helpers ================================================================

// ---- the flash and the notice -----------------------------------------------

/// Returns the flash: four strips along the window's edges, each white at its edge and
/// clear on its inner side, starting invisible. It takes no click, and goes with photo mode.
fn flash() -> impl Bundle {
    let width = px(FLASH_WIDTH_PX);
    let full = percent(100);
    (
        Node {
            width: full,
            height: full,
            ..default()
        },
        // over every root the screen camera draws, under the bar's own layer
        DEV_TOOLS_Z_INDEX,
        Pickable::IGNORE,
        PhotoModeUi,
        PhotoFlash { shown_secs: 0.0 },
        DespawnOnExit(Pause::Photo),
        children![
            flash_strip(
                LinearGradient::TO_BOTTOM,
                Node {
                    top: px(0),
                    width: full,
                    height: width,
                    ..default()
                }
            ),
            flash_strip(
                LinearGradient::TO_TOP,
                Node {
                    bottom: px(0),
                    width: full,
                    height: width,
                    ..default()
                }
            ),
            flash_strip(
                LinearGradient::TO_RIGHT,
                Node {
                    left: px(0),
                    width,
                    height: full,
                    ..default()
                }
            ),
            flash_strip(
                LinearGradient::TO_LEFT,
                Node {
                    right: px(0),
                    width,
                    height: full,
                    ..default()
                }
            ),
        ],
    )
}

/// Returns one strip of the flash, placed by `place` along an edge, its light fading toward
/// `inward`. Where two strips meet, a corner gets the light of both, a little brighter.
fn flash_strip(inward: f32, place: Node) -> impl Bundle {
    (
        Node {
            position_type: PositionType::Absolute,
            ..place
        },
        strip_light(inward, 0.0),
        FlashStrip { inward },
    )
}

/// Returns a strip's light `progress` of the way to its brightest, from 0.0 to 1.0: white at
/// the window's edge, fading to clear toward `inward`.
fn strip_light(inward: f32, progress: f32) -> BackgroundGradient {
    BackgroundGradient::from(LinearGradient::new(
        inward,
        vec![
            ColorStop::new(
                FLASH_COLOR.with_alpha(FLASH_PEAK_ALPHA * progress),
                percent(0),
            ),
            // clear white, not clear black, so the light never passes through grey
            ColorStop::new(FLASH_COLOR.with_alpha(0.0), percent(100)),
        ],
    ))
}

/// Returns the notice: a camera on a white tile, "Photo saved" over the file's name.
fn notice_window(file_name: String) -> impl Bundle {
    (
        dev_window(Node {
            top: px(NOTICE_TOP_PX),
            right: px(NOTICE_RIGHT_PX),
            ..default()
        }),
        children![(
            Node {
                align_items: AlignItems::Center,
                column_gap: px(NOTICE_GAP_PX),
                ..default()
            },
            children![
                (
                    Node {
                        width: px(NOTICE_TILE_PX),
                        height: px(NOTICE_TILE_PX),
                        justify_content: JustifyContent::Center,
                        align_items: AlignItems::Center,
                        border_radius: BorderRadius::all(px(NOTICE_TILE_RADIUS_PX)),
                        ..default()
                    },
                    BackgroundColor(ACCENT),
                    children![custom_icon(Icon::Camera, NOTICE_ICON_PX, INK)],
                ),
                (
                    Node {
                        flex_direction: FlexDirection::Column,
                        ..default()
                    },
                    children![
                        (
                            Text::new(NOTICE_TITLE),
                            font(SMALL_FONT_SIZE_PX),
                            TextColor(TEXT),
                        ),
                        (Text::new(file_name), muted_text(SMALL_FONT_SIZE_PX)),
                    ],
                ),
            ],
        )],
    )
}

// ---- the file ---------------------------------------------------------------

/// Encodes a captured window as a PNG and writes it to `path`, returning the file's name.
/// A failure is only logged: a photo that did not land costs a press of E.
fn write_photo(image: Image, path: &Path) -> Option<String> {
    // every call after `try_into_dynamic` is the `image` crate's, reached through the value
    // Bevy returns, as `picture.rs` does, so `Cargo.toml` never names a crate Bevy brings
    let bytes = image.try_into_dynamic().ok().and_then(|picture| {
        let format = ImageFormat::Png.as_image_crate_format()?;
        let mut bytes = Cursor::new(Vec::new());
        // no alpha: the window has none worth keeping, and a photo viewer would show it
        picture.to_rgb8().write_to(&mut bytes, format).ok()?;
        Some(bytes.into_inner())
    });
    let Some(bytes) = bytes else {
        error!("could not encode a photo");
        return None;
    };
    if let Err(err) = write_atomic(path, bytes) {
        error!("could not write a photo: {err}");
        return None;
    }
    path.file_name()
        .map(|name| name.to_string_lossy().into_owned())
}

// ==== Tests ==================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_flash_is_over_once_its_time_is_up() {
        assert!(FLASH.alpha(FLASH.total_secs()) < 1e-5);
    }
}
