use bevy::asset::LoadedFolder;
use bevy::prelude::*;

// ==== Helpers ================================================================

// not a plugin: the screens call this while spawning, so there is no system here

/// Returns every image in a loaded folder, in file name order. Zero-padded names
/// are what keep a sequence in step, and a folder with other files in it is fine:
/// anything that is not an image is left out.
pub(crate) fn images_in(folder: &LoadedFolder) -> Vec<Handle<Image>> {
    let mut images = Vec::new();
    for handle in &folder.handles {
        if let Ok(image) = handle.clone().try_typed::<Image>() {
            images.push(image);
        }
    }
    images.sort_by_key(|image| image.path().map(ToString::to_string));
    images
}
