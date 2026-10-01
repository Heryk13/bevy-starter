use bevy::prelude::*;
use bevy::text::{EditableText, TextCursorStyle};
use bevy::ui_widgets::{Activate, ScrollArea};
use std::path::{Path, PathBuf};

use super::CloseOverlay;
use super::OVERLAY_GAP_PX;
use super::Overlay;
use super::overlay_root;
use crate::save::{
    SaveData, SaveEntry, Session, is_autosave, list_saves, loaded_session, new_save_path,
    read_save, saves_dir, unix_now, write_save,
};
use crate::screens::StartGame;
use crate::screens::gameplay::pause::SaveStatus;
use crate::screens::loading::LoadingDestination;
use crate::ui::{button, dialog};

// ==== Config =================================================================

const OVERWRITE_QUESTION: &str = "Overwrite this save?";
const SAVED_TEXT: &str = "Saved successfully.";
const SAVE_FAILED_TEXT: &str = "Failed to save.";
const NO_SAVES_TEXT: &str = "No saves yet";
const INCOMPATIBLE_SAVE_TEXT: &str = "Incompatible Save";
const UNREADABLE_SAVE_TEXT: &str = "Unreadable Save";

const SAVE_LIST_MAX_HEIGHT_PX: f32 = 300.0;

// `EditableText` draws no box, and no cursor without a `TextCursorStyle`, so these are what
// show where to click and where the next letter goes; the default cursor is a dark grey that
// vanishes on the backdrop
const SAVE_NAME_WIDTH_PX: f32 = 300.0;
const SAVE_NAME_PADDING_PX: f32 = 6.0;
const SAVE_NAME_BACKGROUND: Color = Color::srgba(1.0, 1.0, 1.0, 0.1);
const SAVE_NAME_CURSOR_COLOR: Color = Color::WHITE;
// a longer name pushes its row's label past the edge of the button in the list
const SAVE_NAME_MAX_CHARS: usize = 20;

// ==== Plugin =================================================================

pub(super) fn plugin(app: &mut App) {
    app.add_systems(OnEnter(Overlay::SaveGame), spawn_save_game_overlay);
    app.add_observer(save_new_game);
    app.add_observer(ask_to_overwrite_save);
    app.add_observer(overwrite_save);
    app.add_systems(OnEnter(Overlay::LoadGame), spawn_load_game_overlay);
    app.add_observer(load_save);
}

// ==== Components =============================================================

/// Marks the button that writes the session to a save file of its own.
#[derive(Component)]
struct NewSave;

/// Marks a row that asks to write the session over a save file, with that file and the
/// save's name, which overwriting keeps.
#[derive(Component)]
struct OverwriteSave(PathBuf, String);

/// Marks the Yes of `OVERWRITE_QUESTION`, with the save file to write over and its name.
#[derive(Component)]
struct ConfirmOverwrite(PathBuf, String);

/// Marks the text field where the player names a new save.
#[derive(Component)]
struct SaveName;

/// Marks a row that loads the save file it names.
#[derive(Component)]
struct LoadSave(PathBuf);

// ==== Systems ================================================================

// ---- save game --------------------------------------------------------------

/// Spawns the save game panel over the current screen, blocking clicks to it.
fn spawn_save_game_overlay(mut commands: Commands) {
    let saves = saves_dir().map(|dir| list_saves(&dir)).unwrap_or_default();
    let root = commands
        .spawn((
            overlay_root(),
            children![
                Text::new("Save Game"),
                (
                    EditableText {
                        max_characters: Some(SAVE_NAME_MAX_CHARS),
                        ..default()
                    },
                    SaveName,
                    TextCursorStyle {
                        color: SAVE_NAME_CURSOR_COLOR,
                        ..default()
                    },
                    Node {
                        padding: UiRect::all(px(SAVE_NAME_PADDING_PX)),
                        width: px(SAVE_NAME_WIDTH_PX),
                        ..default()
                    },
                    BackgroundColor(SAVE_NAME_BACKGROUND),
                ),
                (button("New Save"), NewSave),
            ],
            DespawnOnExit(Overlay::SaveGame),
        ))
        .id();
    let now_secs = unix_now().as_secs();
    let list = commands.spawn((save_list(), ChildOf(root))).id();
    for entry in &saves {
        if is_autosave(entry.path()) {
            continue;
        }
        match entry {
            SaveEntry::Ready(path, save) => {
                commands.spawn((
                    button(&save_label(save, now_secs)),
                    OverwriteSave(path.clone(), save.name.clone()),
                    ChildOf(list),
                ));
            }
            SaveEntry::Incompatible(_) => {
                commands.spawn((Text::new(INCOMPATIBLE_SAVE_TEXT), ChildOf(list)));
            }
            SaveEntry::Unreadable(_) => {
                commands.spawn((Text::new(UNREADABLE_SAVE_TEXT), ChildOf(list)));
            }
        }
    }
    commands.spawn((button("Close"), CloseOverlay, ChildOf(root)));
}

/// Writes the session to a new save file when the "New Save" button is pressed, named by the
/// text field without its outer spaces, then closes the overlay either way, so the pause
/// menu's status line says how it went.
fn save_new_game(
    activate: On<Activate>,
    new_save_buttons: Query<(), With<NewSave>>,
    mut next_overlay: ResMut<NextState<Overlay>>,
    mut session: ResMut<Session>,
    mut statuses: Query<(&mut Text, &mut SaveStatus)>,
    names: Query<&EditableText, With<SaveName>>,
) {
    if new_save_buttons.contains(activate.entity) {
        let name = names
            .single()
            .map(|field| field.value().to_string())
            .unwrap_or_default();
        let status = match saves_dir() {
            Some(dir) => {
                let path = new_save_path(&dir);
                save_status(&path, &mut session, &name)
            }
            None => SAVE_FAILED_TEXT,
        };
        for (mut text, mut line) in &mut statuses {
            text.0 = String::from(status);
            line.shown_secs = 0.0;
        }
        next_overlay.set(Overlay::None);
    }
}

/// Asks `OVERWRITE_QUESTION` when a save's row is pressed, so one stray click never costs
/// an older save.
fn ask_to_overwrite_save(
    activate: On<Activate>,
    mut commands: Commands,
    rows: Query<&OverwriteSave>,
) {
    if let Ok(OverwriteSave(path, name)) = rows.get(activate.entity) {
        commands.spawn((
            dialog(
                OVERWRITE_QUESTION,
                ConfirmOverwrite(path.clone(), name.clone()),
            ),
            DespawnOnExit(Overlay::SaveGame),
        ));
    }
}

/// Writes the session over a save once `OVERWRITE_QUESTION` is answered Yes, keeping the
/// save's name rather than the text field's, then closes the overlay either way, so the
/// pause menu's status line says how it went.
fn overwrite_save(
    activate: On<Activate>,
    confirms: Query<&ConfirmOverwrite>,
    mut next_overlay: ResMut<NextState<Overlay>>,
    mut session: ResMut<Session>,
    mut statuses: Query<(&mut Text, &mut SaveStatus)>,
) {
    if let Ok(ConfirmOverwrite(path, name)) = confirms.get(activate.entity) {
        let status = save_status(path, &mut session, name.trim());
        for (mut text, mut line) in &mut statuses {
            text.0 = String::from(status);
            line.shown_secs = 0.0;
        }
        next_overlay.set(Overlay::None);
    }
}

// ---- load game --------------------------------------------------------------

/// Spawns the load game panel over the current screen, listing every save, autosaves
/// included, since loading one costs nothing that is not already in a save.
fn spawn_load_game_overlay(mut commands: Commands) {
    let saves = saves_dir().map(|dir| list_saves(&dir)).unwrap_or_default();
    let root = commands
        .spawn((
            overlay_root(),
            children![Text::new("Load Game")],
            DespawnOnExit(Overlay::LoadGame),
        ))
        .id();
    let now_secs = unix_now().as_secs();
    let list = commands.spawn((save_list(), ChildOf(root))).id();
    if saves.is_empty() {
        commands.spawn((Text::new(NO_SAVES_TEXT), ChildOf(list)));
    }
    for entry in &saves {
        match entry {
            SaveEntry::Ready(path, save) => {
                commands.spawn((
                    button(&save_label(save, now_secs)),
                    LoadSave(path.clone()),
                    ChildOf(list),
                ));
            }
            SaveEntry::Incompatible(_) => {
                commands.spawn((Text::new(INCOMPATIBLE_SAVE_TEXT), ChildOf(list)));
            }
            SaveEntry::Unreadable(_) => {
                commands.spawn((Text::new(UNREADABLE_SAVE_TEXT), ChildOf(list)));
            }
        }
    }
    commands.spawn((button("Close"), CloseOverlay, ChildOf(root)));
}

/// Starts the game in a save when its row is pressed, closing the overlay so it does not
/// outlive the screen it was opened over. A save that can no longer be read stays put.
fn load_save(
    activate: On<Activate>,
    mut commands: Commands,
    rows: Query<&LoadSave>,
    mut next_overlay: ResMut<NextState<Overlay>>,
) {
    if let Ok(LoadSave(path)) = rows.get(activate.entity) {
        let Some(data) = read_save(path) else {
            error!("could not load the save at {}", path.display());
            return;
        };
        commands.trigger(StartGame {
            session: loaded_session(&data),
            destination: LoadingDestination::Gameplay,
        });
        next_overlay.set(Overlay::None);
    }
}

// ==== Helpers ================================================================

/// Returns the scrolling column the save rows go in, the same in both overlays.
fn save_list() -> impl Bundle {
    (
        Node {
            flex_direction: FlexDirection::Column,
            row_gap: px(OVERLAY_GAP_PX),
            max_height: px(SAVE_LIST_MAX_HEIGHT_PX),
            overflow: Overflow::scroll_y(),
            ..default()
        },
        ScrollArea,
    )
}

/// Returns a save's label: its name, or a stand-in when the player gave none, then its play
/// time and how long ago it was saved. A save dated after `now_secs`, from a clock that
/// was set back, counts as saved just now.
fn save_label(save: &SaveData, now_secs: u64) -> String {
    let total_secs = save.play_secs as u64;
    let hours = total_secs / 3600;
    let minutes = (total_secs % 3600) / 60;
    let age_secs = now_secs.saturating_sub(save.saved_at_secs);
    let name = if save.name.is_empty() {
        "Unnamed"
    } else {
        &save.name
    };
    format!(
        "{name} · {hours}h {minutes:02}m played · {} ago",
        age_label(age_secs)
    )
}

/// Returns an age in its largest whole unit: seconds, minutes, hours or days. Abbreviated,
/// so no unit needs a singular and a plural.
fn age_label(age_secs: u64) -> String {
    if age_secs < 60 {
        format!("{}s", age_secs)
    } else if age_secs < 3600 {
        format!("{}m", age_secs / 60)
    } else if age_secs < 86400 {
        format!("{}h", age_secs / 3600)
    } else {
        format!("{}d", age_secs / 86400)
    }
}

/// Writes the session to `path` and returns the line that tells the player how it went,
/// logging the reason when it failed.
fn save_status(path: &Path, session: &mut Session, name: &str) -> &'static str {
    match write_save(path, session, name) {
        Ok(()) => SAVED_TEXT,
        Err(err) => {
            error!("could not write the save: {err}");
            SAVE_FAILED_TEXT
        }
    }
}

// ==== Tests ==================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn label_shows_hours_and_padded_minutes() {
        let save = SaveData {
            play_secs: 3661.0,
            version: 0,
            name: String::new(),
            saved_at_secs: 0,
        };
        let label = save_label(&save, 0);
        assert_eq!(label, "Unnamed · 1h 01m played · 0s ago");
    }

    #[test]
    fn label_starts_with_the_save_name() {
        let save = SaveData {
            play_secs: 3661.0,
            version: 0,
            name: String::from("Castle"),
            saved_at_secs: 0,
        };
        let label = save_label(&save, 0);
        assert_eq!(label, "Castle · 1h 01m played · 0s ago");
    }

    #[test]
    fn save_dated_in_the_future_was_saved_zero_seconds_ago() {
        let save = SaveData {
            play_secs: 0.0,
            version: 0,
            name: String::new(),
            saved_at_secs: 2 * 86400,
        };
        let label = save_label(&save, 86400);
        assert_eq!(label, "Unnamed · 0h 00m played · 0s ago");
    }

    // ---- age_label: one test per unit, each well inside its range -----------

    #[test]
    fn age_under_a_minute_is_in_seconds() {
        assert_eq!(age_label(30), "30s");
    }

    #[test]
    fn age_under_an_hour_is_in_minutes() {
        assert_eq!(age_label(5 * 60), "5m");
    }

    #[test]
    fn age_under_a_day_is_in_hours() {
        assert_eq!(age_label(3 * 3600), "3h");
    }

    #[test]
    fn age_of_a_day_or_more_is_in_days() {
        assert_eq!(age_label(2 * 86400), "2d");
    }
}
