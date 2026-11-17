use bevy::image::ImageFormat;
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

// ==== Plugin =================================================================

pub(super) fn plugin(app: &mut App) {
    app.init_resource::<PausePicture>();
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
            let image = captured.image.clone();
            let slot = Arc::clone(&slot);
            AsyncComputeTaskPool::get()
                .spawn(async move {
                    let bytes = encode_picture(image);
                    if let Ok(mut slot) = slot.lock() {
                        *slot = bytes;
                    }
                })
                .detach();
        },
    );
}

/// Takes a picture of the window and writes it beside the save at `save_path` once it
/// arrives, for a save made while the game runs: the autosave. A failure is only logged,
/// since a save with no picture is still a save.
pub(crate) fn take_save_picture(commands: &mut Commands, save_path: PathBuf) {
    commands.spawn(Screenshot::primary_window()).observe(
        move |captured: On<ScreenshotCaptured>| {
            let image = captured.image.clone();
            let picture_path = picture_path(&save_path);
            AsyncComputeTaskPool::get()
                .spawn(async move {
                    let Some(bytes) = encode_picture(image) else {
                        error!("could not encode the picture of a save");
                        return;
                    };
                    if let Err(err) = write_atomic(&picture_path, bytes) {
                        error!("could not write the picture of a save: {err}");
                    }
                })
                .detach();
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

/// Lets go of the kept picture as gameplay is left, so a save in the next game never takes
/// the picture of this one.
fn forget_pause_picture(mut picture: ResMut<PausePicture>) {
    *picture = PausePicture::default();
}

// ==== Helpers ================================================================

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

// ==== Tests ==================================================================

#[cfg(test)]
mod tests {
    use bevy::asset::RenderAssetUsages;
    use bevy::image::{CompressedImageFormats, ImageSampler, ImageType};
    use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};

    use super::*;

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

    // ---- encode_picture: a window as Bevy captures one, read back as a file -

    #[test]
    fn a_window_of_any_shape_becomes_a_picture_of_one_size() {
        let window = Image::new_fill(
            Extent3d {
                width: PICTURE_WIDTH_PX * 3,
                height: PICTURE_HEIGHT_PX * 2,
                depth_or_array_layers: 1,
            },
            TextureDimension::D2,
            &[40, 90, 160, 255],
            TextureFormat::Bgra8UnormSrgb,
            RenderAssetUsages::default(),
        );

        let bytes = encode_picture(window).unwrap();
        let picture = Image::from_buffer(
            &bytes,
            ImageType::Extension(PICTURE_EXTENSION),
            CompressedImageFormats::NONE,
            true,
            ImageSampler::Default,
            RenderAssetUsages::default(),
        )
        .unwrap();
        assert_eq!(picture.width(), PICTURE_WIDTH_PX);
        assert_eq!(picture.height(), PICTURE_HEIGHT_PX);
    }

    // ---- PausePicture -------------------------------------------------------

    #[test]
    fn pause_picture_starts_with_nothing_to_give() {
        assert_eq!(PausePicture::default().bytes(), None);
    }
}
