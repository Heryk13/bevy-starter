use bevy::image::ImageFormat;
use bevy::prelude::*;
use bevy::render::view::screenshot::{Screenshot, ScreenshotCaptured};
use bevy::tasks::futures::check_ready;
use bevy::tasks::{AsyncComputeTaskPool, Task};
use std::io::Cursor;
use std::path::Path;

use super::PhotoModeUi;
use crate::dev_tools::{DEV_TOOLS_Z_INDEX, dev_window, spawn_dev_window};
use crate::disk::{game_data_dir, write_atomic};
use crate::fade::Fade;
use crate::input::{Action, ActionName, Binding, Page, action_just_pressed, register_action};
use crate::save::unix_now;
use crate::screens::Pause;
use crate::ui::{ACCENT, INK, Icon, SMALL_FONT_SIZE_PX, TEXT, custom_icon, font, muted_text};

// ==== Config =================================================================

// takes the photo: the action, and the key it starts on
pub(super) const TAKE_PHOTO: ActionName = ActionName("take_photo");
const TAKE_PHOTO_KEY: KeyCode = KeyCode::KeyE;
// English, as every dev tool's words: on the Dev page, the keys panel and photo mode's bar
pub(super) const TAKE_PHOTO_LABEL: &str = "Take Photo";

// ---- the file: a PNG in the folder the file explorer browses ----------------
// in the game's data folder; the explorer calls it Gallery
const GALLERY_DIR_NAME: &str = "gallery";
const PHOTO_EXTENSION: &str = "png";
const SECS_PER_MINUTE: u64 = 60;
const SECS_PER_HOUR: u64 = 60 * SECS_PER_MINUTE;
const SECS_PER_DAY: u64 = 24 * SECS_PER_HOUR;

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
    register_action(
        app,
        Action {
            name: TAKE_PHOTO,
            default: Binding::Key(TAKE_PHOTO_KEY),
            page: Page::Dev,
            label: Some(TAKE_PHOTO_LABEL),
        },
    );
    app.init_resource::<PhotosThisSecond>();
    // the last three run outside photo mode too: a photo still being written as photo mode
    // ends still says so, and its flash and notice still go
    app.add_systems(
        Update,
        (
            take_photo
                .run_if(action_just_pressed(TAKE_PHOTO))
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
    let Some(gallery) = game_data_dir().map(|dir| dir.join(GALLERY_DIR_NAME)) else {
        error!("no data folder to keep photos in");
        return;
    };
    let stamp = photo_stamp(unix_now().as_secs());
    if stamp == this_second.stamp {
        this_second.count += 1;
    } else {
        this_second.stamp = stamp;
        this_second.count = 1;
    }
    let path = gallery.join(photo_file_name(&this_second.stamp, this_second.count));
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

/// Returns the date and time, in UTC, `secs` seconds after 1970 began, as a photo's name
/// spells it: `2026-11-18_14-32-05`, which sorts in the order the photos were taken.
fn photo_stamp(secs: u64) -> String {
    let (year, month, day) = civil_from_days(secs / SECS_PER_DAY);
    let secs_today = secs % SECS_PER_DAY;
    let hour = secs_today / SECS_PER_HOUR;
    let minute = secs_today % SECS_PER_HOUR / SECS_PER_MINUTE;
    let second = secs_today % SECS_PER_MINUTE;
    format!("{year:04}-{month:02}-{day:02}_{hour:02}-{minute:02}-{second:02}")
}

/// Returns the year, month and day `days` days after 1970-01-01: Howard Hinnant's
/// `civil_from_days`, the usual way to turn a day count into the Gregorian calendar, with
/// its well-known constants. It counts from 0000-03-01, so a leap day falls at the end of a
/// year, in eras of 400 years, which repeat exactly; days before 1970 never come here.
fn civil_from_days(days: u64) -> (u64, u64, u64) {
    let days = days + 719_468;
    let era = days / 146_097;
    let day_of_era = days - era * 146_097;
    let year_of_era =
        (day_of_era - day_of_era / 1_460 + day_of_era / 36_524 - day_of_era / 146_096) / 365;
    let day_of_year = day_of_era - (365 * year_of_era + year_of_era / 4 - year_of_era / 100);
    let month_from_march = (5 * day_of_year + 2) / 153;
    let day = day_of_year - (153 * month_from_march + 2) / 5 + 1;
    let month = if month_from_march < 10 {
        month_from_march + 3
    } else {
        month_from_march - 9
    };
    let year = year_of_era + era * 400 + u64::from(month <= 2);
    (year, month, day)
}

/// Returns the file name of the `count`th photo taken in the second `stamp` names: the
/// stamp alone for the first, then `_2`, `_3`.
fn photo_file_name(stamp: &str, count: u32) -> String {
    if count <= 1 {
        format!("{stamp}.{PHOTO_EXTENSION}")
    } else {
        format!("{stamp}_{count}.{PHOTO_EXTENSION}")
    }
}

// ==== Tests ==================================================================

#[cfg(test)]
mod tests {
    use super::*;

    // checked against the Unix clock: 2024-01-01 is 1704067200, and the 29th of February is
    // 59 days after it
    const LEAP_DAY_SECS: u64 = 1_704_067_200 + 59 * SECS_PER_DAY;
    // 2026-01-01 is 1767225600
    const LAST_SECOND_OF_2025: u64 = 1_767_225_600 - 1;

    #[test]
    fn the_first_second_of_1970_is_its_first_day() {
        assert_eq!(photo_stamp(0), "1970-01-01_00-00-00");
    }

    #[test]
    fn a_leap_day_is_named_as_one() {
        let secs = LEAP_DAY_SECS + 12 * SECS_PER_HOUR + 34 * SECS_PER_MINUTE + 56;
        assert_eq!(photo_stamp(secs), "2024-02-29_12-34-56");
    }

    #[test]
    fn the_last_second_of_a_year_stays_in_it() {
        assert_eq!(photo_stamp(LAST_SECOND_OF_2025), "2025-12-31_23-59-59");
        assert_eq!(photo_stamp(LAST_SECOND_OF_2025 + 1), "2026-01-01_00-00-00");
    }

    #[test]
    fn a_second_photo_in_the_same_second_gets_a_name_of_its_own() {
        let stamp = "2026-11-18_14-32-05";
        assert_eq!(photo_file_name(stamp, 1), "2026-11-18_14-32-05.png");
        assert_eq!(photo_file_name(stamp, 2), "2026-11-18_14-32-05_2.png");
    }

    #[test]
    fn the_flash_is_over_once_its_time_is_up() {
        assert!(FLASH.alpha(FLASH.total_secs()) < 1e-5);
    }
}
