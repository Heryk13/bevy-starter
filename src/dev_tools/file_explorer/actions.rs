use bevy::prelude::*;
use bevy::ui_widgets::Activate;
use bevy::window::FileDragAndDrop;
use std::io;
use std::path::{Path, PathBuf};

use super::list::ListRow;
use super::listing::{Place, read_folder};
use super::trash::throw_away;
use super::usage::{ask_before_changing, uses_of};
use super::{DropTarget, Explorer, FileExplorer};
use crate::save::unix_now;
use crate::ui::{ButtonKind, words_button};

// ==== Config =================================================================

// ---- words: English, as every dev tool's, never read in a release build -----
const NEW_FOLDER_BUTTON_WORDS: &str = "+ Folder";
// what a new folder is called until it is renamed, ` 2` and on after it when taken
const NEW_FOLDER_NAME: &str = "New Folder";

// ==== Plugin =================================================================

/// What the explorer does to files on the disk: "+ Folder", moving by dragging, files
/// dropped from the OS copied in, and the disk calls behind every change, the menu's and the
/// rename's included.
pub(super) fn plugin(app: &mut App) {
    app.add_systems(
        Update,
        (
            show_new_folder_button.run_if(resource_exists::<Explorer>),
            // only while it is open: `Explorer` stays once it closes, and a file dropped on
            // the game then is not the explorer's
            copy_dropped_files.run_if(any_with_component::<FileExplorer>),
        ),
    );
    // no run_if, as the explorer's buttons work over any screen; none asks for a resource
    // that may not exist yet, since every press in the game comes here
    app.add_observer(press_new_folder);
    app.add_observer(drop_entry);
}

// ==== Actions ================================================================

/// Returns "+ Folder", for the title bar, shown while the place has folders to make one in.
pub(super) fn new_folder_button() -> impl Bundle {
    (
        words_button(NEW_FOLDER_BUTTON_WORDS, ButtonKind::Quiet),
        NewFolderButton,
    )
}

/// Makes `change`, unless it renames, moves or deletes something the game may load by name
/// in Assets: then it asks first, listing where, and makes it only on the answer.
pub(super) fn request_change(commands: &mut Commands, explorer: &mut Explorer, change: Change) {
    if let Some(asset) = change.changed_asset() {
        let uses = uses_of(asset);
        if !uses.is_empty() {
            ask_before_changing(commands, change, &uses);
            return;
        }
    }
    apply(&change, explorer);
}

/// Makes `change` on the disk, logging a failure, and has the explorer list it again.
pub(super) fn apply(change: &Change, explorer: &mut Explorer) {
    if let Err(error) = change_disk(change) {
        error!(
            "could not {} {}: {error}",
            change.verb(),
            change.inside().display()
        );
    }
    explorer.renaming = None;
    explorer.selected = match change {
        Change::Rename { name, .. } => Some(name.clone()),
        Change::Move { .. } | Change::Delete { .. } => None,
    };
    explorer.disk_changes += 1;
}

/// Makes a folder in the folder shown, `New Folder` or the first number after it that is
/// free, and starts renaming it.
pub(super) fn make_new_folder(explorer: &mut Explorer) {
    let Some(folder) = explorer.folder_on_disk() else {
        return;
    };
    match new_folder(&folder) {
        Ok(name) => {
            explorer.selected = Some(name.clone());
            explorer.renaming = Some(name);
            explorer.disk_changes += 1;
        }
        Err(error) => error!("could not make a folder in {}: {error}", folder.display()),
    }
}

/// Moves the file or folder `from` into `to_dir`, named `wanted` there, or ` 2` and on after
/// it when that is taken, and returns the name it got. The folder is made when it is gone.
pub(super) fn move_entry(from: &Path, to_dir: &Path, wanted: &str) -> io::Result<String> {
    std::fs::create_dir_all(to_dir)?;
    let name = free_name(wanted, from.is_dir(), &names_in(to_dir));
    let to = to_dir.join(&name);
    // a rename, while both sit on one disk; Assets may be on another disk than the data
    // folder, which a rename cannot cross, so then a copy and a removal
    // ponytail: any failure to rename falls back to the copy, so a file locked by another
    // app may be copied and then fail to go, leaving both; tell the errors apart if it shows
    if std::fs::rename(from, &to).is_err() {
        copy_all(from, &to)?;
        remove_entry(from)?;
    }
    Ok(name)
}

/// Copies the file or folder `from` into `to_dir`, as `move_entry` names it, and returns the
/// name it got. The prop editor copies a file dropped on it with it too.
pub(in crate::dev_tools) fn copy_entry(
    from: &Path,
    to_dir: &Path,
    wanted: &str,
) -> io::Result<String> {
    std::fs::create_dir_all(to_dir)?;
    let name = free_name(wanted, from.is_dir(), &names_in(to_dir));
    copy_all(from, &to_dir.join(&name))?;
    Ok(name)
}

/// Moves the file `inside` the assets folder into the Trash, which can put it back. The prop
/// editor deletes a prop's file with it.
pub(in crate::dev_tools) fn trash_asset(inside: &Path) -> io::Result<()> {
    change_disk(&Change::Delete {
        place: Place::Assets,
        inside: inside.to_path_buf(),
    })
}

/// Deletes the file or folder at `path`, everything in a folder included.
pub(super) fn remove_entry(path: &Path) -> io::Result<()> {
    if path.is_dir() {
        std::fs::remove_dir_all(path)
    } else {
        std::fs::remove_file(path)
    }
}

// ==== Types ==================================================================

/// A change to the disk an action asks for, each entry by its place and its path inside it.
/// Recents' entries are the gallery's.
#[derive(Clone, Debug, PartialEq)]
pub(super) enum Change {
    /// Gives the entry a new name in its folder.
    Rename {
        place: Place,
        inside: PathBuf,
        name: String,
    },
    /// Moves the entry into a folder of its own place, or copies it into one of another.
    Move {
        place: Place,
        inside: PathBuf,
        to_place: Place,
        to_folder: PathBuf,
    },
    /// Moves the entry into Trash.
    Delete { place: Place, inside: PathBuf },
}

impl Change {
    /// Returns what the change does, as a verb: `rename`, `move`, `delete`.
    pub(super) fn verb(&self) -> &'static str {
        match self {
            Change::Rename { .. } => "rename",
            Change::Move { .. } => "move",
            Change::Delete { .. } => "delete",
        }
    }

    /// Returns the words of the answer that makes the change though the game uses it.
    pub(super) fn anyway_words(&self) -> &'static str {
        match self {
            Change::Rename { .. } => "Rename Anyway",
            Change::Move { .. } => "Move Anyway",
            Change::Delete { .. } => "Delete Anyway",
        }
    }

    /// Returns the path inside Assets of what the change takes away from the game, renamed,
    /// moved or deleted there, or `None` when it leaves the game's assets as they are: a
    /// copy out of Assets leaves the asset where it was.
    fn changed_asset(&self) -> Option<&Path> {
        match self {
            Change::Rename {
                place: Place::Assets,
                inside,
                ..
            }
            | Change::Delete {
                place: Place::Assets,
                inside,
            }
            | Change::Move {
                place: Place::Assets,
                to_place: Place::Assets,
                inside,
                ..
            } => Some(inside),
            _ => None,
        }
    }

    /// Returns the path inside its place of the entry the change is about.
    fn inside(&self) -> &Path {
        match self {
            Change::Rename { inside, .. }
            | Change::Move { inside, .. }
            | Change::Delete { inside, .. } => inside,
        }
    }
}

// ==== Components =============================================================

/// Marks "+ Folder" on the title bar.
#[derive(Component)]
struct NewFolderButton;

// ==== Systems ================================================================

/// Shows "+ Folder" only while the place shown has folders of its own: Gallery and Assets.
fn show_new_folder_button(
    explorer: Res<Explorer>,
    mut buttons: Query<&mut Node, With<NewFolderButton>>,
) {
    let display = match explorer.place {
        Place::Gallery | Place::Assets => Display::Flex,
        Place::Recents | Place::Trash => Display::None,
    };
    for mut node in &mut buttons {
        if node.display != display {
            node.display = display;
        }
    }
}

/// Copies files and folders dropped on the window from the OS's explorer into the folder
/// shown, Recents' into the gallery. Nothing is dropped into Trash.
fn copy_dropped_files(
    mut drops: MessageReader<FileDragAndDrop>,
    explorer: Option<ResMut<Explorer>>,
) {
    let Some(mut explorer) = explorer else {
        return;
    };
    for drop in drops.read() {
        let FileDragAndDrop::DroppedFile { path_buf, .. } = drop else {
            continue;
        };
        let place = explorer.disk_place();
        if place == Place::Trash {
            warn!("nothing is dropped into Trash: {}", path_buf.display());
            continue;
        }
        let (Some(root), Some(wanted)) = (place.root(), path_buf.file_name()) else {
            continue;
        };
        let folder = root.join(&explorer.folder);
        match copy_entry(path_buf, &folder, &wanted.to_string_lossy()) {
            Ok(name) => {
                explorer.selected = Some(name);
                explorer.disk_changes += 1;
            }
            Err(error) => error!("could not copy {} in: {error}", path_buf.display()),
        }
    }
}

/// Makes a new folder when "+ Folder" is pressed.
fn press_new_folder(
    activate: On<Activate>,
    buttons: Query<(), With<NewFolderButton>>,
    explorer: Option<ResMut<Explorer>>,
) {
    if let (true, Some(mut explorer)) = (buttons.contains(activate.entity), explorer) {
        make_new_folder(&mut explorer);
    }
}

/// Moves an entry dragged from the list onto a folder, a place or a part of the path into
/// it, or copies it there when the folder is in another place.
fn drop_entry(
    drop: On<PointerDragDrop>,
    mut commands: Commands,
    rows: Query<&ListRow>,
    targets: Query<&DropTarget>,
    explorer: Option<ResMut<Explorer>>,
) {
    let (Ok(ListRow(name)), Ok(target), Some(mut explorer)) =
        (rows.get(drop.dropped), targets.get(drop.entity), explorer)
    else {
        return;
    };
    // nothing leaves Trash but by Restore
    if explorer.place == Place::Trash {
        return;
    }
    let change = Change::Move {
        place: explorer.disk_place(),
        inside: explorer.inside(name),
        to_place: target.place,
        to_folder: target.folder.clone(),
    };
    request_change(&mut commands, &mut explorer, change);
}

// ==== Helpers ================================================================

/// Makes the change on the disk.
fn change_disk(change: &Change) -> io::Result<()> {
    match change {
        Change::Rename {
            place,
            inside,
            name,
        } => {
            let from = root_of(*place)?.join(inside);
            std::fs::rename(&from, from.with_file_name(name))
        }
        Change::Move {
            place,
            inside,
            to_place,
            to_folder,
        } => {
            if !can_move(*place, inside, *to_place, to_folder) {
                return Ok(());
            }
            let from = root_of(*place)?.join(inside);
            let to_dir = root_of(*to_place)?.join(to_folder);
            let wanted = inside
                .file_name()
                .map(|name| name.to_string_lossy().into_owned())
                .unwrap_or_default();
            if place == to_place {
                move_entry(&from, &to_dir, &wanted).map(drop)
            } else {
                copy_entry(&from, &to_dir, &wanted).map(drop)
            }
        }
        Change::Delete { place, inside } => throw_away(
            &root_of(Place::Trash)?,
            &root_of(*place)?,
            *place,
            inside,
            unix_now().as_secs(),
        ),
    }
}

/// Returns the folder of `place` on the disk, or an error on a machine with no data folder.
fn root_of(place: Place) -> io::Result<PathBuf> {
    place
        .root()
        .ok_or_else(|| io::Error::other(format!("{} has no folder here", place.name())))
}

/// Makes a folder in `folder`, `New Folder` or the first number after it that is free, and
/// returns its name. `folder` is made too when it is gone, as the gallery before a photo.
pub(in crate::dev_tools) fn new_folder(folder: &Path) -> io::Result<String> {
    let name = free_name(NEW_FOLDER_NAME, true, &names_in(folder));
    std::fs::create_dir_all(folder.join(&name))?;
    Ok(name)
}

/// Copies the file or folder `from` to `to`, a folder with everything in it.
fn copy_all(from: &Path, to: &Path) -> io::Result<()> {
    if !from.is_dir() {
        return std::fs::copy(from, to).map(drop);
    }
    std::fs::create_dir_all(to)?;
    for entry in std::fs::read_dir(from)? {
        let entry = entry?;
        copy_all(&entry.path(), &to.join(entry.file_name()))?;
    }
    Ok(())
}

/// Returns the names of what `folder` holds.
fn names_in(folder: &Path) -> Vec<String> {
    read_folder(folder)
        .into_iter()
        .map(|entry| entry.name)
        .collect()
}

/// Tells whether the entry at `inside` of `place` can go into `to_folder` of `to_place`: not
/// into the folder it is already in, nor into itself or a folder of its own, which would
/// leave it nowhere. Into another place it is copied, so it always can.
fn can_move(place: Place, inside: &Path, to_place: Place, to_folder: &Path) -> bool {
    place != to_place || (inside.parent() != Some(to_folder) && !to_folder.starts_with(inside))
}

/// Returns `wanted`, or when a name in `taken` has it, whatever the case of its letters, the
/// first ` 2`, ` 3` ... that is free, before a file's extension: `tree 2.glb`, `New Folder 2`.
fn free_name(wanted: &str, is_folder: bool, taken: &[String]) -> String {
    let is_taken = |name: &str| {
        taken
            .iter()
            .any(|other| other.to_lowercase() == name.to_lowercase())
    };
    if !is_taken(wanted) {
        return wanted.to_string();
    }
    // a folder's name has no extension, though it may hold a dot
    let (stem, extension) = match wanted.rfind('.') {
        Some(dot) if dot > 0 && !is_folder => wanted.split_at(dot),
        _ => (wanted, ""),
    };
    let mut number = 2;
    loop {
        let name = format!("{stem} {number}{extension}");
        if !is_taken(&name) {
            return name;
        }
        number += 1;
    }
}

// ==== Tests ==================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::disk::fresh_test_dir;

    fn names(names: &[&str]) -> Vec<String> {
        names.iter().map(ToString::to_string).collect()
    }

    // ---- names --------------------------------------------------------------

    #[test]
    fn a_taken_name_gets_the_first_free_number_before_its_extension() {
        let taken = names(&["New Folder", "tree.glb", "tree 2.glb"]);
        assert_eq!(free_name("New Folder", true, &taken), "New Folder 2");
        assert_eq!(free_name("tree.glb", false, &taken), "tree 3.glb");
        assert_eq!(free_name("rock.glb", false, &taken), "rock.glb");
    }

    #[test]
    fn a_folder_with_a_dot_in_its_name_gets_its_number_at_the_end() {
        let taken = names(&["v1.2"]);
        assert_eq!(free_name("v1.2", true, &taken), "v1.2 2");
    }

    // ---- the disk -----------------------------------------------------------

    #[test]
    fn a_second_new_folder_is_numbered() {
        let dir = fresh_test_dir("bevy-starter-explorer-new-folder");
        assert_eq!(new_folder(&dir).unwrap(), "New Folder");
        assert_eq!(new_folder(&dir).unwrap(), "New Folder 2");
        assert!(dir.join("New Folder 2").is_dir());
    }

    #[test]
    fn a_moved_file_leaves_its_folder_and_takes_a_number_where_its_name_is_taken() {
        let dir = fresh_test_dir("bevy-starter-explorer-move");
        std::fs::create_dir_all(dir.join("props")).unwrap();
        std::fs::write(dir.join("tree.glb"), [0_u8]).unwrap();
        std::fs::write(dir.join("props").join("tree.glb"), [1_u8]).unwrap();
        let name = move_entry(&dir.join("tree.glb"), &dir.join("props"), "tree.glb").unwrap();
        assert_eq!(name, "tree 2.glb");
        assert!(!dir.join("tree.glb").exists());
        assert!(dir.join("props").join("tree 2.glb").exists());
    }

    #[test]
    fn a_folder_copied_into_another_root_keeps_all_it_holds_and_stays() {
        let dir = fresh_test_dir("bevy-starter-explorer-copy");
        let gallery = dir.join("gallery");
        std::fs::create_dir_all(gallery.join("trailer")).unwrap();
        std::fs::write(gallery.join("trailer").join("shot.png"), [0_u8]).unwrap();
        let assets = dir.join("assets");
        copy_entry(&gallery.join("trailer"), &assets, "trailer").unwrap();
        assert!(assets.join("trailer").join("shot.png").exists());
        assert!(gallery.join("trailer").join("shot.png").exists());
    }

    #[test]
    fn a_folder_never_moves_into_itself_its_own_folders_or_where_it_is() {
        let props = Path::new("props");
        let assets = Place::Assets;
        assert!(!can_move(assets, props, assets, props));
        assert!(!can_move(assets, props, assets, Path::new("props/trees")));
        assert!(!can_move(assets, props, assets, Path::new("")));
        assert!(can_move(assets, props, assets, Path::new("models")));
        assert!(can_move(assets, props, Place::Gallery, props));
    }
}
