use bevy::prelude::*;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use crate::disk::{game_data_dir, write_atomic};

// ==== Config =================================================================

// play since the last save that quitting may lose without asking, so leaving
// right after starting or saving is not met with a question
const UNSAVED_GRACE_SECS: f32 = 30.0;
// unsaved play before the game saves by itself; a manual save starts it over
const AUTOSAVE_INTERVAL_SECS: f32 = 300.0;
// raised whenever `SaveData` changes in a way an older save cannot be read as, a field
// renamed or its meaning changed, so that save is listed as incompatible instead of loaded
// wrong; a new `GameData` field only needs its default
const SAVE_VERSION: u32 = 1;
const SAVES_DIR_NAME: &str = "saves";
// every autosave is a new file, kept like the player's own; the prefix is what tells the
// two apart, so the Save list can leave autosaves out
const AUTOSAVE_FILE_PREFIX: &str = "autosave_";

// ==== Session ================================================================

/// The game in progress: its `GameData`, how long it has been played, how long at its last
/// save, and how long at the last autosave attempt, which a failed autosave waits a full
/// interval from.
#[derive(Resource, Clone, Default)]
pub(crate) struct Session {
    pub(crate) game: GameData,
    pub(crate) play_secs: f32,
    pub(crate) saved_play_secs: f32,
    pub(crate) autosave_tried_secs: f32,
}

/// What the game itself keeps in a save: the world, the player, their progress. Empty until
/// the gameplay has something to remember. It travels the whole way already: a new game
/// starts from `default()`, every save writes it, and a loaded save hands it back in the
/// `Session`, so gameplay builds its world from `Session::game` on `OnEnter(Screen::Gameplay)`
/// and keeps it current for the next save.
// `serde(default)` lets a save written before a field existed still load, with that field
// at its default, so adding one needs no `SAVE_VERSION` bump
#[derive(Serialize, Deserialize, Clone, Default)]
#[serde(default)]
pub(crate) struct GameData {}

/// Returns whether quitting now would lose play time that is in no save.
pub(crate) fn has_unsaved_progress(session: &Session) -> bool {
    session.play_secs - session.saved_play_secs > UNSAVED_GRACE_SECS
}

/// Returns whether enough play has gone unsaved that the game should save by itself.
pub(crate) fn autosave_is_due(session: &Session) -> bool {
    session.play_secs - session.saved_play_secs.max(session.autosave_tried_secs)
        > AUTOSAVE_INTERVAL_SECS
}

/// Returns the game a save holds, with all its play time counted as saved, since it
/// was just read from a save.
pub(crate) fn loaded_session(data: &SaveData) -> Session {
    Session {
        game: data.game.clone(),
        play_secs: data.play_secs,
        saved_play_secs: data.play_secs,
        ..default()
    }
}

/// Says whether a save the player asked for was written, so whoever shows it decides how,
/// and whoever writes it never learns about that screen's widgets.
#[derive(Event)]
pub(crate) struct SaveFinished {
    pub(crate) saved: bool,
}

// ==== Save Data ==============================================================

/// What one save file holds: what the lists show about it, and the game itself. The
/// gameplay adds its fields to `GameData`, not here.
#[derive(Serialize, Deserialize)]
pub(crate) struct SaveData {
    pub(crate) version: u32,
    pub(crate) name: String,
    pub(crate) saved_at_secs: u64,
    pub(crate) play_secs: f32,
    // saves written before `GameData` existed have no `game` at all
    #[serde(default)]
    pub(crate) game: GameData,
}

/// One file in the saves folder, as far as the game could make sense of it.
pub(crate) enum SaveEntry {
    Ready(PathBuf, SaveData),
    Incompatible(PathBuf),
    Unreadable(PathBuf),
}

impl SaveEntry {
    /// Returns the file this entry was read from, whatever the game made of it.
    pub(crate) fn path(&self) -> &Path {
        match self {
            SaveEntry::Ready(path, _)
            | SaveEntry::Incompatible(path)
            | SaveEntry::Unreadable(path) => path,
        }
    }
}

/// What a save file's text turned out to be, before it is paired with its path.
enum ParsedSave {
    Ready(SaveData),
    Incompatible,
    Unreadable,
}

/// Returns what a save file's text holds: a save, one from an unknown version, or neither.
fn parse_save(text: &str) -> ParsedSave {
    match ron::from_str::<SaveData>(text) {
        Ok(save) if save.version == SAVE_VERSION => ParsedSave::Ready(save),
        Ok(_) => ParsedSave::Incompatible,
        Err(_) => ParsedSave::Unreadable,
    }
}

/// Returns the most recently saved game that can be loaded, by its own date rather than
/// its place in the list, since the folder hands files back in no promised order.
pub(crate) fn newest_ready(entries: &[SaveEntry]) -> Option<&SaveData> {
    entries
        .iter()
        .filter_map(|entry| match entry {
            SaveEntry::Ready(_, data) => Some(data),
            _ => None,
        })
        .max_by_key(|data| data.saved_at_secs)
}

/// Puts the newest save first, and the files the game cannot load, which have no date,
/// last. The folder hands files back in no promised order.
fn sort_newest_first(entries: &mut [SaveEntry]) {
    // `None < Some`, so reversed, the dated saves come first and the undated last
    entries.sort_by_key(|entry| {
        std::cmp::Reverse(match entry {
            SaveEntry::Ready(_, data) => Some(data.saved_at_secs),
            SaveEntry::Incompatible(_) | SaveEntry::Unreadable(_) => None,
        })
    });
}

// ==== Save Files =============================================================

/// Returns the folder that holds one file per save, inside the game's data folder.
pub(crate) fn saves_dir() -> Option<PathBuf> {
    game_data_dir().map(|dir| dir.join(SAVES_DIR_NAME))
}

/// Returns the time since 1970, or zero on a clock set before it: a save with an odd age
/// beats a save that never happens.
pub(crate) fn unix_now() -> Duration {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
}

/// Returns a path in `dir` for a save that does not exist yet, named by the current time
/// in milliseconds so two saves in the same second still get two files.
pub(crate) fn new_save_path(dir: &Path) -> PathBuf {
    let timestamp = unix_now().as_millis();
    dir.join(format!("save_{timestamp}.ron"))
}

/// Returns a path in `dir` for a new autosave, named like `new_save_path`'s but with
/// `AUTOSAVE_FILE_PREFIX`, so every autosave is kept.
pub(crate) fn new_autosave_path(dir: &Path) -> PathBuf {
    dir.join(format!(
        "{AUTOSAVE_FILE_PREFIX}{}.ron",
        unix_now().as_millis()
    ))
}

/// Returns whether `path` is an autosave, by its file name. A name that is not valid UTF-8
/// is never one, since `new_autosave_path` only makes plain ones.
pub(crate) fn is_autosave(path: &Path) -> bool {
    path.file_name()
        .and_then(|name| name.to_str())
        .is_some_and(|name| name.starts_with(AUTOSAVE_FILE_PREFIX))
}

/// Returns every save in the saves folder, as `list_saves` reads them. No data folder at
/// all is an empty list, the same as a folder with nothing in it.
pub(crate) fn saves_on_disk() -> Vec<SaveEntry> {
    saves_dir().map(|dir| list_saves(&dir)).unwrap_or_default()
}

/// Returns an entry for every save file in `dir`, readable or not, so the player can
/// always see and delete what is there, newest first. A missing folder is an empty list.
// takes the folder rather than finding it, so its tests can point it at a temporary one
fn list_saves(dir: &Path) -> Vec<SaveEntry> {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return Vec::new();
    };
    let mut saves = Vec::new();
    for entry in entries {
        let Ok(entry) = entry else {
            continue;
        };
        let path = entry.path();
        if path.extension() != Some("ron".as_ref()) {
            continue;
        };
        let Ok(text) = std::fs::read_to_string(&path) else {
            saves.push(SaveEntry::Unreadable(path));
            continue;
        };
        match parse_save(&text) {
            ParsedSave::Ready(data) => saves.push(SaveEntry::Ready(path, data)),
            ParsedSave::Incompatible => saves.push(SaveEntry::Incompatible(path)),
            ParsedSave::Unreadable => saves.push(SaveEntry::Unreadable(path)),
        }
    }
    sort_newest_first(&mut saves);
    saves
}

/// Reads one save file, or `None` when it is gone, unreadable or from another version.
/// Picking a save reads it again rather than trusting the list, which may be stale.
pub(crate) fn read_save(path: &Path) -> Option<SaveData> {
    let text = std::fs::read_to_string(path).ok()?;
    match parse_save(&text) {
        ParsedSave::Ready(data) => Some(data),
        ParsedSave::Incompatible | ParsedSave::Unreadable => None,
    }
}

/// Writes the session to `path` as a save stamped with the current time, whether that
/// file is new or an older save being overwritten. Marks the session as saved only once
/// the file is whole, so a failed write still counts as unsaved progress.
pub(crate) fn write_save(path: &Path, session: &mut Session, name: &str) -> std::io::Result<()> {
    let save = SaveData {
        version: SAVE_VERSION,
        name: name.to_string(),
        saved_at_secs: unix_now().as_secs(),
        play_secs: session.play_secs,
        game: session.game.clone(),
    };
    let text = ron::to_string(&save).map_err(std::io::Error::other)?;
    write_atomic(path, &text)?;
    session.saved_play_secs = session.play_secs;
    Ok(())
}

// ==== Tests ==================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_game_has_no_unsaved_progress() {
        let session = Session::default();
        assert!(!has_unsaved_progress(&session));
    }

    #[test]
    fn short_play_has_no_unsaved_progress() {
        let mut session = Session::default();
        session.play_secs = UNSAVED_GRACE_SECS / 2.0;
        assert!(!has_unsaved_progress(&session));
    }

    #[test]
    fn long_play_has_unsaved_progress() {
        let mut session = Session::default();
        session.play_secs = UNSAVED_GRACE_SECS * 2.0;
        assert!(has_unsaved_progress(&session));
    }

    #[test]
    fn saving_clears_unsaved_progress() {
        let mut session = Session::default();
        session.play_secs = UNSAVED_GRACE_SECS * 2.0;
        assert!(has_unsaved_progress(&session));
        session.saved_play_secs = session.play_secs;
        assert!(!has_unsaved_progress(&session));
    }

    #[test]
    fn autosave_waits_for_the_interval() {
        let mut session = Session::default();
        session.play_secs = AUTOSAVE_INTERVAL_SECS / 2.0;
        assert!(!autosave_is_due(&session));
    }

    #[test]
    fn autosave_is_due_after_the_interval() {
        let mut session = Session::default();
        session.play_secs = AUTOSAVE_INTERVAL_SECS * 2.0;
        assert!(autosave_is_due(&session));
    }

    #[test]
    fn saving_puts_the_autosave_off() {
        let mut session = Session::default();
        session.play_secs = AUTOSAVE_INTERVAL_SECS * 2.0;
        assert!(autosave_is_due(&session));
        session.saved_play_secs = session.play_secs;
        assert!(!autosave_is_due(&session));
    }

    #[test]
    fn failed_autosave_waits_a_full_interval() {
        let mut session = Session::default();
        session.play_secs = AUTOSAVE_INTERVAL_SECS * 2.0;
        session.autosave_tried_secs = session.play_secs;
        assert!(!autosave_is_due(&session));
        session.play_secs += AUTOSAVE_INTERVAL_SECS * 2.0;
        assert!(autosave_is_due(&session));
    }

    #[test]
    fn loaded_game_starts_with_nothing_unsaved() {
        let data = SaveData {
            version: SAVE_VERSION,
            name: String::new(),
            saved_at_secs: 0,
            play_secs: AUTOSAVE_INTERVAL_SECS * 2.0,
            game: GameData::default(),
        };
        let session = loaded_session(&data);
        assert!(!has_unsaved_progress(&session));
        assert!(!autosave_is_due(&session));
    }

    // ---- is_autosave: real paths, so a renamed prefix stays covered ---------

    #[test]
    fn new_autosaves_are_recognised() {
        let path = new_autosave_path(Path::new("saves"));
        assert!(is_autosave(&path));
    }

    #[test]
    fn player_saves_are_not_autosaves() {
        let path = new_save_path(Path::new("saves"));
        assert!(!is_autosave(&path));
    }

    // ---- parse_save: one test per kind of file ------------------------------

    #[test]
    fn save_from_this_version_is_ready() {
        let text =
            format!("(version: {SAVE_VERSION}, saved_at_secs: 0, play_secs: 0.0, name: \"test\")");
        let parsed = parse_save(&text);
        assert!(matches!(parsed, ParsedSave::Ready(_)));
    }

    #[test]
    fn save_from_another_version_is_incompatible() {
        let text = format!(
            "(version: {}, saved_at_secs: 0, play_secs: 0.0, name: \"test\")",
            SAVE_VERSION + 1
        );
        let parsed = parse_save(&text);
        assert!(matches!(parsed, ParsedSave::Incompatible));
    }

    #[test]
    fn text_that_is_not_a_save_is_unreadable() {
        let parsed = parse_save("this is not a save");
        assert!(matches!(parsed, ParsedSave::Unreadable));
    }

    // ---- newest_ready: what Continue loads ----------------------------------

    /// Builds a loadable entry saved at the given time, the only field the choice reads.
    fn ready_save(saved_at_secs: u64) -> SaveEntry {
        let data = SaveData {
            version: SAVE_VERSION,
            name: String::new(),
            saved_at_secs,
            play_secs: 0.0,
            game: GameData::default(),
        };
        SaveEntry::Ready(PathBuf::new(), data)
    }

    #[test]
    fn newest_save_wins_wherever_it_is_listed() {
        let entries = [ready_save(300), ready_save(100), ready_save(200)];
        let newest = newest_ready(&entries).map(|data| data.saved_at_secs);
        assert_eq!(newest, Some(300));
    }

    #[test]
    fn saves_that_cannot_load_are_never_chosen() {
        let entries = [
            ready_save(100),
            SaveEntry::Incompatible(PathBuf::new()),
            SaveEntry::Unreadable(PathBuf::new()),
        ];
        let newest = newest_ready(&entries).map(|data| data.saved_at_secs);
        assert_eq!(newest, Some(100));
    }

    #[test]
    fn no_loadable_save_gives_nothing_to_continue() {
        let entries = [
            SaveEntry::Incompatible(PathBuf::new()),
            SaveEntry::Unreadable(PathBuf::new()),
        ];
        assert!(newest_ready(&entries).is_none());
        assert!(newest_ready(&[]).is_none());
    }

    // ---- sort_newest_first: the order both lists show -----------------------

    #[test]
    fn lists_show_the_newest_save_first() {
        let mut entries = [ready_save(100), ready_save(300), ready_save(200)];
        sort_newest_first(&mut entries);
        let dates: Vec<_> = entries
            .iter()
            .filter_map(|entry| match entry {
                SaveEntry::Ready(_, data) => Some(data.saved_at_secs),
                _ => None,
            })
            .collect();
        assert_eq!(dates, [300, 200, 100]);
    }

    #[test]
    fn files_that_cannot_load_go_last() {
        let mut entries = [
            SaveEntry::Unreadable(PathBuf::new()),
            ready_save(100),
            SaveEntry::Incompatible(PathBuf::new()),
        ];
        sort_newest_first(&mut entries);
        assert!(matches!(entries[0], SaveEntry::Ready(..)));
        assert!(!matches!(entries[1], SaveEntry::Ready(..)));
        assert!(!matches!(entries[2], SaveEntry::Ready(..)));
    }

    // ---- list_saves: a real folder per test, never shared -------------------

    #[test]
    fn every_kind_of_save_file_is_listed() {
        let dir = std::env::temp_dir().join("bevy-starter-list-saves");
        std::fs::create_dir_all(&dir).unwrap();

        let ready_path = dir.join("ready.ron");
        write_atomic(
            &ready_path,
            &format!("(version: {SAVE_VERSION}, saved_at_secs: 0, play_secs: 0.0, name: \"test\")"),
        )
        .unwrap();

        let incompatible_path = dir.join("incompatible.ron");
        write_atomic(
            &incompatible_path,
            &format!(
                "(version: {}, saved_at_secs: 0, play_secs: 0.0, name: \"test\")",
                SAVE_VERSION + 1
            ),
        )
        .unwrap();

        let unreadable_path = dir.join("unreadable.ron");
        write_atomic(&unreadable_path, "this is not a save").unwrap();

        let saves = list_saves(&dir);
        assert_eq!(saves.len(), 3);
        assert!(
            saves
                .iter()
                .any(|s| matches!(s, SaveEntry::Ready(path, _) if *path == ready_path))
        );
        assert!(
            saves
                .iter()
                .any(|s| matches!(s, SaveEntry::Incompatible(path) if *path == incompatible_path))
        );
        assert!(
            saves
                .iter()
                .any(|s| matches!(s, SaveEntry::Unreadable(path) if *path == unreadable_path))
        );
    }

    #[test]
    fn missing_folder_lists_nothing() {
        let dir = std::env::temp_dir().join("bevy-starter-missing-folder");
        if dir.exists() {
            std::fs::remove_dir_all(&dir).unwrap();
        }
        let saves = list_saves(&dir);
        assert!(saves.is_empty());
    }

    #[test]
    fn written_save_lists_back_ready() {
        let dir = std::env::temp_dir().join("bevy-starter-written-save");
        std::fs::create_dir_all(&dir).unwrap();

        let mut session = Session::default();
        session.play_secs = UNSAVED_GRACE_SECS * 2.0;
        let name = "save";
        write_save(&dir.join("ready.ron"), &mut session, name).unwrap();
        assert!(!has_unsaved_progress(&session));

        let saves = list_saves(&dir);
        assert_eq!(saves.len(), 1);
        let Some(SaveEntry::Ready(_, save)) = saves.first() else {
            panic!("the written save did not list back as ready");
        };
        assert!((save.play_secs - session.play_secs).abs() < 0.001);
        assert_eq!(save.name, name);
    }

    #[test]
    fn written_save_reads_back() {
        let dir = std::env::temp_dir().join("bevy-starter-read-save");
        let path = dir.join("save.ron");
        let mut session = Session::default();
        session.play_secs = UNSAVED_GRACE_SECS * 2.0;
        write_save(&path, &mut session, "save").unwrap();

        let Some(save) = read_save(&path) else {
            panic!("the written save did not read back");
        };
        assert!((save.play_secs - session.play_secs).abs() < 0.001);
    }

    #[test]
    fn missing_save_reads_as_nothing() {
        let path = std::env::temp_dir().join("bevy-starter-no-such-save.ron");
        if path.exists() {
            std::fs::remove_file(&path).unwrap();
        }
        assert!(read_save(&path).is_none());
    }

    #[test]
    fn save_is_written_into_a_missing_folder() {
        let dir = std::env::temp_dir().join("bevy-starter-write-save-missing-folder");
        if dir.exists() {
            std::fs::remove_dir_all(&dir).unwrap();
        }

        let mut session = Session::default();
        session.play_secs = 123.4;
        let name = "save";
        write_save(&dir.join("ready.ron"), &mut session, name).unwrap();

        let saves = list_saves(&dir);
        assert_eq!(saves.len(), 1);
        let Some(SaveEntry::Ready(_, save)) = saves.first() else {
            panic!("the written save did not list back as ready");
        };
        assert!((save.play_secs - session.play_secs).abs() < 0.001);
        assert_eq!(save.name, name);
    }
}
