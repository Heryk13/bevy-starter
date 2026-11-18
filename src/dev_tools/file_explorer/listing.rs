use bevy::asset::io::file::FileAssetReader;
use bevy::prelude::*;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::time::UNIX_EPOCH;

use crate::dev_tools::{
    SECS_PER_DAY, SECS_PER_HOUR, SECS_PER_MINUTE, civil_from_days, gallery_dir,
};
use crate::disk::game_data_dir;
use crate::ui::Icon;

// ==== Config =================================================================

// ---- places: where Assets and Trash are -------------------------------------
// the folder Bevy loads from, `AssetPlugin::file_path`, left at its default
const ASSETS_DIR_NAME: &str = "assets";
// in the game's data folder, beside the gallery
const TRASH_DIR_NAME: &str = "trash";

// ---- sizes: the unit that keeps the number short ----------------------------
// 1024, as Windows' own explorer counts, so the two agree on a file's size
const BYTES_PER_UNIT: f64 = 1024.0;
const SIZE_UNITS: [&str; 4] = ["KB", "MB", "GB", "TB"];
// a size under this many of its unit keeps one decimal: 2.4 MB, but 980 KB
const ONE_DECIMAL_UNDER: f64 = 10.0;

// ---- kinds: what a file is, from its extension ------------------------------
const IMAGE_EXTENSIONS: [&str; 4] = ["png", "jpg", "jpeg", "ktx2"];
const VIDEO_EXTENSIONS: [&str; 3] = ["mp4", "webm", "mkv"];
const MODEL_EXTENSIONS: [&str; 2] = ["glb", "gltf"];
const SOUND_EXTENSIONS: [&str; 4] = ["ogg", "wav", "mp3", "flac"];

// ==== Disk ===================================================================

/// Returns what `folder` holds, folders first and each by name. A folder that is not there
/// yet, as the gallery before the first photo, holds nothing; any other failure is logged
/// and holds nothing too, so the list always shows what it could read.
pub(super) fn read_folder(folder: &Path) -> Vec<Entry> {
    let reading = match std::fs::read_dir(folder) {
        Ok(reading) => reading,
        Err(error) => {
            if error.kind() != std::io::ErrorKind::NotFound {
                error!("could not read {}: {error}", folder.display());
            }
            return Vec::new();
        }
    };
    let mut entries: Vec<Entry> = reading
        .filter_map(Result::ok)
        .filter_map(|dir_entry| read_entry(&dir_entry.path()))
        .collect();
    sort_entries(&mut entries);
    entries
}

/// Returns the file or folder at `path` as the list shows it, named by its file name, or
/// `None` when the disk will not say what it is.
pub(super) fn read_entry(path: &Path) -> Option<Entry> {
    let metadata = std::fs::metadata(path).ok()?;
    let changed_secs = metadata
        .modified()
        .ok()
        .and_then(|time| time.duration_since(UNIX_EPOCH).ok())
        .map_or(0, |since| since.as_secs());
    Some(Entry {
        name: path.file_name()?.to_string_lossy().into_owned(),
        is_folder: metadata.is_dir(),
        changed_secs,
        size_bytes: metadata.len(),
    })
}

/// Returns every file under `root`, its folders' included, newest first, each named by its
/// path inside `root` (`trailer/shot.png`), so the list shows where it was sorted into and
/// the explorer finds it from `root` alone.
// ponytail: walks the whole gallery each time Recents is shown, links to folders aside. Fine
// for photos taken by hand; a gallery of thousands would want the walk on a task
pub(super) fn read_recents(root: &Path) -> Vec<Entry> {
    let mut entries = Vec::new();
    let mut folders = vec![PathBuf::new()];
    while let Some(folder) = folders.pop() {
        for entry in read_folder(&root.join(&folder)) {
            let inside = folder.join(&entry.name);
            // a link to a folder, as a shortcut back to one holding it, is not walked into:
            // the walk would go round it for ever
            if entry.is_folder && is_link(&root.join(&inside)) {
                continue;
            }
            if entry.is_folder {
                folders.push(inside);
            } else {
                entries.push(Entry {
                    name: inside.to_string_lossy().replace('\\', "/"),
                    ..entry
                });
            }
        }
    }
    sort_newest_first(&mut entries);
    entries
}

/// Tells whether `path` is a link to somewhere else, a symbolic link or a Windows junction,
/// rather than a file or a folder of its own.
fn is_link(path: &Path) -> bool {
    std::fs::symlink_metadata(path).is_ok_and(|metadata| metadata.file_type().is_symlink())
}

/// Opens `path` with the app the OS opens it with when it is double clicked: a photo in its
/// viewer, a video in its player. A failure to start one is only logged.
pub(super) fn open_with_os(path: &Path) {
    let program = opener_for(std::env::consts::OS);
    if let Err(error) = std::process::Command::new(program).arg(path).spawn() {
        error!("could not open {} with {program}: {error}", path.display());
    }
}

// ==== Types ==================================================================

/// Where the explorer looks: a folder on the disk the path never climbs out of, or Recents,
/// every photo of the gallery at once.
// written into `trash.ron`, beside each entry of the trash, as where it came from
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub(super) enum Place {
    Recents,
    Gallery,
    Assets,
    Trash,
}

impl Place {
    /// Returns the place's name, as the sidebar and the path write it.
    pub(super) fn name(self) -> &'static str {
        match self {
            Place::Recents => "Recents",
            Place::Gallery => "Gallery",
            Place::Assets => "Assets",
            Place::Trash => "Trash",
        }
    }

    /// Returns the folder on the disk the place shows, Recents the gallery's, or `None` on
    /// a machine with no data folder. Assets is the folder Bevy loads from, the repository's
    /// `assets/` under `just run`.
    pub(super) fn root(self) -> Option<PathBuf> {
        match self {
            Place::Recents | Place::Gallery => gallery_dir(),
            Place::Assets => Some(FileAssetReader::get_base_path().join(ASSETS_DIR_NAME)),
            Place::Trash => game_data_dir().map(|dir| dir.join(TRASH_DIR_NAME)),
        }
    }
}

/// One file or folder in the folder shown, with what the list writes beside it.
#[derive(Clone, Debug, PartialEq)]
pub(super) struct Entry {
    pub(super) name: String,
    pub(super) is_folder: bool,
    // seconds since 1970 of its last change, zero when the disk will not say
    pub(super) changed_secs: u64,
    pub(super) size_bytes: u64,
}

/// What an entry is, which picks its icon.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Kind {
    Folder,
    Image,
    Video,
    Model,
    Sound,
    Other,
}

impl Kind {
    /// Returns the icon an entry of this kind is listed with.
    pub(super) fn icon(self) -> Icon {
        match self {
            Kind::Folder => Icon::Folder,
            Kind::Image => Icon::Image,
            Kind::Video => Icon::Film,
            Kind::Model => Icon::Box,
            Kind::Sound => Icon::Music,
            Kind::Other => Icon::File,
        }
    }
}

// ==== Helpers ================================================================

/// Puts folders before files, and each by name, the case of its letters aside, as the
/// Windows explorer lists them.
pub(super) fn sort_entries(entries: &mut [Entry]) {
    entries.sort_by(|a, b| {
        b.is_folder
            .cmp(&a.is_folder)
            .then_with(|| a.name.to_lowercase().cmp(&b.name.to_lowercase()))
    });
}

/// Puts the newest first, the order Recents lists them in; two of the same second by name.
pub(super) fn sort_newest_first(entries: &mut [Entry]) {
    entries.sort_by(|a, b| {
        b.changed_secs
            .cmp(&a.changed_secs)
            .then_with(|| a.name.cmp(&b.name))
    });
}

/// Returns the program that opens a file in its default app on the OS named `os`, as
/// `std::env::consts::OS` names it: Windows' explorer, macOS' `open`, and the desktop's
/// `xdg-open` anywhere else.
pub(super) fn opener_for(os: &str) -> &'static str {
    match os {
        "windows" => "explorer",
        "macos" => "open",
        _ => "xdg-open",
    }
}

/// Returns what an entry is: a folder, or a file by its extension, whatever its case.
pub(super) fn kind_of(entry: &Entry) -> Kind {
    if entry.is_folder {
        return Kind::Folder;
    }
    let extension = Path::new(&entry.name)
        .extension()
        .map(|extension| extension.to_string_lossy().to_lowercase())
        .unwrap_or_default();
    let is = |extensions: &[&str]| extensions.contains(&extension.as_str());
    if is(&IMAGE_EXTENSIONS) {
        Kind::Image
    } else if is(&VIDEO_EXTENSIONS) {
        Kind::Video
    } else if is(&MODEL_EXTENSIONS) {
        Kind::Model
    } else if is(&SOUND_EXTENSIONS) {
        Kind::Sound
    } else {
        Kind::Other
    }
}

/// Returns a file's size as the list writes it: `12 B`, `980 KB`, `2.4 MB`.
pub(super) fn size_text(bytes: u64) -> String {
    // a file's size fits an f64 to the byte below 9 PB, far past any file here
    let mut size = bytes as f64;
    if size < BYTES_PER_UNIT {
        return format!("{bytes} B");
    }
    let mut unit = SIZE_UNITS[0];
    for next_unit in SIZE_UNITS {
        size /= BYTES_PER_UNIT;
        unit = next_unit;
        if size < BYTES_PER_UNIT {
            break;
        }
    }
    if size < ONE_DECIMAL_UNDER {
        format!("{size:.1} {unit}")
    } else {
        format!("{size:.0} {unit}")
    }
}

/// Returns a moment as the list writes it, in UTC like a photo's name: `2026-11-18 14:32`.
pub(super) fn date_text(secs: u64) -> String {
    let (year, month, day) = civil_from_days(secs / SECS_PER_DAY);
    let secs_today = secs % SECS_PER_DAY;
    let hour = secs_today / SECS_PER_HOUR;
    let minute = secs_today % SECS_PER_HOUR / SECS_PER_MINUTE;
    format!("{year:04}-{month:02}-{day:02} {hour:02}:{minute:02}")
}

/// Returns the parts of the path from a place to `folder`, a path inside it: each part's
/// name, and the folder it leads back to. The first is the place itself, at the empty path.
pub(super) fn path_parts(place_name: &str, folder: &Path) -> Vec<(String, PathBuf)> {
    let mut parts = vec![(place_name.to_string(), PathBuf::new())];
    let mut walked = PathBuf::new();
    for part in folder.iter() {
        walked.push(part);
        parts.push((part.to_string_lossy().into_owned(), walked.clone()));
    }
    parts
}

// ==== Tests ==================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::disk::fresh_test_dir;

    fn entry(name: &str, is_folder: bool) -> Entry {
        Entry {
            name: name.to_string(),
            is_folder,
            changed_secs: 0,
            size_bytes: 0,
        }
    }

    // ---- sorting and kinds --------------------------------------------------

    #[test]
    fn folders_come_first_and_each_by_name_whatever_its_case() {
        let mut entries = vec![
            entry("b.png", false),
            entry("trailer", true),
            entry("A.png", false),
            entry("Bugs", true),
        ];
        sort_entries(&mut entries);
        let names: Vec<&str> = entries.iter().map(|entry| entry.name.as_str()).collect();
        assert_eq!(names, ["Bugs", "trailer", "A.png", "b.png"]);
    }

    #[test]
    fn a_file_is_known_by_its_extension_whatever_its_case() {
        assert_eq!(kind_of(&entry("shot.PNG", false)), Kind::Image);
        assert_eq!(kind_of(&entry("tree.glb", false)), Kind::Model);
        assert_eq!(kind_of(&entry("click.ogg", false)), Kind::Sound);
        assert_eq!(kind_of(&entry("notes.txt", false)), Kind::Other);
        assert_eq!(kind_of(&entry("trailer.png", true)), Kind::Folder);
    }

    // ---- sizes and dates ----------------------------------------------------

    #[test]
    fn a_size_is_written_in_the_unit_that_keeps_it_short() {
        assert_eq!(size_text(12), "12 B");
        assert_eq!(size_text(980 * 1024), "980 KB");
        assert_eq!(size_text(2_516_582), "2.4 MB");
    }

    #[test]
    fn a_date_is_written_to_the_minute_in_utc() {
        // 2024-02-29 12:34:56, a leap day: 2024-01-01 is 1704067200, 59 days before
        let secs = 1_704_067_200 + 59 * SECS_PER_DAY + 12 * SECS_PER_HOUR + 34 * 60 + 56;
        assert_eq!(date_text(secs), "2024-02-29 12:34");
    }

    // ---- the path -----------------------------------------------------------

    #[test]
    fn the_path_starts_at_its_place_and_leads_back_to_each_folder() {
        let parts = path_parts("Gallery", Path::new("trailer/intro"));
        assert_eq!(
            parts,
            [
                ("Gallery".to_string(), PathBuf::new()),
                ("trailer".to_string(), PathBuf::from("trailer")),
                ("intro".to_string(), PathBuf::from("trailer/intro")),
            ]
        );
    }

    // ---- the disk -----------------------------------------------------------

    #[test]
    fn a_folder_reads_back_with_its_folders_first() {
        let dir = fresh_test_dir("bevy-starter-explorer-read-folder");
        std::fs::write(dir.join("shot.png"), [0_u8; 3]).unwrap();
        std::fs::create_dir(dir.join("trailer")).unwrap();
        let entries = read_folder(&dir);
        let names: Vec<&str> = entries.iter().map(|entry| entry.name.as_str()).collect();
        assert_eq!(names, ["trailer", "shot.png"]);
        assert_eq!(entries[1].size_bytes, 3);
    }

    #[test]
    fn a_folder_not_made_yet_holds_nothing() {
        let dir = fresh_test_dir("bevy-starter-explorer-missing-folder");
        assert!(read_folder(&dir.join("gallery")).is_empty());
    }

    #[test]
    fn recents_lists_every_file_under_its_folders_by_its_path_inside() {
        let dir = fresh_test_dir("bevy-starter-explorer-recents");
        std::fs::create_dir(dir.join("trailer")).unwrap();
        std::fs::write(dir.join("trailer").join("shot.png"), [0_u8]).unwrap();
        std::fs::write(dir.join("first.png"), [0_u8]).unwrap();
        let mut names: Vec<String> = read_recents(&dir)
            .into_iter()
            .map(|entry| entry.name)
            .collect();
        names.sort();
        assert_eq!(names, ["first.png", "trailer/shot.png"]);
    }

    // ---- recents and opening ------------------------------------------------

    #[test]
    fn recents_puts_the_newest_first() {
        let mut entries = vec![
            Entry {
                changed_secs: 10,
                ..entry("old.png", false)
            },
            Entry {
                changed_secs: 20,
                ..entry("new.png", false)
            },
        ];
        sort_newest_first(&mut entries);
        assert_eq!(entries[0].name, "new.png");
    }

    #[test]
    fn each_os_opens_a_file_with_its_own_program() {
        assert_eq!(opener_for("windows"), "explorer");
        assert_eq!(opener_for("macos"), "open");
        assert_eq!(opener_for("linux"), "xdg-open");
    }
}
