use bevy::asset::io::file::FileAssetReader;
use bevy::prelude::*;
use bevy::ui_widgets::Activate;
use std::path::Path;

use super::Explorer;
use super::actions::{Change, apply};
use super::dialog::{close_explorer_dialogs, spawn_answers, spawn_question};
use crate::ui::{SMALL_FONT_SIZE_PX, TEXT, TEXT_MUTED, font};

// ==== Config =================================================================

// ---- where the search reads: the repository Bevy loads the assets from ------
const SOURCE_DIR_NAME: &str = "src";
const SOURCE_EXTENSION: &str = "rs";
const ASSETS_DIR_NAME: &str = "assets";
// the text files of `assets/` that name other assets: shaders, data, translations, models
// written as text
const ASSET_TEXT_EXTENSIONS: [&str; 4] = ["wgsl", "ron", "ftl", "gltf"];

// ---- the question: each use on a line of its own ----------------------------
const QUESTION_WIDTH_PX: f32 = 720.0;
// the uses listed, the rest counted: a folder named all over the code would push the
// answers off the screen
const USES_SHOWN: usize = 8;
// a line of code is cut after this many characters, so one long line cannot widen the card
const LINE_SHOWN_CHARS: usize = 80;
// between a use's place and its line
const USE_GAP_PX: f32 = 12.0;

// ==== Plugin =================================================================

/// Where an asset is used: before Assets has something renamed, moved or deleted, the
/// explorer reads the repository's code for its path, and when the game names it, asks first.
pub(super) fn plugin(app: &mut App) {
    // no run_if, as the explorer's buttons work over any screen
    app.add_observer(answer_usage_question);
}

// ==== Usage ==================================================================

/// One line of the repository that names an asset: the file, as a path from the
/// repository, the line's number from 1, and the line itself.
#[derive(Debug, PartialEq)]
pub(super) struct Use {
    file: String,
    line_number: usize,
    line: String,
}

/// Returns every line of the repository's code and asset text that uses the asset at
/// `inside` of Assets, or none for a build run outside the repository, which has no code
/// beside its assets to read.
pub(super) fn uses_of(inside: &Path) -> Vec<Use> {
    let repository = FileAssetReader::get_base_path();
    let source = repository.join(SOURCE_DIR_NAME);
    if !source.is_dir() {
        info_once!(
            "no {} beside the assets, so the explorer cannot tell what the game uses",
            source.display()
        );
        return Vec::new();
    }
    let asset = slashed(inside);
    let is_folder = repository.join(ASSETS_DIR_NAME).join(inside).is_dir();
    let mut files = Vec::new();
    read_text_files(&repository, &source, &[SOURCE_EXTENSION], &mut files);
    read_text_files(
        &repository,
        &repository.join(ASSETS_DIR_NAME),
        &ASSET_TEXT_EXTENSIONS,
        &mut files,
    );
    find_uses(&asset, is_folder, &files)
}

/// Asks before making `change`, listing `uses`, the lines of the game that name what it
/// renames, moves or deletes; the red answer makes it anyway.
pub(super) fn ask_before_changing(commands: &mut Commands, change: Change, uses: &[Use]) {
    let question = format!(
        "The game uses this. {} it anyway?",
        capitalized(change.verb())
    );
    let card = spawn_question(commands, question, QUESTION_WIDTH_PX);
    for found in uses.iter().take(USES_SHOWN) {
        commands.spawn((use_line(found), ChildOf(card)));
    }
    if uses.len() > USES_SHOWN {
        commands.spawn((
            Text::new(format!("and {} more", uses.len() - USES_SHOWN)),
            font(SMALL_FONT_SIZE_PX),
            TextColor(TEXT_MUTED),
            ChildOf(card),
        ));
    }
    let words = change.anyway_words();
    spawn_answers(commands, card, words, UsageAnswer(change));
}

// ==== Components =============================================================

/// Marks the answer that makes the change it holds, though the game uses what it changes.
#[derive(Component)]
struct UsageAnswer(Change);

// ==== Systems ================================================================

/// Closes the question and makes the change it was asked about.
fn answer_usage_question(
    activate: On<Activate>,
    mut commands: Commands,
    answers: Query<&UsageAnswer>,
    // an `Option`: this observer sees every button pressed in the game, before the explorer
    // has ever opened and made the resource
    explorer: Option<ResMut<Explorer>>,
) {
    let (Ok(UsageAnswer(change)), Some(mut explorer)) = (answers.get(activate.entity), explorer)
    else {
        return;
    };
    close_explorer_dialogs(&mut commands);
    apply(change, &mut explorer);
}

// ==== Helpers ================================================================

/// Returns a use as the question lists it: where it is, `src/screens/menu.rs:126`, and the
/// line itself, trimmed and cut short.
fn use_line(found: &Use) -> impl Bundle {
    let line: String = found.line.trim().chars().take(LINE_SHOWN_CHARS).collect();
    (
        Node {
            column_gap: px(USE_GAP_PX),
            ..default()
        },
        children![
            (
                Text::new(format!("{}:{}", found.file, found.line_number)),
                font(SMALL_FONT_SIZE_PX),
                TextColor(TEXT_MUTED),
            ),
            (Text::new(line), font(SMALL_FONT_SIZE_PX), TextColor(TEXT)),
        ],
    )
}

/// Adds to `files` every file under `folder` whose extension is one of `extensions`, each
/// named by its path from `repository` and holding its text. A file that is not text is
/// passed over.
fn read_text_files(
    repository: &Path,
    folder: &Path,
    extensions: &[&str],
    files: &mut Vec<(String, String)>,
) {
    let Ok(reading) = std::fs::read_dir(folder) else {
        return;
    };
    for entry in reading.filter_map(Result::ok) {
        let path = entry.path();
        if path.is_dir() {
            read_text_files(repository, &path, extensions, files);
            continue;
        }
        let is_text = path
            .extension()
            .is_some_and(|extension| extensions.contains(&&*extension.to_string_lossy()));
        if let (true, Ok(text), Ok(from_repository)) = (
            is_text,
            std::fs::read_to_string(&path),
            path.strip_prefix(repository),
        ) {
            files.push((slashed(from_repository), text));
        }
    }
}

/// Returns the lines of `files`, each a name and its text, that use the asset at `asset`, a
/// path inside `assets/` written with `/`. A file is used where its path appears, and where a
/// folder holding it appears as a whole string, since a folder read whole uses every file in
/// it; a folder where its path appears followed by a `/`, or as a whole string, or where a
/// folder holding it does. Each line is listed once.
fn find_uses(asset: &str, is_folder: bool, files: &[(String, String)]) -> Vec<Use> {
    let holders: Vec<String> = Path::new(asset)
        .ancestors()
        .skip(1)
        .filter(|holder| !holder.as_os_str().is_empty())
        .map(slashed)
        .chain(is_folder.then(|| asset.to_string()))
        .map(|folder| format!("\"{folder}\""))
        .collect();
    let mut uses = Vec::new();
    for (file, text) in files {
        for (index, line) in text.lines().enumerate() {
            let used = names_path(line, asset, is_folder)
                || holders.iter().any(|holder| line.contains(holder.as_str()));
            if used {
                uses.push(Use {
                    file: file.clone(),
                    line_number: index + 1,
                    line: line.to_string(),
                });
            }
        }
    }
    uses
}

/// Tells whether `line` names the path `asset` as a path of its own, not as the start or the
/// end of a longer name: `menu/music.ogg`, but not `menu/music.ogg.bak` nor
/// `submenu/music.ogg`. What it sits after may be a `/`, as in `../assets/menu/music.ogg`; a
/// folder's path is followed by a `/`, a file's by nothing that could go on its name.
fn names_path(line: &str, asset: &str, is_folder: bool) -> bool {
    let goes_on_a_name = |character: char| character.is_alphanumeric() || "_-.".contains(character);
    line.match_indices(asset).any(|(start, _)| {
        let before = line[..start].chars().next_back();
        let after = line[start + asset.len()..].chars().next();
        let starts_alone = before.is_none_or(|character| !goes_on_a_name(character));
        let ends_alone = if is_folder {
            after == Some('/')
        } else {
            after.is_none_or(|character| !goes_on_a_name(character) && character != '/')
        };
        starts_alone && ends_alone
    })
}

/// Returns `path` written with `/` between its parts, as the code names assets on every OS.
fn slashed(path: &Path) -> String {
    path.components()
        .map(|part| part.as_os_str().to_string_lossy())
        .collect::<Vec<_>>()
        .join("/")
}

/// Returns `word` with its first letter upper case.
fn capitalized(word: &str) -> String {
    let mut letters = word.chars();
    letters
        .next()
        .map(|first| first.to_uppercase().chain(letters).collect())
        .unwrap_or_default()
}

// ==== Tests ==================================================================

#[cfg(test)]
mod tests {
    use super::*;

    /// Returns one file of code, `src/screens/menu.rs`, holding `text`.
    fn menu_rs(text: &str) -> Vec<(String, String)> {
        vec![("src/screens/menu.rs".to_string(), text.to_string())]
    }

    #[test]
    fn a_file_is_found_by_its_path_in_a_load_and_in_an_include() {
        let files = menu_rs(
            "let music = assets.load(\"menu/music.ogg\");\n\
             const FONT: &[u8] = include_bytes!(\"../assets/menu/music.ogg\");",
        );
        let uses = find_uses("menu/music.ogg", false, &files);
        let lines: Vec<usize> = uses.iter().map(|found| found.line_number).collect();
        assert_eq!(lines, [1, 2]);
        assert_eq!(uses[0].file, "src/screens/menu.rs");
    }

    #[test]
    fn a_file_in_a_folder_read_whole_is_found_by_that_folder() {
        let files = menu_rs("load_folder(\"intro/frames\")");
        assert_eq!(find_uses("intro/frames/001.png", false, &files).len(), 1);
    }

    #[test]
    fn a_folder_is_found_by_its_path_and_a_slash() {
        let files = menu_rs("assets.load(\"menu/music.ogg\")\nmenu_screen()");
        let uses = find_uses("menu", true, &files);
        assert_eq!(uses.len(), 1);
        assert_eq!(uses[0].line_number, 1);
    }

    #[test]
    fn a_path_that_only_starts_or_ends_the_same_is_not_a_use_and_a_line_counts_once() {
        let files = menu_rs(
            "\"menu/music.ogg.bak\"\n\"submenu/music.ogg\"\n\"menu/music.ogg\", \"menu/music.ogg\"",
        );
        let uses = find_uses("menu/music.ogg", false, &files);
        assert_eq!(uses.len(), 1);
        assert_eq!(uses[0].line_number, 3);
    }

    #[test]
    fn a_file_no_line_names_is_used_nowhere() {
        let files = menu_rs("assets.load(\"menu/music.ogg\")");
        assert!(find_uses("props/tree.glb", false, &files).is_empty());
    }
}
