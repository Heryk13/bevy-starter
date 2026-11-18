use bevy::prelude::*;
use bevy::ui_widgets::Activate;
use serde::{Deserialize, Serialize};
use std::io;
use std::path::{Path, PathBuf};

use super::Explorer;
use super::actions::{move_entry, remove_entry};
use super::dialog::{close_explorer_dialogs, spawn_answers, spawn_question};
use super::listing::{Entry, Place, path_parts, read_folder, sort_newest_first};
use crate::disk::write_atomic;
use crate::ui::{ButtonKind, words_button};

// ==== Config =================================================================

// ---- words: English, as every dev tool's, never read in a release build -----
const EMPTY_TRASH_WORDS: &str = "Empty Trash";
const EMPTY_TRASH_QUESTION: &str = "Empty the trash? Everything in it is deleted for good.";
const DELETE_FOR_GOOD_WORDS: &str = "Delete for Good";
// between the parts of where an entry came from, as the path writes them
const FROM_SEPARATOR: &str = " › ";

// ---- the trash's own file, beside what it holds -----------------------------
const TRASH_FILE_NAME: &str = "trash.ron";

// ---- questions --------------------------------------------------------------
const QUESTION_WIDTH_PX: f32 = 420.0;

// ==== Plugin =================================================================

/// The explorer's Trash: what Delete moves there, each entry with where it came from, Restore
/// putting it back, Delete for Good and Empty Trash, each after a question.
pub(super) fn plugin(app: &mut App) {
    app.add_systems(
        Update,
        show_empty_trash_button.run_if(resource_exists::<Explorer>),
    );
    // no run_if, as the explorer's buttons work over any screen; none asks for a resource
    // that may not exist yet, since every press of every button in the game comes here
    app.add_observer(press_empty_trash);
    app.add_observer(answer_empty_trash);
    app.add_observer(answer_delete_for_good);
}

// ==== Trash ==================================================================

/// Returns Empty Trash, for the title bar, hidden until Trash is shown.
pub(super) fn empty_trash_button() -> impl Bundle {
    (
        words_button(EMPTY_TRASH_WORDS, ButtonKind::Danger),
        EmptyTrashButton,
    )
}

/// Asks whether to empty the trash: nothing in it comes back.
pub(super) fn ask_to_empty_trash(commands: &mut Commands) {
    let card = spawn_question(commands, EMPTY_TRASH_QUESTION.into(), QUESTION_WIDTH_PX);
    spawn_answers(commands, card, EMPTY_TRASH_WORDS, EmptyTrashAnswer);
}

/// Asks whether to delete the entry named `name` in the trash for good.
pub(super) fn ask_to_delete_for_good(commands: &mut Commands, name: String) {
    let question = format!("Delete «{name}» for good?");
    let card = spawn_question(commands, question, QUESTION_WIDTH_PX);
    spawn_answers(
        commands,
        card,
        DELETE_FOR_GOOD_WORDS,
        DeleteForGoodAnswer(name),
    );
}

/// Moves the entry at `inside` of `place`, whose folder is `root`, into `trash`, under a name
/// of its own there, and writes down where it came from and when.
pub(super) fn throw_away(
    trash: &Path,
    root: &Path,
    place: Place,
    inside: &Path,
    now_secs: u64,
) -> io::Result<()> {
    let wanted = file_name(inside)?;
    let name = move_entry(&root.join(inside), trash, &wanted)?;
    let mut thrown = read_thrown(trash);
    thrown.push(Thrown {
        name,
        place,
        inside: inside.to_path_buf(),
        deleted_secs: now_secs,
    });
    write_thrown(trash, &thrown)
}

/// Puts the entry named `name` in `trash` back where it came from, in the folder `root_of`
/// gives its place: under its own name, ` 2` after it when that is taken now, and its
/// folders made again when they are gone.
pub(super) fn restore(
    trash: &Path,
    name: &str,
    root_of: impl Fn(Place) -> Option<PathBuf>,
) -> io::Result<()> {
    let mut thrown = read_thrown(trash);
    let Some(index) = thrown.iter().position(|entry| entry.name == name) else {
        return Err(io::Error::other(format!(
            "the trash does not know where {name} came from"
        )));
    };
    let entry = thrown.remove(index);
    let root = root_of(entry.place)
        .ok_or_else(|| io::Error::other(format!("{} has no folder here", entry.place.name())))?;
    let folder = root.join(entry.inside.parent().unwrap_or(Path::new("")));
    move_entry(&trash.join(name), &folder, &file_name(&entry.inside)?)?;
    write_thrown(trash, &thrown)
}

/// Tells whether the trash knows where the entry named `name` came from, so it can go back.
pub(super) fn is_known(trash: &Path, name: &str) -> bool {
    read_thrown(trash).iter().any(|entry| entry.name == name)
}

/// Returns what the trash holds, the newest deleted first, each with where it came from,
/// written as the path writes it (`Gallery › trailer`), or `None` when the trash does not
/// know. Its own file is not listed.
pub(super) fn read_trash(trash: &Path) -> Vec<(Entry, Option<String>)> {
    let thrown = read_thrown(trash);
    let mut entries: Vec<Entry> = read_folder(trash)
        .into_iter()
        .filter(|entry| entry.name != TRASH_FILE_NAME)
        .map(|entry| {
            let deleted = thrown.iter().find(|known| known.name == entry.name);
            Entry {
                changed_secs: deleted.map_or(entry.changed_secs, |known| known.deleted_secs),
                ..entry
            }
        })
        .collect();
    sort_newest_first(&mut entries);
    entries
        .into_iter()
        .map(|entry| {
            let from = thrown
                .iter()
                .find(|known| known.name == entry.name)
                .map(from_text);
            (entry, from)
        })
        .collect()
}

// ==== Types ==================================================================

/// One entry of the trash, as `trash.ron` remembers it: its name in the trash, where it came
/// from, the place and its path inside it, and when it was deleted.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
struct Thrown {
    name: String,
    place: Place,
    inside: PathBuf,
    deleted_secs: u64,
}

// ==== Components =============================================================

/// Marks Empty Trash on the title bar.
#[derive(Component)]
struct EmptyTrashButton;

/// Marks the answer that empties the trash.
#[derive(Component)]
struct EmptyTrashAnswer;

/// Marks the answer that deletes the entry it names in the trash for good.
#[derive(Component)]
struct DeleteForGoodAnswer(String);

// ==== Systems ================================================================

/// Shows Empty Trash only while Trash is the place shown.
fn show_empty_trash_button(
    explorer: Res<Explorer>,
    mut buttons: Query<&mut Node, With<EmptyTrashButton>>,
) {
    let display = if explorer.place == Place::Trash {
        Display::Flex
    } else {
        Display::None
    };
    for mut node in &mut buttons {
        if node.display != display {
            node.display = display;
        }
    }
}

/// Asks whether to empty the trash when Empty Trash on the title bar is pressed.
fn press_empty_trash(
    activate: On<Activate>,
    mut commands: Commands,
    buttons: Query<(), With<EmptyTrashButton>>,
) {
    if buttons.contains(activate.entity) {
        ask_to_empty_trash(&mut commands);
    }
}

/// Closes the question and deletes everything in the trash, its own file included.
fn answer_empty_trash(
    activate: On<Activate>,
    mut commands: Commands,
    answers: Query<(), With<EmptyTrashAnswer>>,
    // an `Option`: this observer sees every button pressed in the game, before the explorer
    // has ever opened and made the resource
    explorer: Option<ResMut<Explorer>>,
) {
    let (true, Some(mut explorer)) = (answers.contains(activate.entity), explorer) else {
        return;
    };
    close_explorer_dialogs(&mut commands);
    if let Some(trash) = Place::Trash.root()
        && let Err(error) = empty_folder(&trash)
    {
        error!("could not empty the trash: {error}");
    }
    explorer.selected = None;
    explorer.disk_changes += 1;
}

/// Closes the question and deletes the entry it names for good.
fn answer_delete_for_good(
    activate: On<Activate>,
    mut commands: Commands,
    answers: Query<&DeleteForGoodAnswer>,
    explorer: Option<ResMut<Explorer>>,
) {
    let (Ok(DeleteForGoodAnswer(name)), Some(mut explorer)) =
        (answers.get(activate.entity), explorer)
    else {
        return;
    };
    close_explorer_dialogs(&mut commands);
    if let Some(trash) = Place::Trash.root()
        && let Err(error) = delete_for_good(&trash, name)
    {
        error!("could not delete {name} for good: {error}");
    }
    explorer.selected = None;
    explorer.disk_changes += 1;
}

// ==== Helpers ================================================================

/// Returns what `trash.ron` in `trash` remembers: nothing when there is none yet, and nothing
/// but a line in the log when it cannot be read, so the trash still lists what it holds.
fn read_thrown(trash: &Path) -> Vec<Thrown> {
    let path = trash.join(TRASH_FILE_NAME);
    match std::fs::read_to_string(&path) {
        Ok(text) => ron::from_str(&text).unwrap_or_else(|error| {
            error!("could not read {}: {error}", path.display());
            Vec::new()
        }),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Vec::new(),
        Err(error) => {
            error!("could not read {}: {error}", path.display());
            Vec::new()
        }
    }
}

/// Writes `thrown` into `trash.ron` in `trash`, whole or not at all.
fn write_thrown(trash: &Path, thrown: &[Thrown]) -> io::Result<()> {
    let text = ron::to_string(thrown).map_err(io::Error::other)?;
    write_atomic(&trash.join(TRASH_FILE_NAME), text)
}

/// Deletes the entry named `name` in `trash` for good, and forgets where it came from.
fn delete_for_good(trash: &Path, name: &str) -> io::Result<()> {
    remove_entry(&trash.join(name))?;
    let mut thrown = read_thrown(trash);
    thrown.retain(|entry| entry.name != name);
    write_thrown(trash, &thrown)
}

/// Deletes everything in `folder`, the folder included. A folder already gone is empty.
fn empty_folder(folder: &Path) -> io::Result<()> {
    match std::fs::remove_dir_all(folder) {
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(()),
        done => done,
    }
}

/// Returns where an entry came from, as the path writes it: its place, then its folders.
fn from_text(thrown: &Thrown) -> String {
    let folder = thrown.inside.parent().unwrap_or(Path::new(""));
    path_parts(thrown.place.name(), folder)
        .into_iter()
        .map(|(name, _)| name)
        .collect::<Vec<_>>()
        .join(FROM_SEPARATOR)
}

/// Returns the last part of `path`, the entry's own name.
fn file_name(path: &Path) -> io::Result<String> {
    path.file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .ok_or_else(|| io::Error::other(format!("{} names no entry", path.display())))
}

// ==== Tests ==================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::disk::fresh_test_dir;

    /// Returns a gallery holding `trailer/shot.png`, and an empty trash beside it.
    fn gallery_and_trash(test: &str) -> (PathBuf, PathBuf) {
        let dir = fresh_test_dir(test);
        let gallery = dir.join("gallery");
        std::fs::create_dir_all(gallery.join("trailer")).unwrap();
        std::fs::write(gallery.join("trailer").join("shot.png"), [0_u8]).unwrap();
        (gallery, dir.join("trash"))
    }

    const SHOT: &str = "trailer/shot.png";

    #[test]
    fn a_deleted_file_lands_in_the_trash_which_knows_where_it_came_from() {
        let (gallery, trash) = gallery_and_trash("bevy-starter-explorer-throw-away");
        throw_away(&trash, &gallery, Place::Gallery, Path::new(SHOT), 7).unwrap();
        assert!(!gallery.join(SHOT).exists());
        assert!(trash.join("shot.png").exists());
        let listed = read_trash(&trash);
        assert_eq!(listed.len(), 1);
        assert_eq!(listed[0].0.changed_secs, 7);
        assert_eq!(listed[0].1.as_deref(), Some("Gallery › trailer"));
    }

    #[test]
    fn a_restored_file_is_back_where_it_was() {
        let (gallery, trash) = gallery_and_trash("bevy-starter-explorer-restore");
        throw_away(&trash, &gallery, Place::Gallery, Path::new(SHOT), 0).unwrap();
        restore(&trash, "shot.png", |_| Some(gallery.clone())).unwrap();
        assert!(gallery.join(SHOT).exists());
        assert!(read_trash(&trash).is_empty());
    }

    #[test]
    fn a_file_restored_onto_a_name_taken_since_gets_a_number() {
        let (gallery, trash) = gallery_and_trash("bevy-starter-explorer-restore-taken");
        throw_away(&trash, &gallery, Place::Gallery, Path::new(SHOT), 0).unwrap();
        std::fs::write(gallery.join(SHOT), [1_u8]).unwrap();
        restore(&trash, "shot.png", |_| Some(gallery.clone())).unwrap();
        assert!(gallery.join("trailer").join("shot 2.png").exists());
    }

    #[test]
    fn a_file_restored_into_a_folder_deleted_since_makes_it_again() {
        let (gallery, trash) = gallery_and_trash("bevy-starter-explorer-restore-folder");
        throw_away(&trash, &gallery, Place::Gallery, Path::new(SHOT), 0).unwrap();
        std::fs::remove_dir(gallery.join("trailer")).unwrap();
        restore(&trash, "shot.png", |_| Some(gallery.clone())).unwrap();
        assert!(gallery.join(SHOT).exists());
    }

    #[test]
    fn a_file_deleted_for_good_is_gone_from_the_trash_and_its_file() {
        let (gallery, trash) = gallery_and_trash("bevy-starter-explorer-delete-for-good");
        throw_away(&trash, &gallery, Place::Gallery, Path::new(SHOT), 0).unwrap();
        delete_for_good(&trash, "shot.png").unwrap();
        assert!(!trash.join("shot.png").exists());
        assert!(read_thrown(&trash).is_empty());
    }

    #[test]
    fn emptying_a_folder_deletes_all_it_holds() {
        let dir = fresh_test_dir("bevy-starter-explorer-empty-trash");
        let trash = dir.join("trash");
        std::fs::create_dir_all(trash.join("old folder")).unwrap();
        std::fs::write(trash.join("old.png"), [0_u8]).unwrap();
        empty_folder(&trash).unwrap();
        assert!(!trash.exists());
    }

    #[test]
    fn emptying_a_folder_already_gone_is_no_error() {
        let dir = fresh_test_dir("bevy-starter-explorer-empty-missing-trash");
        assert!(empty_folder(&dir.join("trash")).is_ok());
    }
}
