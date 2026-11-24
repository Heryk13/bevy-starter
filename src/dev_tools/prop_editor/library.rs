use bevy::platform::collections::HashMap;
use bevy::prelude::*;
use bevy::window::FileDragAndDrop;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

use super::super::file_explorer::copy_entry;
use super::{PropEditor, PropKind};
use crate::disk::{game_data_dir, write_atomic};
use crate::screens::gameplay::map::assets_dir;

// ==== Config =================================================================

// inside the assets folder, and the asset path a prop's file starts with
pub(crate) const PROPS_DIR_NAME: &str = "props";

// ---- rescan: how often the open window reads the folder again ---------------
// ponytail: a read of one small folder a second, rather than the file watcher, which
// `load_folder` brings with a load of every model in it; watch it if folders grow large
const RESCAN_SECS: f32 = 1.0;

// ---- preferences: the developer's own, in the data folder -------------------
const PREFERENCES_FILE_NAME: &str = "prop_editor.ron";
// the last props placed, newest first
const RECENT_PROPS: usize = 20;

// ==== Plugin =================================================================

/// What the editor offers: the files in `assets/props/` and its folders the genre takes, and
/// the folders themselves, read again while the window is open so a file added shows at once;
/// the folder open, where a file dropped lands; and the developer's favourites and recents.
pub(super) fn plugin(app: &mut App) {
    app.init_resource::<PropLibrary>();
    app.init_resource::<PropFolders>();
    app.init_resource::<OpenFolder>();
    app.init_resource::<PropThumbnails>();
    app.add_systems(Startup, load_preferences);
    app.add_systems(
        Update,
        (
            (copy_dropped_props, rescan_props)
                .chain()
                .run_if(in_state(PropEditor::On)),
            save_preferences.run_if(resource_changed::<PropPreferences>),
        ),
    );
}

// ==== Library ================================================================

/// The props on disk the genre takes, in every folder, by asset path: what the window lists,
/// and what a genre makes thumbnails of.
#[derive(Resource, Default, PartialEq)]
pub(crate) struct PropLibrary(pub(crate) Vec<LibraryEntry>);

/// Every folder inside the props folder, at any depth, by its path inside it (`trees/pines`),
/// sorted.
#[derive(Resource, Default, PartialEq)]
pub(super) struct PropFolders(pub(super) Vec<String>);

/// The folder the window shows, by its path inside the props folder, empty for the props
/// folder itself: where the grid looks, and where a file dropped or a folder made lands.
#[derive(Resource, Default, PartialEq)]
pub(super) struct OpenFolder(pub(super) String);

/// A prop's file: its asset path (`props/trees/douglas_pine.glb`), its name, and its size on
/// disk.
#[derive(Clone, PartialEq, Debug)]
pub(crate) struct LibraryEntry {
    pub(crate) file: String,
    pub(crate) name: String,
    pub(crate) size: u64,
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

    /// Puts `file` first among the recents, once, keeping the last `RECENT_PROPS`.
    pub(super) fn place_recent(&mut self, file: &str) {
        self.recent.retain(|kept| kept != file);
        self.recent.insert(0, file.to_string());
        self.recent.truncate(RECENT_PROPS);
    }
}

/// Returns the props folder on disk, where dropped files are copied.
pub(crate) fn props_dir() -> PathBuf {
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

/// Copies a file dropped on the window from the OS's explorer into the folder open, when the
/// genre takes it, a taken name numbered (`pine 2.glb`); anything else is refused in the log.
/// The next rescan lists it, within a second.
fn copy_dropped_props(
    mut commands: Commands, // multiplayer
    mut drops: MessageReader<FileDragAndDrop>,
    kind: Option<Res<PropKind>>,
    open: Res<OpenFolder>,
) {
    let extensions = kind.map_or(&[][..], |kind| kind.extensions);
    for drop in drops.read() {
        let FileDragAndDrop::DroppedFile { path_buf, .. } = drop else {
            continue;
        };
        let Some(wanted) = path_buf.file_name().map(|name| name.to_string_lossy()) else {
            continue;
        };
        if path_buf.is_dir() || !is_prop_file(&wanted, extensions) {
            warn!(
                "not a prop this game takes ({}): {}",
                extensions.join(", "),
                path_buf.display()
            );
            continue;
        }
        match copy_entry(path_buf, &props_dir().join(&open.0), &wanted) {
            Ok(name) => {
                let inside = inside_folder(&open.0, &name);
                info!("added the prop {inside}");
                commands.trigger(crate::multiplayer::PropFileAdded(inside)); // multiplayer
            }
            Err(err) => error!("could not copy {} in: {err}", path_buf.display()),
        }
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
    mut folders: ResMut<PropFolders>,
    mut open: ResMut<OpenFolder>,
) {
    // the state changes as the window opens: read at once rather than a second later
    *since_secs += time.delta_secs();
    if *since_secs < RESCAN_SECS && !editor.is_changed() {
        return;
    }
    *since_secs = 0.0;
    let extensions = kind.map_or(&[][..], |kind| kind.extensions);
    let (found_props, found_folders) = read_props(extensions);
    let found_props = PropLibrary(found_props);
    if *library != found_props {
        *library = found_props;
    }
    // the folder open is gone, deleted or renamed elsewhere: back to the props folder
    if !open.0.is_empty() && !found_folders.contains(&open.0) {
        open.0.clear();
    }
    let found_folders = PropFolders(found_folders);
    if *folders != found_folders {
        *folders = found_folders;
    }
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

// ==== Helpers ================================================================

/// Returns the preferences file's path, or `None` on a machine with no data folder.
fn preferences_path() -> Option<PathBuf> {
    game_data_dir().map(|dir| dir.join(PREFERENCES_FILE_NAME))
}

/// Returns the files in the props folder and every folder in it whose extension is one of
/// `extensions`, sorted by asset path, and those folders, sorted by their path inside it. A
/// missing folder, as in an Empty game, holds none.
fn read_props(extensions: &[&str]) -> (Vec<LibraryEntry>, Vec<String>) {
    let mut entries = Vec::new();
    let mut folders = Vec::new();
    // folders still to read, by their path inside the props folder: no recursion, so no
    // depth of folders runs the stack out
    let mut to_read = vec![String::new()];
    while let Some(folder) = to_read.pop() {
        let Ok(read) = std::fs::read_dir(props_dir().join(&folder)) else {
            continue;
        };
        for entry in read.flatten() {
            let (Ok(metadata), Ok(name)) = (entry.metadata(), entry.file_name().into_string())
            else {
                continue;
            };
            let inside = inside_folder(&folder, &name);
            if metadata.is_dir() {
                folders.push(inside.clone());
                to_read.push(inside);
            } else if is_prop_file(&name, extensions) {
                entries.push(LibraryEntry {
                    file: format!("{PROPS_DIR_NAME}/{inside}"),
                    name,
                    size: metadata.len(),
                });
            }
        }
    }
    entries.sort_by(|a, b| a.file.cmp(&b.file));
    folders.sort();
    (entries, folders)
}

/// Returns the path inside the props folder of `name`, in the folder at `folder` inside it,
/// empty for the props folder itself: `trees/pine.glb`, with a `/` on every OS, as asset
/// paths are written.
pub(super) fn inside_folder(folder: &str, name: &str) -> String {
    if folder.is_empty() {
        name.to_string()
    } else {
        format!("{folder}/{name}")
    }
}

/// Returns the folder a prop's file is in, by its path inside the props folder, from its asset
/// path: `trees` for `props/trees/pine.glb`, empty for `props/pine.glb`.
pub(super) fn folder_of(file: &str) -> &str {
    let inside = file
        .strip_prefix(PROPS_DIR_NAME)
        .and_then(|rest| rest.strip_prefix('/'))
        .unwrap_or(file);
    parent_folder(inside)
}

/// Returns the folder `path` is in, inside the props folder: `trees` for `trees/pines`, empty
/// for `trees`.
pub(super) fn parent_folder(path: &str) -> &str {
    path.rsplit_once('/').map_or("", |(parent, _)| parent)
}

/// Tells whether a file named `name` is a prop the genre takes, by its extension, whatever
/// its case.
pub(crate) fn is_prop_file(name: &str, extensions: &[&str]) -> bool {
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
    fn a_path_inside_the_props_folder_names_its_folders() {
        assert_eq!(inside_folder("", "pine.glb"), "pine.glb");
        assert_eq!(
            inside_folder("trees/pines", "pine.glb"),
            "trees/pines/pine.glb"
        );
    }

    #[test]
    fn a_prop_is_in_the_folder_its_asset_path_names() {
        assert_eq!(folder_of("props/pine.glb"), "");
        assert_eq!(folder_of("props/trees/pine.glb"), "trees");
        assert_eq!(folder_of("props/trees/pines/pine.glb"), "trees/pines");
    }

    #[test]
    fn a_folder_goes_up_one_level_at_a_time() {
        assert_eq!(parent_folder("trees/pines"), "trees");
        assert_eq!(parent_folder("trees"), "");
    }

    #[test]
    fn a_recent_prop_comes_first_and_only_once() {
        let mut preferences = PropPreferences::default();
        preferences.place_recent("props/a.glb");
        preferences.place_recent("props/b.glb");
        preferences.place_recent("props/a.glb");
        assert_eq!(preferences.recent, ["props/a.glb", "props/b.glb"]);
    }

    #[test]
    fn the_recents_keep_only_the_last_ones() {
        let mut preferences = PropPreferences::default();
        for number in 0..RECENT_PROPS + 5 {
            preferences.place_recent(&format!("props/{number}.glb"));
        }
        assert_eq!(preferences.recent.len(), RECENT_PROPS);
        let newest = format!("props/{}.glb", RECENT_PROPS + 4);
        assert_eq!(preferences.recent.first(), Some(&newest));
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
