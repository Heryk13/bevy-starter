use bevy::picking::hover::Hovered;
use bevy::prelude::*;
use bevy::ui_widgets::Activate;
use bevy::window::PrimaryWindow;
use std::path::Path;

use super::super::file_explorer::{menu_place, trash_asset};
use super::super::{DEV_TOOLS_Z_INDEX, DevUi, dev_window};
use super::PropEditor;
use super::edits::{Held, MapEdit, PropsSheet};
use super::window::PropCell;
use crate::save::Session;
use crate::screens::gameplay::map::{Prop, PropFile};
use crate::ui::{ButtonKind, ClosesOnEsc, Dismiss, floating_root, words_button};

// ==== Config =================================================================

// ---- words: English, as every dev tool's, never read in a release build -----
const DELETE_WORDS: &str = "Delete";

// ---- menu: over the props window, as the explorer's menu is over the explorer
const MENU_Z_INDEX: GlobalZIndex = GlobalZIndex(DEV_TOOLS_Z_INDEX.0 + 10);

// ==== Plugin =================================================================

/// A prop file's menu, a right click on its cell opens: Delete moves the file to the file
/// explorer's Trash, which can put it back, and deletes every prop placed from it, through
/// the host in a shared game, as the Delete key does. The others in a shared game keep their
/// copy of the file: files only ever travel to them.
pub(super) fn plugin(app: &mut App) {
    app.add_systems(
        Update,
        close_cell_menu_on_press_outside.run_if(in_state(PropEditor::On)),
    );
    app.add_observer(open_cell_menu.run_if(in_state(PropEditor::On)));
    app.add_observer(delete_prop_file.run_if(in_state(PropEditor::On)));
    app.add_observer(dismiss_cell_menu.run_if(in_state(PropEditor::On)));
}

// ==== Components =============================================================

/// Marks the root of a prop file's menu.
#[derive(Component)]
struct CellMenu;

/// Marks the menu's sheet, whose `Hovered` tells a press on it from one outside.
#[derive(Component)]
struct CellMenuSheet;

/// Marks the menu's Delete, with the asset path of the file it deletes.
#[derive(Component)]
struct DeletePropFile(String);

// ==== Systems ================================================================

/// Opens a prop file's menu where the pointer is on a right click on its cell, in place of
/// one already open.
fn open_cell_menu(
    click: On<PointerClick>,
    mut commands: Commands,
    cells: Query<&PropCell>,
    menus: Query<Entity, With<CellMenu>>,
    window: Single<&Window, With<PrimaryWindow>>,
) {
    if click.button != PointerButton::Secondary {
        return;
    }
    let Ok(cell) = cells.get(click.entity) else {
        return;
    };
    for menu in &menus {
        commands.entity(menu).despawn();
    }
    commands.spawn((
        // the pointer let through around the sheet, as an OS menu
        floating_root(MENU_Z_INDEX),
        Pickable::IGNORE,
        DevUi,
        ClosesOnEsc,
        CellMenu,
        DespawnOnExit(PropEditor::On),
        children![(
            dev_window(menu_place(click.pointer.position, window.size())),
            CellMenuSheet,
            // a sheet of the editor's, so a click on it never reaches the world behind it
            PropsSheet,
            Hovered::default(),
            children![(
                words_button(DELETE_WORDS, ButtonKind::Danger),
                DeletePropFile(cell.0.clone()),
            )],
        )],
    ));
}

/// Closes the menu when a press lands outside it. That press goes on to what is under it, as
/// in the OS's own menus.
fn close_cell_menu_on_press_outside(
    mut commands: Commands,
    mouse: Res<ButtonInput<MouseButton>>,
    sheets: Query<&Hovered, With<CellMenuSheet>>,
    menus: Query<Entity, With<CellMenu>>,
) {
    if mouse.get_just_pressed().next().is_none() || sheets.iter().any(Hovered::get) {
        return;
    }
    for menu in &menus {
        commands.entity(menu).despawn();
    }
}

/// Deletes a prop file when its Delete is pressed: every prop placed from it, a new one of it
/// in the hand, and the file itself, into the Trash.
fn delete_prop_file(
    activate: On<Activate>,
    mut commands: Commands,
    buttons: Query<&DeletePropFile>,
    menus: Query<Entity, With<CellMenu>>,
    in_hand: Query<(Entity, &PropFile), (With<Held>, Without<Prop>)>,
    session: Res<Session>,
) {
    let Ok(DeletePropFile(file)) = buttons.get(activate.entity) else {
        return;
    };
    for menu in &menus {
        commands.entity(menu).despawn();
    }
    for (entity, held) in &in_hand {
        if held.0 == *file {
            commands.entity(entity).despawn();
        }
    }
    let placed = session.game.map.iter().flat_map(|map| &map.props);
    for prop in placed.filter(|prop| prop.file == *file) {
        commands.trigger(MapEdit::Delete { id: prop.id });
    }
    match trash_asset(Path::new(file)) {
        Ok(()) => info!("moved the prop file {file} to the Trash"),
        Err(err) => error!("could not delete the prop file {file}: {err}"),
    }
}

/// Closes the menu when Esc dismisses it, before the editor's window.
fn dismiss_cell_menu(
    dismiss: On<Dismiss>,
    mut commands: Commands,
    menus: Query<(), With<CellMenu>>,
) {
    if menus.contains(dismiss.entity) {
        commands.entity(dismiss.entity).despawn();
    }
}
