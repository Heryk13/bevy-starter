use bevy::asset::RenderAssetUsages;
use bevy::image::{CompressedImageFormats, ImageFormat, ImageSampler, ImageType};
use bevy::prelude::*;
use bevy::render::view::screenshot::{Screenshot, ScreenshotCaptured};
use bevy::tasks::AsyncComputeTaskPool;
use std::io::Cursor;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use crate::disk::write_atomic;
use crate::screens::Screen;

// ==== Config =================================================================

// ---- the file: a small JPEG beside its save, under the save's own name ------
const PICTURE_EXTENSION: &str = "jpg";
// larger than a list shows it, so it stays sharp on a scaled UI, and 16:9 whatever the
// window's shape, so every row of a list frames its picture alike
const PICTURE_WIDTH_PX: u32 = 320;
const PICTURE_HEIGHT_PX: u32 = 180;

// ---- derived ----------------------------------------------------------------
// the picture's shape, for whoever frames one at another size
pub(crate) const PICTURE_ASPECT_RATIO: f32 = PICTURE_WIDTH_PX as f32 / PICTURE_HEIGHT_PX as f32;

// ==== Plugin =================================================================

pub(super) fn plugin(app: &mut App) {
    app.init_resource::<PausePicture>();
    // once in a run, under its first loading screen: every game passes through one
    // before the first pause or autosave could take a picture
    app.add_systems(
        OnEnter(Screen::Loading),
        warm_up_screenshots.run_if(run_once),
    );
    app.add_systems(OnExit(Screen::Gameplay), forget_pause_picture);
}

// ==== Pictures ===============================================================

/// Takes a picture of the window for the saves made under the pause menu about to open,
/// and keeps it in `PausePicture`. Called the frame the pause is asked for, when the window
/// still shows the game alone: the menu spawns a frame later.
pub(crate) fn take_pause_picture(commands: &mut Commands) {
    // a `PausePicture` of its own for each pause: a capture that arrives late, or after the
    // next pause was asked, fills the one it was taken for, which nobody reads any more
    let picture = PausePicture::default();
    let slot = Arc::clone(&picture.0);
    commands.insert_resource(picture);
    commands.spawn(Screenshot::primary_window()).observe(
        move |captured: On<ScreenshotCaptured>| {
            let slot = Arc::clone(&slot);
            encode_capture(captured, move |bytes| {
                if let Ok(mut slot) = slot.lock() {
                    *slot = Some(bytes);
                }
            });
        },
    );
}

/// Takes a picture of the window and writes it beside the save at `save_path` once it
/// arrives, for a save made while the game runs: the autosave. A failure is only logged,
/// since a save with no picture is still a save.
pub(crate) fn take_save_picture(commands: &mut Commands, save_path: PathBuf) {
    commands.spawn(Screenshot::primary_window()).observe(
        move |captured: On<ScreenshotCaptured>| {
            let picture_path = picture_path(&save_path);
            encode_capture(captured, move |bytes| {
                if let Err(err) = write_atomic(&picture_path, bytes) {
                    error!("could not write the picture of a save: {err}");
                }
            });
        },
    );
}

/// Gives the save at `save_path` the picture kept for this pause. With none kept, it takes
/// away the picture an overwritten save had, which would show another moment. A failure is
/// only logged.
pub(crate) fn save_pause_picture(save_path: &Path, picture: &PausePicture) {
    let picture_path = picture_path(save_path);
    let written = match picture.bytes() {
        Some(bytes) => write_atomic(&picture_path, bytes),
        None => remove_if_there(&picture_path),
    };
    if let Err(err) = written {
        error!("could not write the picture of a save: {err}");
    }
}

/// Reads the picture of the save at `save_path`, ready to be shown. `None` when the save
/// has none, as one from before pictures existed, or when its file cannot be read as one:
/// either way the list shows an empty frame, and the save loads all the same.
// ponytail: read and decoded on the main thread, a file per save, as a list opens. a small
// JPEG takes well under a millisecond, but every autosave is kept, so a player with
// hundreds of saves waits for hundreds of files. the way up is to read them in a task, or
// only for the rows the list has scrolled to
pub(crate) fn read_save_picture(save_path: &Path) -> Option<Image> {
    let bytes = std::fs::read(picture_path(save_path)).ok()?;
    decode_picture(&bytes)
}

/// Returns where the picture of the save at `save_path` is kept: beside it, under its name.
pub(super) fn picture_path(save_path: &Path) -> PathBuf {
    save_path.with_extension(PICTURE_EXTENSION)
}

/// Removes a file, and counts one that is not there as removed.
pub(super) fn remove_if_there(path: &Path) -> std::io::Result<()> {
    match std::fs::remove_file(path) {
        Err(err) if err.kind() != std::io::ErrorKind::NotFound => Err(err),
        _ => Ok(()),
    }
}

// ==== Resources ==============================================================

/// The picture of the game taken as the pause was last asked for, already as the bytes of a
/// save's picture: the last look at the game with no menu over it. Empty until the capture
/// is encoded, which happens off the main thread, and for good when it never arrives.
// shared with the task that encodes the capture, which fills it when it is done
#[derive(Resource, Default)]
pub(crate) struct PausePicture(Arc<Mutex<Option<Vec<u8>>>>);

impl PausePicture {
    /// Returns the picture's bytes, or `None` when there is none to give.
    fn bytes(&self) -> Option<Vec<u8>> {
        self.0.lock().ok().and_then(|slot| slot.clone())
    }
}

// ==== Systems ================================================================

/// Takes a screenshot nobody keeps. The first one of a run makes Bevy build the pipeline
/// that draws a captured frame back to the window, and until it is built that frame is not
/// drawn at all: taken here, the frame lost is the first of a loading screen, which comes in
/// from black, and not the player's first pause or first autosave.
fn warm_up_screenshots(mut commands: Commands) {
    // with no observer, the capture is dropped as it arrives, and Bevy despawns the entity
    commands.spawn(Screenshot::primary_window());
}

/// Lets go of the kept picture as gameplay is left, so a save in the next game never takes
/// the picture of this one.
fn forget_pause_picture(mut picture: ResMut<PausePicture>) {
    *picture = PausePicture::default();
}

// ==== Helpers ================================================================

/// Encodes a captured window off the main thread and hands the picture's bytes to `then`,
/// there too. A capture that cannot be encoded is logged, and `then` never runs.
fn encode_capture(
    mut captured: On<ScreenshotCaptured>,
    then: impl FnOnce(Vec<u8>) + Send + 'static,
) {
    // taken out of the event, not cloned: it is every pixel of the window, and this is the
    // only observer of its screenshot, so nobody reads the event after it
    let image = std::mem::take(&mut captured.event_mut().image);
    AsyncComputeTaskPool::get()
        .spawn(async move {
            match encode_picture(image) {
                Some(bytes) => then(bytes),
                None => error!("could not encode the picture of a save"),
            }
        })
        .detach();
}

/// Returns the part of a window `width` by `height` that a picture keeps: the largest
/// rectangle of the picture's shape, around the window's center.
fn picture_crop(width: u32, height: u32) -> URect {
    // the two shapes compared as cross products, so the sizes stay whole numbers
    let (crop_width, crop_height) = if width * PICTURE_HEIGHT_PX > height * PICTURE_WIDTH_PX {
        (height * PICTURE_WIDTH_PX / PICTURE_HEIGHT_PX, height)
    } else {
        (width, width * PICTURE_HEIGHT_PX / PICTURE_WIDTH_PX)
    };
    let (left, top) = ((width - crop_width) / 2, (height - crop_height) / 2);
    URect::new(left, top, left + crop_width, top + crop_height)
}

/// Returns a captured window as the bytes of a save's picture: cropped to the picture's
/// shape, shrunk to its size and encoded as a JPEG. `None` when the capture is in a format
/// that cannot be read, or the encoder is not built in.
// every call after `try_into_dynamic` is the `image` crate's, reached through the value
// Bevy returns, so `Cargo.toml` never names a crate Bevy already brings
fn encode_picture(image: Image) -> Option<Vec<u8>> {
    let crop = picture_crop(image.width(), image.height());
    let picture = image
        .try_into_dynamic()
        .ok()?
        .crop_imm(crop.min.x, crop.min.y, crop.width(), crop.height())
        .thumbnail_exact(PICTURE_WIDTH_PX, PICTURE_HEIGHT_PX)
        // a JPEG holds no alpha, and its encoder refuses a picture that has one
        .to_rgb8();
    let format = ImageFormat::Jpeg.as_image_crate_format()?;
    let mut bytes = Cursor::new(Vec::new());
    picture.write_to(&mut bytes, format).ok()?;
    Some(bytes.into_inner())
}

/// Returns the image a picture's bytes hold, or `None` when they are not a picture. Only the
/// renderer keeps it: nothing reads its pixels once it is on screen.
fn decode_picture(bytes: &[u8]) -> Option<Image> {
    Image::from_buffer(
        bytes,
        ImageType::Extension(PICTURE_EXTENSION),
        CompressedImageFormats::NONE,
        true,
        ImageSampler::Default,
        RenderAssetUsages::RENDER_WORLD,
    )
    .ok()
}

// ==== Tests ==================================================================

#[cfg(test)]
mod tests {
    use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};

    use super::*;
    use crate::disk::fresh_test_dir;

    /// Returns a window as Bevy captures one, three pictures wide and two tall, in one color.
    fn captured_window() -> Image {
        Image::new_fill(
            Extent3d {
                width: PICTURE_WIDTH_PX * 3,
                height: PICTURE_HEIGHT_PX * 2,
                depth_or_array_layers: 1,
            },
            TextureDimension::D2,
            &[40, 90, 160, 255],
            TextureFormat::Bgra8UnormSrgb,
            RenderAssetUsages::default(),
        )
    }

    // ---- picture_path -------------------------------------------------------

    #[test]
    fn picture_sits_beside_its_save_under_its_name() {
        let save = Path::new("saves").join("save_1.ron");
        let picture = picture_path(&save);
        assert_eq!(picture.parent(), save.parent());
        assert_eq!(picture.file_stem(), save.file_stem());
        assert_ne!(picture.extension(), save.extension());
    }

    // ---- picture_crop: windows sized from the picture, so they follow it ----

    #[test]
    fn wide_window_loses_its_sides() {
        let crop = picture_crop(PICTURE_WIDTH_PX * 3, PICTURE_HEIGHT_PX * 2);
        assert_eq!(
            crop.size(),
            UVec2::new(PICTURE_WIDTH_PX, PICTURE_HEIGHT_PX) * 2
        );
        assert_eq!(crop.min, UVec2::new(PICTURE_WIDTH_PX / 2, 0));
    }

    #[test]
    fn tall_window_loses_its_top_and_bottom() {
        let crop = picture_crop(PICTURE_WIDTH_PX * 2, PICTURE_HEIGHT_PX * 3);
        assert_eq!(
            crop.size(),
            UVec2::new(PICTURE_WIDTH_PX, PICTURE_HEIGHT_PX) * 2
        );
        assert_eq!(crop.min, UVec2::new(0, PICTURE_HEIGHT_PX / 2));
    }

    #[test]
    fn window_of_the_pictures_shape_is_kept_whole() {
        let crop = picture_crop(PICTURE_WIDTH_PX * 2, PICTURE_HEIGHT_PX * 2);
        assert_eq!(crop.min, UVec2::ZERO);
        assert_eq!(
            crop.size(),
            UVec2::new(PICTURE_WIDTH_PX, PICTURE_HEIGHT_PX) * 2
        );
    }

    // ---- encode_picture and decode_picture: a window out, a picture back ----

    #[test]
    fn a_captured_window_becomes_a_picture_of_the_pictures_size() {
        let bytes = encode_picture(captured_window()).unwrap();
        let picture = decode_picture(&bytes).unwrap();
        assert_eq!(picture.width(), PICTURE_WIDTH_PX);
        assert_eq!(picture.height(), PICTURE_HEIGHT_PX);
    }

    #[test]
    fn bytes_that_are_no_picture_decode_to_nothing() {
        assert!(decode_picture(b"this is not a picture").is_none());
    }

    // ---- read_save_picture: real files, in a folder of the test's own -------

    #[test]
    fn a_written_picture_reads_back() {
        let save_path = fresh_test_dir("bevy-starter-read-picture").join("save.ron");
        let bytes = encode_picture(captured_window()).unwrap();
        write_atomic(&picture_path(&save_path), bytes).unwrap();

        assert!(read_save_picture(&save_path).is_some());
    }

    #[test]
    fn a_save_without_a_picture_reads_none() {
        let save_path = fresh_test_dir("bevy-starter-read-no-picture").join("save.ron");
        assert!(read_save_picture(&save_path).is_none());
    }

    #[test]
    fn a_broken_picture_reads_none() {
        let save_path = fresh_test_dir("bevy-starter-read-broken-picture").join("save.ron");
        write_atomic(&picture_path(&save_path), "this is not a picture").unwrap();

        assert!(read_save_picture(&save_path).is_none());
    }

    // ---- PausePicture and save_pause_picture --------------------------------

    // the first bytes of a JPEG: what the picture holds does not matter to these tests
    const PICTURE_BYTES: [u8; 2] = [0xFF, 0xD8];

    #[test]
    fn pause_picture_starts_with_nothing_to_give() {
        assert_eq!(PausePicture::default().bytes(), None);
    }

    #[test]
    fn a_save_made_under_the_pause_gets_the_kept_picture() {
        let save_path = fresh_test_dir("bevy-starter-save-pause-picture").join("save.ron");
        let kept = PausePicture(Arc::new(Mutex::new(Some(PICTURE_BYTES.to_vec()))));

        save_pause_picture(&save_path, &kept);
        assert_eq!(
            std::fs::read(picture_path(&save_path)).unwrap(),
            PICTURE_BYTES
        );
    }

    #[test]
    fn a_save_with_no_kept_picture_loses_its_old_one() {
        let save_path = fresh_test_dir("bevy-starter-save-no-pause-picture").join("save.ron");
        write_atomic(&picture_path(&save_path), PICTURE_BYTES).unwrap();

        save_pause_picture(&save_path, &PausePicture::default());
        assert!(!picture_path(&save_path).exists());
    }
}
