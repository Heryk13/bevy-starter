use bevy::input_focus::InputFocus;
use bevy::prelude::*;
use bevy::text::{EditableText, TextCursorStyle};
use bevy::ui_widgets::{Activate, ScrollArea};
use fluent_bundle::FluentArgs;
use std::path::{Path, PathBuf};

use super::{OVERLAY_GAP_PX, Overlay, close_footer, overlay_card, overlay_root};
use crate::i18n::{Language, Localized, Translations};
use crate::save::{
    SaveData, SaveEntry, Session, is_autosave, list_saves, loaded_session, new_save_path,
    read_save, saves_dir, unix_now, write_save,
};
use crate::screens::StartGame;
use crate::screens::gameplay::pause::SaveStatus;
use crate::screens::loading::LoadingDestination;
use crate::ui::{
    ACCENT, BODY_FONT_SIZE_PX, BORDER, BORDER_PX, ButtonKind, DANGER, FIELD_FILL,
    SMALL_FONT_SIZE_PX, TEXT, TEXT_MUTED, button, dialog, font, heading, row_button,
};

// ==== Config =================================================================

// ---- translation keys -------------------------------------------------------
const OVERWRITE_QUESTION: &str = "dialog-overwrite-save";
const DELETE_QUESTION: &str = "dialog-delete-save";
const SAVED_TEXT: &str = "status-saved";
const SAVE_FAILED_TEXT: &str = "status-save-failed";
const NO_SAVES_TEXT: &str = "saves-none";
const INCOMPATIBLE_SAVE_TEXT: &str = "save-incompatible";
const UNREADABLE_SAVE_TEXT: &str = "save-unreadable";

// ---- lists ------------------------------------------------------------------
const SAVE_LIST_MAX_HEIGHT_PX: f32 = 300.0;
// a row's name and details sit close, so they read as one save
const SAVE_ROW_LINE_GAP_PX: f32 = 2.0;

// ---- name field -------------------------------------------------------------
// `EditableText` draws no box, and no cursor without a `TextCursorStyle`, so these are what
// show where to click and where the next letter goes; the default cursor is a dark grey that
// vanishes on the card
const SAVE_NAME_PADDING_PX: f32 = 10.0;
const SAVE_NAME_RADIUS_PX: f32 = 8.0;
// a longer name pushes its row past the edge of the list
const SAVE_NAME_MAX_CHARS: usize = 20;

// ==== Plugin =================================================================

pub(super) fn plugin(app: &mut App) {
    app.add_systems(OnEnter(Overlay::SaveGame), spawn_save_game_overlay);
    app.add_systems(
        Update,
        paint_save_name_field.run_if(in_state(Overlay::SaveGame)),
    );
    app.add_observer(save_new_game.run_if(in_state(Overlay::SaveGame)));
    app.add_observer(ask_to_overwrite_save.run_if(in_state(Overlay::SaveGame)));
    app.add_observer(overwrite_save.run_if(in_state(Overlay::SaveGame)));
    app.add_systems(OnEnter(Overlay::LoadGame), spawn_load_game_overlay);
    app.add_observer(load_save.run_if(in_state(Overlay::LoadGame)));
    app.add_observer(ask_to_delete_save.run_if(in_state(Overlay::LoadGame)));
    app.add_observer(delete_save.run_if(in_state(Overlay::LoadGame)));
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

/// Marks a row's Delete button, with the save file it asks to delete.
#[derive(Component)]
struct DeleteSave(PathBuf);

/// Marks the Yes of `DELETE_QUESTION`, with the save file to delete.
#[derive(Component)]
struct ConfirmDelete(PathBuf);

// ==== Systems ================================================================

// ---- save game --------------------------------------------------------------

/// Spawns the save game panel over the current screen, blocking clicks to it: the name field
/// beside New Save, then the saves it can write over.
fn spawn_save_game_overlay(
    mut commands: Commands,
    translations: Res<Translations>,
    language: Res<Language>,
) {
    let saves = saves_dir().map(|dir| list_saves(&dir)).unwrap_or_default();
    let root = commands
        .spawn((overlay_root(), DespawnOnExit(Overlay::SaveGame)))
        .id();
    let card = commands
        .spawn((
            overlay_card(),
            ChildOf(root),
            children![
                heading("save-game-title"),
                (
                    Node {
                        column_gap: px(OVERLAY_GAP_PX),
                        align_items: AlignItems::Center,
                        ..default()
                    },
                    children![
                        (
                            EditableText {
                                max_characters: Some(SAVE_NAME_MAX_CHARS),
                                ..default()
                            },
                            SaveName,
                            TextCursorStyle {
                                color: ACCENT,
                                ..default()
                            },
                            font(BODY_FONT_SIZE_PX),
                            TextColor(TEXT),
                            Node {
                                flex_grow: 1.0,
                                padding: UiRect::all(px(SAVE_NAME_PADDING_PX)),
                                border: UiRect::all(px(BORDER_PX)),
                                border_radius: BorderRadius::all(px(SAVE_NAME_RADIUS_PX)),
                                ..default()
                            },
                            BackgroundColor(FIELD_FILL),
                            BorderColor::all(BORDER),
                        ),
                        (button("save-new", ButtonKind::Primary), NewSave),
                    ],
                ),
            ],
        ))
        .id();
    let now_secs = unix_now().as_secs();
    let list = commands.spawn((save_list(), ChildOf(card))).id();
    for entry in &saves {
        if is_autosave(entry.path()) {
            continue;
        }
        match entry {
            SaveEntry::Ready(path, save) => {
                commands.spawn((
                    row_button(save_row(
                        save_name(save, is_autosave(path), &translations, *language),
                        save_details(save, now_secs, &translations, *language),
                    )),
                    OverwriteSave(path.clone(), save.name.clone()),
                    ChildOf(list),
                ));
            }
            SaveEntry::Incompatible(_) => {
                commands.spawn((muted_label(INCOMPATIBLE_SAVE_TEXT), ChildOf(list)));
            }
            SaveEntry::Unreadable(_) => {
                commands.spawn((muted_label(UNREADABLE_SAVE_TEXT), ChildOf(list)));
            }
        }
    }
    commands.spawn((close_footer(), ChildOf(card)));
}

/// Outlines the name field in the accent while it has the keyboard, so the player sees
/// where their typing goes.
fn paint_save_name_field(
    focus: Res<InputFocus>,
    mut fields: Query<(Entity, &mut BorderColor), With<SaveName>>,
) {
    for (entity, mut border) in &mut fields {
        let color = if focus.get() == Some(entity) {
            ACCENT
        } else {
            BORDER
        };
        *border = BorderColor::all(color);
    }
}

/// Writes the session to a new save file when the "New Save" button is pressed, named by the
/// text field without its outer spaces, then closes the overlay either way, so the pause
/// menu's status line says how it went.
fn save_new_game(
    activate: On<Activate>,
    new_save_buttons: Query<(), With<NewSave>>,
    mut next_overlay: ResMut<NextState<Overlay>>,
    mut session: ResMut<Session>,
    mut statuses: Query<(&mut Text, &mut TextColor, &mut SaveStatus)>,
    translations: Res<Translations>,
    language: Res<Language>,
    names: Query<&EditableText, With<SaveName>>,
) {
    if new_save_buttons.contains(activate.entity) {
        let name = names
            .single()
            .map(|field| field.value().to_string().trim().to_string())
            .unwrap_or_default();
        let status = match saves_dir() {
            Some(dir) => {
                let path = new_save_path(&dir);
                save_status(&path, &mut session, &name)
            }
            None => SAVE_FAILED_TEXT,
        };
        for (mut text, mut color, mut line) in &mut statuses {
            text.0 = translations.text(*language, status);
            color.0 = status_color(status);
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
                ButtonKind::Danger,
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
    mut statuses: Query<(&mut Text, &mut TextColor, &mut SaveStatus)>,
    translations: Res<Translations>,
    language: Res<Language>,
) {
    if let Ok(ConfirmOverwrite(path, name)) = confirms.get(activate.entity) {
        let status = save_status(path, &mut session, name);
        for (mut text, mut color, mut line) in &mut statuses {
            text.0 = translations.text(*language, status);
            color.0 = status_color(status);
            line.shown_secs = 0.0;
        }
        next_overlay.set(Overlay::None);
    }
}

// ---- load game --------------------------------------------------------------

/// Spawns the load game panel over the current screen, listing every save, autosaves
/// included, since loading one costs nothing that is not already in a save. Every row has
/// a Delete button, the files the game cannot load included, since this is the only way
/// they leave the list.
fn spawn_load_game_overlay(
    mut commands: Commands,
    translations: Res<Translations>,
    language: Res<Language>,
) {
    let saves = saves_dir().map(|dir| list_saves(&dir)).unwrap_or_default();
    let root = commands
        .spawn((overlay_root(), DespawnOnExit(Overlay::LoadGame)))
        .id();
    let card = commands
        .spawn((
            overlay_card(),
            ChildOf(root),
            children![heading("load-game-title")],
        ))
        .id();
    let now_secs = unix_now().as_secs();
    let list = commands.spawn((save_list(), ChildOf(card))).id();
    if saves.is_empty() {
        commands.spawn((muted_label(NO_SAVES_TEXT), ChildOf(list)));
    }
    for entry in &saves {
        let row = commands
            .spawn((
                Node {
                    column_gap: px(OVERLAY_GAP_PX),
                    align_items: AlignItems::Center,
                    ..default()
                },
                ChildOf(list),
            ))
            .id();
        match entry {
            SaveEntry::Ready(path, save) => {
                commands.spawn((
                    row_button(save_row(
                        save_name(save, is_autosave(path), &translations, *language),
                        save_details(save, now_secs, &translations, *language),
                    )),
                    LoadSave(path.clone()),
                    ChildOf(row),
                ));
            }
            // flex_grow, like a row button's: the Delete beside it stays at the right edge
            SaveEntry::Incompatible(_) => {
                commands.spawn((
                    muted_label(INCOMPATIBLE_SAVE_TEXT),
                    Node {
                        flex_grow: 1.0,
                        ..default()
                    },
                    ChildOf(row),
                ));
            }
            SaveEntry::Unreadable(_) => {
                commands.spawn((
                    muted_label(UNREADABLE_SAVE_TEXT),
                    Node {
                        flex_grow: 1.0,
                        ..default()
                    },
                    ChildOf(row),
                ));
            }
        }
        commands.spawn((
            button("save-delete", ButtonKind::Danger),
            DeleteSave(entry.path().to_path_buf()),
            ChildOf(row),
        ));
    }
    commands.spawn((close_footer(), ChildOf(card)));
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

/// Asks `DELETE_QUESTION` when a row's Delete is pressed, since a deleted save is gone for
/// good.
fn ask_to_delete_save(
    activate: On<Activate>,
    mut commands: Commands,
    delete_buttons: Query<&DeleteSave>,
) {
    if let Ok(DeleteSave(path)) = delete_buttons.get(activate.entity) {
        commands.spawn((
            dialog(
                DELETE_QUESTION,
                ButtonKind::Danger,
                ConfirmDelete(path.clone()),
            ),
            DespawnOnExit(Overlay::LoadGame),
        ));
    }
}

/// Deletes a save once `DELETE_QUESTION` is answered Yes, then enters the overlay again,
/// which reads the folder anew, so the list loses the row without code to rebuild it.
/// A failed delete is logged, and the row stays.
fn delete_save(
    activate: On<Activate>,
    confirms: Query<&ConfirmDelete>,
    mut next_overlay: ResMut<NextState<Overlay>>,
) {
    if let Ok(ConfirmDelete(path)) = confirms.get(activate.entity) {
        if let Err(err) = std::fs::remove_file(path) {
            error!("could not delete the save at {}: {err}", path.display());
        }
        // `set`, not `set_if_neq`: the same state runs `OnExit` and `OnEnter` again
        next_overlay.set(Overlay::LoadGame);
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

/// Returns a row's content: the save's name, its details in small muted text underneath.
fn save_row(name: String, details: String) -> impl Bundle {
    (
        Node {
            flex_direction: FlexDirection::Column,
            row_gap: px(SAVE_ROW_LINE_GAP_PX),
            ..default()
        },
        children![
            (Text::new(name), font(BODY_FONT_SIZE_PX), TextColor(TEXT)),
            (
                Text::new(details),
                font(SMALL_FONT_SIZE_PX),
                TextColor(TEXT_MUTED),
            ),
        ],
    )
}

/// Returns muted body text showing a translation key: what a list says in place of a save.
fn muted_label(key: &'static str) -> impl Bundle {
    (
        Localized(key),
        font(BODY_FONT_SIZE_PX),
        TextColor(TEXT_MUTED),
    )
}

/// Returns a save's name in `language`, or its stand-in for an autosave or a save the player
/// gave no name. An autosave's name comes from its file, not from inside it, so one written
/// in English still reads in the language the player chose since.
fn save_name(
    save: &SaveData,
    autosave: bool,
    translations: &Translations,
    language: Language,
) -> String {
    if autosave {
        translations.text(language, "save-autosave")
    } else if save.name.trim().is_empty() {
        translations.text(language, "save-unnamed")
    } else {
        save.name.clone()
    }
}

/// Returns a save's details in `language`: its play time and how long ago it was saved. A
/// save dated after `now_secs`, from a clock that was set back, counts as saved just now.
fn save_details(
    save: &SaveData,
    now_secs: u64,
    translations: &Translations,
    language: Language,
) -> String {
    let total_secs = save.play_secs as u64;
    let age_secs = now_secs.saturating_sub(save.saved_at_secs);
    let mut played_args = FluentArgs::new();
    played_args.set("hours", total_secs / 3600);
    played_args.set("minutes", format!("{:02}", (total_secs % 3600) / 60));
    let mut args = FluentArgs::new();
    args.set(
        "played",
        translations.text_with(language, "save-played", &played_args),
    );
    args.set("age", age_label(age_secs, translations, language));
    translations.text_with(language, "save-details", &args)
}

/// Returns an age in its largest whole unit, seconds, minutes, hours or days, in
/// `language`. Abbreviated, so no unit needs a singular and a plural.
fn age_label(age_secs: u64, translations: &Translations, language: Language) -> String {
    let (key, count) = if age_secs < 60 {
        ("age-seconds", age_secs)
    } else if age_secs < 3600 {
        ("age-minutes", age_secs / 60)
    } else if age_secs < 86400 {
        ("age-hours", age_secs / 3600)
    } else {
        ("age-days", age_secs / 86400)
    };
    let mut args = FluentArgs::new();
    args.set("count", count);
    translations.text_with(language, key, &args)
}

/// Writes the session to `path` and returns the key of the line that tells the player how
/// it went, logging the reason when it failed.
fn save_status(path: &Path, session: &mut Session, name: &str) -> &'static str {
    match write_save(path, session, name) {
        Ok(()) => SAVED_TEXT,
        Err(err) => {
            error!("could not write the save: {err}");
            SAVE_FAILED_TEXT
        }
    }
}

/// Returns the color of the status line for a `save_status` key: the accent for a save that
/// worked, red for one that did not.
fn status_color(status: &str) -> Color {
    if status == SAVED_TEXT { ACCENT } else { DANGER }
}

// ==== Tests ==================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::save::GameData;

    /// Returns a save with the given name, play time and date, the rest left empty.
    fn save(name: &str, play_secs: f32, saved_at_secs: u64) -> SaveData {
        SaveData {
            play_secs,
            version: 0,
            name: String::from(name),
            saved_at_secs,
            game: GameData::default(),
        }
    }

    // ---- save_name ----------------------------------------------------------

    #[test]
    fn save_shows_the_name_the_player_gave_it() {
        let translations = Translations::default();
        let name = save_name(
            &save("Castle", 0.0, 0),
            false,
            &translations,
            Language::English,
        );
        assert_eq!(name, "Castle");
    }

    #[test]
    fn save_without_a_name_is_called_unnamed() {
        let translations = Translations::default();
        let name = save_name(&save("", 0.0, 0), false, &translations, Language::English);
        assert_eq!(name, "Unnamed");
    }

    #[test]
    fn blank_name_is_called_unnamed() {
        let translations = Translations::default();
        let name = save_name(
            &save("     ", 0.0, 0),
            false,
            &translations,
            Language::English,
        );
        assert_eq!(name, "Unnamed");
    }

    #[test]
    fn autosave_is_named_by_its_file_in_the_chosen_language() {
        let translations = Translations::default();
        let name = save_name(
            &save("Autosave", 0.0, 0),
            true,
            &translations,
            Language::Japanese,
        );
        assert_eq!(name, "オートセーブ");
    }

    #[test]
    fn player_name_is_never_translated() {
        let translations = Translations::default();
        let name = save_name(
            &save("Castle", 0.0, 0),
            false,
            &translations,
            Language::Japanese,
        );
        assert_eq!(name, "Castle");
    }

    // ---- save_details -------------------------------------------------------

    #[test]
    fn details_show_hours_and_padded_minutes() {
        let translations = Translations::default();
        let details = save_details(&save("", 3661.0, 0), 0, &translations, Language::English);
        assert_eq!(details, "1h 01m played · 0s ago");
    }

    #[test]
    fn save_dated_in_the_future_was_saved_zero_seconds_ago() {
        let translations = Translations::default();
        let details = save_details(
            &save("", 0.0, 2 * 86400),
            86400,
            &translations,
            Language::English,
        );
        assert_eq!(details, "0h 00m played · 0s ago");
    }

    #[test]
    fn details_read_in_japanese() {
        let translations = Translations::default();
        let details = save_details(
            &save("", 3661.0, 0),
            2 * 86400,
            &translations,
            Language::Japanese,
        );
        assert_eq!(details, "プレイ時間 1:01 · 2日前");
    }

    // ---- age_label: one test per unit, each well inside its range -----------

    #[test]
    fn age_under_a_minute_is_in_seconds() {
        let translations = Translations::default();
        assert_eq!(age_label(30, &translations, Language::English), "30s ago");
    }

    #[test]
    fn age_under_an_hour_is_in_minutes() {
        let translations = Translations::default();
        assert_eq!(
            age_label(5 * 60, &translations, Language::English),
            "5m ago"
        );
    }

    #[test]
    fn age_under_a_day_is_in_hours() {
        let translations = Translations::default();
        assert_eq!(
            age_label(3 * 3600, &translations, Language::English),
            "3h ago"
        );
    }

    #[test]
    fn age_of_a_day_or_more_is_in_days() {
        let translations = Translations::default();
        assert_eq!(
            age_label(2 * 86400, &translations, Language::English),
            "2d ago"
        );
    }

    #[test]
    fn age_reads_in_japanese() {
        let translations = Translations::default();
        assert_eq!(
            age_label(5 * 60, &translations, Language::Japanese),
            "5分前"
        );
    }

    // ---- status_color -------------------------------------------------------

    #[test]
    fn saved_status_is_in_the_accent() {
        assert_eq!(status_color(SAVED_TEXT), ACCENT);
    }

    #[test]
    fn failed_status_is_red() {
        assert_eq!(status_color(SAVE_FAILED_TEXT), DANGER);
    }
}
