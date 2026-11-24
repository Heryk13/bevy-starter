use bevy::input_focus::tab_navigation::TabIndex;
use bevy::input_focus::{AutoFocus, InputFocus};
use bevy::picking::hover::Hovered;
use bevy::prelude::*;
use bevy::text::{EditableText, TextCursorStyle};
use bevy::ui_widgets::{Activate, TextInput};
use bevy::window::PrimaryWindow;

use super::super::file_explorer::{is_name_allowed, menu_place};
use super::super::{DEV_TOOLS_Z_INDEX, DevUi, dev_window};
use super::edits::PropsSheet;
use super::files::PropFileEdit;
use super::library::{
    OpenFolder, PROPS_DIR_NAME, PropFolders, PropLibrary, PropPreferences, folder_of,
    inside_folder, is_prop_file, parent_folder,
};
use super::window::{FolderCell, PropCell, PropsGrid};
use super::{PropEditor, PropKind};
use crate::ui::{
    ACCENT, BORDER_PX, ButtonKind, ClosesOnEsc, DANGER, Dismiss, FIELD_FILL, SMALL_FONT_SIZE_PX,
    TEXT, floating_root, font, words_button,
};

// ==== Config =================================================================

// ---- words: English, as every dev tool's, never read in a release build -----
const FAVOURITE_WORDS: &str = "Favourite";
const UNFAVOURITE_WORDS: &str = "Unfavourite";
const RENAME_WORDS: &str = "Rename";
const DELETE_WORDS: &str = "Delete";
const NEW_FOLDER_WORDS: &str = "New Folder";

// ---- menu: over the props window, as the explorer's menu is over the explorer
const MENU_Z_INDEX: GlobalZIndex = GlobalZIndex(DEV_TOOLS_Z_INDEX.0 + 10);

// ---- rename field -----------------------------------------------------------
const RENAME_FIELD_WIDTH_PX: f32 = 200.0;
const RENAME_FIELD_PADDING_X_PX: f32 = 8.0;
const RENAME_FIELD_PADDING_Y_PX: f32 = 4.0;
const RENAME_FIELD_RADIUS_PX: f32 = 6.0;

// ==== Plugin =================================================================

/// The props window's right-click menus, as an explorer's: on a prop's file Favourite (or
/// Unfavourite), Rename and Delete; on a folder Rename and Delete; on the grid's empty space
/// New Folder. Rename turns the menu into a field where it was, which Enter keeps and Esc or a
/// click elsewhere gives up, red while the name would be refused. Every change to the folder
/// goes through a `PropFileEdit` (`files.rs`); a favourite is the developer's own.
pub(super) fn plugin(app: &mut App) {
    app.add_systems(
        Update,
        (close_cell_menu_on_press_outside, finish_prop_rename).run_if(in_state(PropEditor::On)),
    );
    app.add_observer(open_cell_menu.run_if(in_state(PropEditor::On)));
    app.add_observer(apply_cell_choice.run_if(in_state(PropEditor::On)));
    app.add_observer(dismiss_cell_menu.run_if(in_state(PropEditor::On)));
}

// ==== Components =============================================================

/// Marks the root of a right-click menu, or of the rename field that took its place, with
/// where the pointer opened it.
#[derive(Component)]
struct CellMenu {
    at: Vec2,
}

/// Marks the menu's sheet, whose `Hovered` tells a press on it from one outside.
#[derive(Component)]
struct CellMenuSheet;

/// What a menu item does, with what to: paths are inside the props folder, a prop's file by
/// its asset path for its favourite.
#[derive(Component, Clone)]
enum CellChoice {
    Favourite(String),
    Rename(String),
    Delete(String),
    NewFolder,
}

/// The rename field: the path of what it renames, the names its folder holds, and whether it
/// has had the keyboard yet, which it loses on a click elsewhere.
#[derive(Component)]
struct PropRenameField {
    path: String,
    taken: Vec<String>,
    had_keyboard: bool,
}

// ==== Systems ================================================================

/// Opens a right-click menu where the pointer is, in place of one already open: a prop's file's
/// on its cell, a folder's on its cell, New Folder on the grid's empty space.
fn open_cell_menu(
    click: On<PointerClick>,
    mut commands: Commands,
    props: Query<&PropCell>,
    folders: Query<&FolderCell>,
    grids: Query<(), With<PropsGrid>>,
    preferences: Res<PropPreferences>,
    menus: Query<Entity, With<CellMenu>>,
    window: Single<&Window, With<PrimaryWindow>>,
) {
    if click.button != PointerButton::Secondary {
        return;
    }
    let items = if let Ok(PropCell(file)) = props.get(click.entity) {
        let favourite_words = if preferences.favourites.contains(file) {
            UNFAVOURITE_WORDS
        } else {
            FAVOURITE_WORDS
        };
        let path = inside_props(file).to_string();
        vec![
            (favourite_words, CellChoice::Favourite(file.clone())),
            (RENAME_WORDS, CellChoice::Rename(path.clone())),
            (DELETE_WORDS, CellChoice::Delete(path)),
        ]
    } else if let Ok(FolderCell(folder)) = folders.get(click.entity) {
        vec![
            (RENAME_WORDS, CellChoice::Rename(folder.clone())),
            (DELETE_WORDS, CellChoice::Delete(folder.clone())),
        ]
    } else if grids.contains(click.entity) {
        vec![(NEW_FOLDER_WORDS, CellChoice::NewFolder)]
    } else {
        return;
    };
    close_menus(&mut commands, &menus);
    let at = click.pointer.position;
    commands.spawn(menu_root(at)).with_children(|root| {
        root.spawn((
            dev_window(menu_place(at, window.size())),
            CellMenuSheet,
            // a sheet of the editor's, so a click on it never reaches the world behind it
            PropsSheet,
            Hovered::default(),
        ))
        .with_children(|sheet| {
            for (words, choice) in items {
                let kind = if matches!(choice, CellChoice::Delete(_)) {
                    ButtonKind::Danger
                } else {
                    ButtonKind::Choice
                };
                sheet.spawn((words_button(words, kind), choice));
            }
        });
    });
}

/// Closes the menu, or gives the rename up, when a press lands outside it. That press goes on
/// to what is under it, as in the OS's own menus.
fn close_cell_menu_on_press_outside(
    mut commands: Commands,
    mouse: Res<ButtonInput<MouseButton>>,
    sheets: Query<&Hovered, With<CellMenuSheet>>,
    menus: Query<Entity, With<CellMenu>>,
) {
    if mouse.get_just_pressed().next().is_none() || sheets.iter().any(Hovered::get) {
        return;
    }
    close_menus(&mut commands, &menus);
}

/// Runs the menu item pressed: a favourite marked or unmarked, the rename field in the menu's
/// place, a delete or a new folder asked of the props folder.
fn apply_cell_choice(
    activate: On<Activate>,
    mut commands: Commands,
    choices: Query<&CellChoice>,
    menus: Query<(Entity, &CellMenu)>,
    mut preferences: ResMut<PropPreferences>,
    open: Res<OpenFolder>,
    library: Res<PropLibrary>,
    folders: Res<PropFolders>,
    window: Single<&Window, With<PrimaryWindow>>,
) {
    let Ok(choice) = choices.get(activate.entity) else {
        return;
    };
    let at = menus.iter().next().map(|(_, menu)| menu.at);
    for (menu, _) in &menus {
        commands.entity(menu).despawn();
    }
    match choice {
        CellChoice::Favourite(file) => preferences.toggle_favourite(file),
        CellChoice::Rename(path) => {
            let Some(at) = at else {
                return;
            };
            let taken = names_in(parent_folder(path), &library, &folders);
            let name = path.rsplit('/').next().unwrap_or(path);
            commands.spawn(menu_root(at)).with_children(|root| {
                root.spawn((
                    dev_window(menu_place(at, window.size())),
                    CellMenuSheet,
                    PropsSheet,
                    Hovered::default(),
                    children![rename_field(name, path, taken)],
                ));
            });
        }
        CellChoice::Delete(path) => {
            commands.trigger(PropFileEdit::Delete { path: path.clone() });
        }
        CellChoice::NewFolder => {
            commands.trigger(PropFileEdit::NewFolder {
                parent: open.0.clone(),
            });
        }
    }
}

/// Keeps the name typed when Enter is pressed, asking the props folder for the rename, or
/// gives it up when the field loses the keyboard to a click elsewhere; paints the field red
/// while the name would be refused, and Enter does nothing then.
fn finish_prop_rename(
    mut commands: Commands,
    keys: Res<ButtonInput<KeyCode>>,
    focus: Res<InputFocus>,
    kind: Option<Res<PropKind>>,
    mut fields: Query<(
        Entity,
        &EditableText,
        &mut PropRenameField,
        &mut BorderColor,
    )>,
    menus: Query<Entity, With<CellMenu>>,
) {
    let extensions = kind.as_deref().map_or(&[][..], |kind| kind.extensions);
    for (entity, text, mut field, mut border) in &mut fields {
        let typed = text.value().to_string();
        let name = typed.trim();
        let old = field
            .path
            .rsplit('/')
            .next()
            .unwrap_or(&field.path)
            .to_string();
        let is_file = is_prop_file(&old, extensions);
        let allowed = is_name_allowed(name, &old, &field.taken)
            && (!is_file || is_prop_file(name, extensions));
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
        close_menus(&mut commands, &menus);
        if entered && allowed && name != old {
            commands.trigger(PropFileEdit::Rename {
                from: field.path.clone(),
                to: inside_folder(parent_folder(&field.path), name),
            });
        }
    }
}

/// Closes the menu, or gives the rename up, when Esc dismisses it, before the editor's window.
fn dismiss_cell_menu(
    dismiss: On<Dismiss>,
    mut commands: Commands,
    menus: Query<(), With<CellMenu>>,
) {
    if menus.contains(dismiss.entity) {
        commands.entity(dismiss.entity).despawn();
    }
}

// ==== Helpers ================================================================

/// Returns the root of a menu, or of the rename field, opened at `at`: the pointer let through
/// around its sheet, as an OS menu.
fn menu_root(at: Vec2) -> impl Bundle {
    (
        floating_root(MENU_Z_INDEX),
        Pickable::IGNORE,
        DevUi,
        ClosesOnEsc,
        CellMenu { at },
        DespawnOnExit(PropEditor::On),
    )
}

/// Returns the rename field for `path`, holding its `name`, the keyboard handed to it at once.
fn rename_field(name: &str, path: &str, taken: Vec<String>) -> impl Bundle {
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
            width: px(RENAME_FIELD_WIDTH_PX),
            padding: UiRect::axes(px(RENAME_FIELD_PADDING_X_PX), px(RENAME_FIELD_PADDING_Y_PX)),
            border: UiRect::all(px(BORDER_PX)),
            border_radius: BorderRadius::all(px(RENAME_FIELD_RADIUS_PX)),
            ..default()
        },
        BackgroundColor(FIELD_FILL),
        BorderColor::all(ACCENT),
        PropRenameField {
            path: path.to_string(),
            taken,
            had_keyboard: false,
        },
    )
}

/// Despawns every menu and rename field.
fn close_menus(commands: &mut Commands, menus: &Query<Entity, With<CellMenu>>) {
    for menu in menus {
        commands.entity(menu).despawn();
    }
}

/// Returns the names in the folder `folder` inside the props folder: its props' files and its
/// folders, which a rename there must not take.
fn names_in(folder: &str, library: &PropLibrary, folders: &PropFolders) -> Vec<String> {
    let files = library
        .0
        .iter()
        .filter(|entry| folder_of(&entry.file) == folder)
        .map(|entry| entry.name.clone());
    let subfolders = folders
        .0
        .iter()
        .filter(|path| parent_folder(path) == folder)
        .map(|path| path.rsplit('/').next().unwrap_or(path).to_string());
    files.chain(subfolders).collect()
}

/// Returns the path inside the props folder of a prop's file, from its asset path.
fn inside_props(file: &str) -> &str {
    file.strip_prefix(PROPS_DIR_NAME)
        .and_then(|rest| rest.strip_prefix('/'))
        .unwrap_or(file)
}

// ==== Tests ==================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dev_tools::prop_editor::library::LibraryEntry;

    #[test]
    fn a_rename_must_not_take_a_name_its_folder_holds() {
        let library = PropLibrary(vec![
            LibraryEntry {
                file: "props/trees/pine.glb".to_string(),
                name: "pine.glb".to_string(),
                size: 1,
            },
            LibraryEntry {
                file: "props/rock.glb".to_string(),
                name: "rock.glb".to_string(),
                size: 1,
            },
        ]);
        let folders = PropFolders(vec!["trees".to_string(), "trees/firs".to_string()]);
        assert_eq!(names_in("trees", &library, &folders), ["pine.glb", "firs"]);
        assert_eq!(names_in("", &library, &folders), ["rock.glb", "trees"]);
    }

    #[test]
    fn a_prop_file_is_named_inside_the_props_folder() {
        assert_eq!(inside_props("props/trees/pine.glb"), "trees/pine.glb");
    }
}
