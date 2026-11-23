use bevy::platform::collections::HashMap;
use bevy::prelude::*;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

use super::{PropEditor, PropKind};
use crate::disk::{game_data_dir, write_atomic};
use crate::screens::gameplay::map::assets_dir;

// ==== Config =================================================================

// inside the assets folder, and the asset path a prop's file starts with
const PROPS_DIR_NAME: &str = "props";

// ---- rescan: how often the open window reads the folder again ---------------
// ponytail: a read of one small folder a second, rather than the file watcher, which
// `load_folder` brings with a load of every model in it; watch it if folders grow large
const RESCAN_SECS: f32 = 1.0;

// ---- preferences: the developer's own, in the data folder -------------------
const PREFERENCES_FILE_NAME: &str = "prop_editor.ron";

// ==== Plugin =================================================================

/// What the editor offers: the files in `assets/props/` the genre takes, read again while the
/// window is open so a file added shows at once, and the developer's favourites and recents.
pub(super) fn plugin(app: &mut App) {
    app.init_resource::<PropLibrary>();
    app.init_resource::<PropThumbnails>();
    app.add_systems(Startup, load_preferences);
    app.add_systems(
        Update,
        (
            rescan_props.run_if(in_state(PropEditor::On)),
            save_preferences.run_if(resource_changed::<PropPreferences>),
        ),
    );
}

// ==== Library ================================================================

/// The props on disk the genre takes, by name: what the window lists.
#[derive(Resource, Default, PartialEq)]
pub(super) struct PropLibrary(pub(super) Vec<LibraryEntry>);

/// A prop's file: its asset path (`props/douglas_pine.glb`), its name, and its size on disk.
#[derive(Clone, PartialEq, Debug)]
pub(super) struct LibraryEntry {
    pub(super) file: String,
    pub(super) name: String,
    pub(super) size: u64,
}

/// The thumbnail of each prop's file, by its asset path, which the genre makes: the image
/// itself in 2D, a picture of the model in 3D. A file with none shows an icon.
#[derive(Resource, Default)]
pub(crate) struct PropThumbnails(pub(crate) HashMap<String, Handle<Image>>);

/// The developer's favourites and the props they placed last, newest first, by asset path:
/// not the game's, so kept in the data folder rather than in `assets/`.
#[derive(Resource, Serialize, Deserialize, Default, Clone, PartialEq, Debug)]
pub(super) struct PropPreferences {
    pub(super) favourites: Vec<String>,
    pub(super) recent: Vec<String>,
}

impl PropPreferences {
    /// Marks `file` as a favourite, or unmarks it.
    pub(super) fn toggle_favourite(&mut self, file: &str) {
        if let Some(at) = self.favourites.iter().position(|kept| kept == file) {
            self.favourites.remove(at);
        } else {
            self.favourites.push(file.to_string());
        }
    }
}

/// Returns the props folder on disk, where dropped files are copied.
pub(super) fn props_dir() -> PathBuf {
    assets_dir().join(PROPS_DIR_NAME)
}

// ==== Systems ================================================================

/// Reads the favourites and recents, or starts with none when the file is missing or broken.
fn load_preferences(mut commands: Commands) {
    let preferences = preferences_path()
        .and_then(|path| std::fs::read_to_string(path).ok())
        .and_then(|text| ron::from_str(&text).ok())
        .unwrap_or_default();
    commands.insert_resource::<PropPreferences>(preferences);
}

/// Writes the favourites and recents whenever they change. A failure is only logged: losing
/// a star costs nothing.
fn save_preferences(preferences: Res<PropPreferences>) {
    if preferences.is_added() {
        return;
    }
    let Some(path) = preferences_path() else {
        return;
    };
    let written = ron::to_string(&*preferences)
        .map_err(std::io::Error::other)
        .and_then(|text| write_atomic(&path, text));
    if let Err(err) = written {
        error!("could not keep the prop editor's favourites: {err}");
    }
}

/// Reads the props folder as the window opens and every `RESCAN_SECS` while it is open, and
/// changes the library only when what is there changed, so the window redraws only then.
fn rescan_props(
    time: Res<Time<Real>>,
    editor: Res<State<PropEditor>>,
    mut since_secs: Local<f32>,
    kind: Option<Res<PropKind>>,
    mut library: ResMut<PropLibrary>,
) {
    // the state changes as the window opens: read at once rather than a second later
    *since_secs += time.delta_secs();
    if *since_secs < RESCAN_SECS && !editor.is_changed() {
        return;
    }
    *since_secs = 0.0;
    let extensions = kind.map_or(&[][..], |kind| kind.extensions);
    let found = PropLibrary(read_props(extensions));
    if *library != found {
        *library = found;
    }
}

// ==== Helpers ================================================================

/// Returns the preferences file's path, or `None` on a machine with no data folder.
fn preferences_path() -> Option<PathBuf> {
    game_data_dir().map(|dir| dir.join(PREFERENCES_FILE_NAME))
}

/// Returns the files in the props folder whose extension is one of `extensions`, by name. A
/// missing folder, as in an Empty game, holds none.
fn read_props(extensions: &[&str]) -> Vec<LibraryEntry> {
    let Ok(folder) = std::fs::read_dir(props_dir()) else {
        return Vec::new();
    };
    let mut entries: Vec<LibraryEntry> = folder
        .flatten()
        .filter_map(|entry| {
            let metadata = entry.metadata().ok()?;
            let name = entry.file_name().into_string().ok()?;
            (metadata.is_file() && is_prop_file(&name, extensions)).then(|| LibraryEntry {
                file: format!("{PROPS_DIR_NAME}/{name}"),
                name,
                size: metadata.len(),
            })
        })
        .collect();
    entries.sort_by(|a, b| a.name.cmp(&b.name));
    entries
}

/// Tells whether a file named `name` is a prop the genre takes, by its extension, whatever
/// its case.
pub(super) fn is_prop_file(name: &str, extensions: &[&str]) -> bool {
    std::path::Path::new(name)
        .extension()
        .and_then(|extension| extension.to_str())
        .is_some_and(|extension| {
            extensions
                .iter()
                .any(|taken| taken.eq_ignore_ascii_case(extension))
        })
}

// ==== Tests ==================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_prop_file_is_known_by_its_extension_whatever_its_case() {
        assert!(is_prop_file("pine.glb", &["glb"]));
        assert!(is_prop_file("PINE.GLB", &["glb"]));
        assert!(!is_prop_file("pine.png", &["glb"]));
        assert!(!is_prop_file("pine", &["glb"]));
    }

    #[test]
    fn a_star_marks_a_prop_and_a_second_unmarks_it() {
        let mut preferences = PropPreferences::default();
        preferences.toggle_favourite("props/a.glb");
        assert_eq!(preferences.favourites, ["props/a.glb"]);
        preferences.toggle_favourite("props/a.glb");
        assert!(preferences.favourites.is_empty());
    }
}
