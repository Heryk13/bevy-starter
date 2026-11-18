use bevy::input_focus::tab_navigation::TabIndex;
use bevy::input_focus::{AutoFocus, InputFocus};
use bevy::prelude::*;
use bevy::text::{EditableText, TextCursorStyle};
use bevy::ui_widgets::TextInput;

use super::Explorer;
use super::actions::{Change, request_change};
use crate::dev_tools::DEV_TOOLS_Z_INDEX;
use crate::ui::{
    ACCENT, BORDER_PX, ClosesOnEsc, DANGER, Dismiss, FIELD_FILL, SMALL_FONT_SIZE_PX, TEXT, font,
};

// ==== Config =================================================================

// ---- names: what Windows refuses in a file's name, and so every OS here -----
const REFUSED_CHARACTERS: [char; 9] = ['<', '>', ':', '"', '/', '\\', '|', '?', '*'];

// ---- the field: the name in the list, made a text field ---------------------
const RENAME_FIELD_PADDING_X_PX: f32 = 6.0;
const RENAME_FIELD_PADDING_Y_PX: f32 = 2.0;
const RENAME_FIELD_RADIUS_PX: f32 = 6.0;
// one above the explorer, under its questions: so Esc gives up the rename before it closes
// the explorer, and after it closes a question
const RENAME_FIELD_ABOVE: i32 = 1;

// ---- derived ----------------------------------------------------------------
const RENAME_FIELD_Z_INDEX: GlobalZIndex = GlobalZIndex(DEV_TOOLS_Z_INDEX.0 + RENAME_FIELD_ABOVE);

// ==== Plugin =================================================================

/// Renaming in place: the entry's name in the list becomes a text field, which Enter or a
/// click elsewhere keeps, Esc gives up, and which turns red on a name its folder refuses.
pub(super) fn plugin(app: &mut App) {
    app.add_systems(
        Update,
        finish_rename.run_if(any_with_component::<RenameField>),
    );
    // no run_if, as the explorer works over any screen; it asks for no resource that may
    // not exist yet, since every Esc in the game comes here
    app.add_observer(give_up_rename);
}

// ==== Widgets ================================================================

/// Returns the text field an entry's name becomes while it is renamed, holding `name`, the
/// keyboard handed to it at once. `taken` is every name in its folder, its own included.
pub(super) fn rename_field(name: &str, taken: Vec<String>) -> impl Bundle {
    (
        EditableText::new(name),
        // `EditableText` only holds the text, this is what types into it
        TextInput,
        // a click only hands the keyboard to an entity with a `TabIndex`; `AutoFocus` hands
        // it now, as the field appears
        TabIndex::default(),
        AutoFocus,
        TextCursorStyle {
            color: ACCENT,
            ..default()
        },
        font(SMALL_FONT_SIZE_PX),
        TextColor(TEXT),
        Node {
            flex_grow: 1.0,
            padding: UiRect::axes(px(RENAME_FIELD_PADDING_X_PX), px(RENAME_FIELD_PADDING_Y_PX)),
            border: UiRect::all(px(BORDER_PX)),
            border_radius: BorderRadius::all(px(RENAME_FIELD_RADIUS_PX)),
            ..default()
        },
        BackgroundColor(FIELD_FILL),
        BorderColor::all(ACCENT),
        // a modal as far as Esc goes, which closes the topmost by its layer
        RENAME_FIELD_Z_INDEX,
        ClosesOnEsc,
        RenameField {
            old: name.to_string(),
            taken,
            had_keyboard: false,
        },
    )
}

// ==== Components =============================================================

/// The text field an entry's name is renamed in: the name it had, the names its folder
/// holds, and whether the field has had the keyboard yet, which it loses on a click
/// elsewhere.
#[derive(Component)]
struct RenameField {
    old: String,
    taken: Vec<String>,
    had_keyboard: bool,
}

// ==== Systems ================================================================

/// Keeps the name typed in the rename field when Enter is pressed or the field loses the
/// keyboard to a click elsewhere, painting the field red while the name would be refused.
/// Enter does nothing on a refused name; a click elsewhere then gives the rename up.
fn finish_rename(
    mut commands: Commands,
    keys: Res<ButtonInput<KeyCode>>,
    focus: Res<InputFocus>,
    mut fields: Query<(Entity, &EditableText, &mut RenameField, &mut BorderColor)>,
    explorer: Option<ResMut<Explorer>>,
) {
    let Some(mut explorer) = explorer else {
        return;
    };
    for (entity, text, mut field, mut border) in &mut fields {
        let typed = text.value().to_string();
        let name = typed.trim();
        let allowed = is_name_allowed(name, &field.old, &field.taken);
        let wanted = BorderColor::all(if allowed { ACCENT } else { DANGER });
        if *border != wanted {
            *border = wanted;
        }
        let has_keyboard = focus.get() == Some(entity);
        if has_keyboard && !field.had_keyboard {
            field.had_keyboard = true;
        }
        let clicked_away = field.had_keyboard && !has_keyboard;
        let entered = keys.any_just_pressed([KeyCode::Enter, KeyCode::NumpadEnter]);
        if !clicked_away && !(entered && allowed) {
            continue;
        }
        explorer.renaming = None;
        if allowed && name != field.old {
            let change = Change::Rename {
                place: explorer.disk_place(),
                inside: explorer.inside(&field.old),
                name: name.to_string(),
            };
            request_change(&mut commands, &mut explorer, change);
        }
    }
}

/// Gives the rename up when Esc dismisses its field, leaving the name as it was.
fn give_up_rename(
    dismiss: On<Dismiss>,
    fields: Query<(), With<RenameField>>,
    explorer: Option<ResMut<Explorer>>,
) {
    if let (true, Some(mut explorer)) = (fields.contains(dismiss.entity), explorer) {
        explorer.renaming = None;
    }
}

// ==== Helpers ================================================================

/// Tells whether `name`, with its outer spaces already trimmed, can be what was called `old`:
/// not empty, not `.` or `..`, with no character Windows refuses, and not the name of another
/// entry of its folder, `taken`, whatever the case of its letters, as Windows compares them.
fn is_name_allowed(name: &str, old: &str, taken: &[String]) -> bool {
    let is_other_entry = |other: &String| {
        other.to_lowercase() == name.to_lowercase() && other.to_lowercase() != old.to_lowercase()
    };
    !name.is_empty()
        && name != "."
        && name != ".."
        && !name
            .chars()
            .any(|character| character.is_control() || REFUSED_CHARACTERS.contains(&character))
        && !taken.iter().any(is_other_entry)
}

// ==== Tests ==================================================================

#[cfg(test)]
mod tests {
    use super::*;

    fn names(names: &[&str]) -> Vec<String> {
        names.iter().map(ToString::to_string).collect()
    }

    #[test]
    fn a_name_is_refused_when_empty_a_dot_or_with_a_character_windows_refuses() {
        for refused in ["", ".", "..", "a/b", "a\\b", "a:b", "a?b", "a*b", "a\nb"] {
            assert!(!is_name_allowed(refused, "old.png", &[]), "{refused:?}");
        }
    }

    #[test]
    fn a_name_another_entry_has_is_refused_whatever_its_case() {
        let taken = names(&["old.png", "Tree.glb"]);
        assert!(!is_name_allowed("tree.glb", "old.png", &taken));
    }

    #[test]
    fn a_free_name_and_the_entrys_own_in_another_case_are_accepted() {
        let taken = names(&["old.png", "tree.glb"]);
        assert!(is_name_allowed("new.png", "old.png", &taken));
        assert!(is_name_allowed("OLD.png", "old.png", &taken));
    }
}
