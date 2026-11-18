use bevy::picking::hover::Hovered;
use bevy::prelude::*;
use bevy::ui_widgets::Activate;
use bevy::window::PrimaryWindow;

use super::Explorer;
use super::actions::{Change, make_new_folder, request_change};
use super::dialog::{EXPLORER_DIALOG_Z_INDEX, ExplorerDialog, close_explorer_dialogs};
use super::list::{ExplorerList, ListRow};
use super::listing::{Place, open_with_os};
use super::trash::{ask_to_delete_for_good, ask_to_empty_trash, is_known, restore};
use crate::dev_tools::DevUi;
use crate::ui::{
    BORDER_PX, ButtonKind, ClosesOnEsc, SMALL_FONT_SIZE_PX, TEXT, card_glass, floating_root, font,
    laid_out_button,
};

// ==== Config =================================================================

// ---- the sheet: small and glass, where the pointer is -----------------------
const MENU_WIDTH_PX: f32 = 180.0;
const MENU_PADDING_PX: f32 = 6.0;
const MENU_RADIUS_PX: f32 = 12.0;

// ---- items ------------------------------------------------------------------
const MENU_ITEM_HEIGHT_PX: f32 = 32.0;
const MENU_ITEM_PADDING_PX: f32 = 12.0;
const MENU_ITEM_RADIUS_PX: f32 = 8.0;

// ==== Plugin =================================================================

/// The list's right-click menus: an entry's (Open, Rename, Delete; in Trash Restore and
/// Delete for Good) and the empty space's (New Folder; in Trash Empty Trash).
pub(super) fn plugin(app: &mut App) {
    app.add_systems(
        Update,
        close_menu_on_press_elsewhere.run_if(any_with_component::<MenuPanel>),
    );
    // no run_if, as the explorer's buttons work over any screen; none asks for a resource
    // that may not exist yet, since every press in the game comes here
    app.add_observer(open_context_menu);
    app.add_observer(pick_menu_item);
}

// ==== Types ==================================================================

/// What a right-click menu offers.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum MenuItem {
    Open,
    Rename,
    Delete,
    NewFolder,
    Restore,
    DeleteForGood,
    EmptyTrash,
}

impl MenuItem {
    /// Returns the item's words, English as every dev tool's.
    fn words(self) -> &'static str {
        match self {
            MenuItem::Open => "Open",
            MenuItem::Rename => "Rename",
            MenuItem::Delete => "Delete",
            MenuItem::NewFolder => "New Folder",
            MenuItem::Restore => "Restore",
            MenuItem::DeleteForGood => "Delete for Good",
            MenuItem::EmptyTrash => "Empty Trash",
        }
    }

    /// Returns how the item is painted: red for what nothing brings back, the others bare
    /// until the pointer comes.
    fn kind(self) -> ButtonKind {
        match self {
            MenuItem::DeleteForGood | MenuItem::EmptyTrash => ButtonKind::Danger,
            _ => ButtonKind::Quiet,
        }
    }
}

// ==== Components =============================================================

/// Marks the sheet of a right-click menu, whose root is an `ExplorerDialog`.
#[derive(Component)]
struct MenuPanel;

/// An item of a right-click menu: what it does, and the entry it does it to, by its name in
/// the list, or `None` for the menu of the list's empty space.
#[derive(Component)]
struct MenuChoice {
    item: MenuItem,
    entry: Option<String>,
}

// ==== Systems ================================================================

/// Closes a right-click menu when a press lands outside it. That press goes on to what is
/// under it, as in the OS's own menus, and a right click there opens a menu of its own.
fn close_menu_on_press_elsewhere(
    mut commands: Commands,
    mouse: Res<ButtonInput<MouseButton>>,
    panels: Query<(Ref<Hovered>, &ChildOf), With<MenuPanel>>,
) {
    if mouse.get_just_pressed().next().is_none() {
        return;
    }
    for (hovered, root) in &panels {
        // not one this very press opened, a right click on a row, which no pointer is over yet
        if !hovered.is_added() && !hovered.get() {
            commands.entity(root.parent()).despawn();
        }
    }
}

/// Opens a right-click menu where the pointer is: an entry's on a row, the folder's on the
/// list's empty space. A right-clicked row is selected first, as in the OS's explorer.
fn open_context_menu(
    mut click: On<PointerClick>,
    mut commands: Commands,
    rows: Query<&ListRow>,
    lists: Query<(), With<ExplorerList>>,
    windows: Query<&Window, With<PrimaryWindow>>,
    explorer: Option<ResMut<Explorer>>,
) {
    if click.button != PointerButton::Secondary {
        return;
    }
    let entry = if let Ok(ListRow(name)) = rows.get(click.entity) {
        // the row's menu, not also the one of the list's empty space it sits in
        click.propagate(false);
        Some(name.clone())
    } else if lists.contains(click.entity) {
        None
    } else {
        return;
    };
    let (Some(mut explorer), Ok(window)) = (explorer, windows.single()) else {
        return;
    };
    if entry.is_some() {
        explorer.selected.clone_from(&entry);
    }
    let restorable = match (explorer.place, &entry, Place::Trash.root()) {
        (Place::Trash, Some(name), Some(trash)) => is_known(&trash, name),
        _ => false,
    };
    let items = menu_items(explorer.place, entry.is_some(), restorable);
    close_explorer_dialogs(&mut commands);
    if items.is_empty() {
        return;
    }
    let root = commands
        .spawn((
            // no backdrop and the pointer let through around the sheet, as an OS menu: a
            // press elsewhere closes it and still lands where it was aimed
            floating_root(EXPLORER_DIALOG_Z_INDEX),
            Pickable::IGNORE,
            DevUi,
            ClosesOnEsc,
            ExplorerDialog,
        ))
        .id();
    let panel = commands
        .spawn((
            Node {
                width: px(MENU_WIDTH_PX),
                flex_direction: FlexDirection::Column,
                padding: UiRect::all(px(MENU_PADDING_PX)),
                border: UiRect::all(px(BORDER_PX)),
                border_radius: BorderRadius::all(px(MENU_RADIUS_PX)),
                ..menu_place(click.pointer.position, window.size())
            },
            card_glass(),
            Hovered::default(),
            MenuPanel,
            ChildOf(root),
        ))
        .id();
    for item in items {
        commands.spawn((
            laid_out_button(
                Node {
                    height: px(MENU_ITEM_HEIGHT_PX),
                    padding: UiRect::horizontal(px(MENU_ITEM_PADDING_PX)),
                    align_items: AlignItems::Center,
                    border_radius: BorderRadius::all(px(MENU_ITEM_RADIUS_PX)),
                    ..default()
                },
                item.kind(),
                (
                    Text::new(item.words()),
                    font(SMALL_FONT_SIZE_PX),
                    TextColor(TEXT),
                ),
            ),
            MenuChoice {
                item,
                entry: entry.clone(),
            },
            ChildOf(panel),
        ));
    }
}

/// Does what the menu item pressed offers, and closes the menu.
fn pick_menu_item(
    activate: On<Activate>,
    mut commands: Commands,
    choices: Query<&MenuChoice>,
    explorer: Option<ResMut<Explorer>>,
) {
    let (Ok(choice), Some(mut explorer)) = (choices.get(activate.entity), explorer) else {
        return;
    };
    close_explorer_dialogs(&mut commands);
    match (choice.item, choice.entry.clone()) {
        (MenuItem::Open, Some(name)) => {
            if let Some(path) = explorer.on_disk(&name) {
                open_with_os(&path);
            }
        }
        (MenuItem::Rename, Some(name)) => {
            explorer.selected = Some(name.clone());
            explorer.renaming = Some(name);
        }
        (MenuItem::Delete, Some(name)) => {
            let change = Change::Delete {
                place: explorer.disk_place(),
                inside: explorer.inside(&name),
            };
            request_change(&mut commands, &mut explorer, change);
        }
        (MenuItem::NewFolder, _) => make_new_folder(&mut explorer),
        (MenuItem::Restore, Some(name)) => {
            if let Some(trash) = Place::Trash.root()
                && let Err(error) = restore(&trash, &name, Place::root)
            {
                error!("could not restore {name}: {error}");
            }
            explorer.selected = None;
            explorer.disk_changes += 1;
        }
        (MenuItem::DeleteForGood, Some(name)) => ask_to_delete_for_good(&mut commands, name),
        (MenuItem::EmptyTrash, _) => ask_to_empty_trash(&mut commands),
        // an entry's item with no entry: no menu offers one
        _ => {}
    }
}

// ==== Helpers ================================================================

/// Returns what a right-click menu offers, on an entry or on the empty space of the list of
/// `place`; `restorable` when the entry is in Trash and the trash knows where it came from.
fn menu_items(place: Place, on_entry: bool, restorable: bool) -> Vec<MenuItem> {
    match (place, on_entry) {
        // Recents' names are paths through its folders, so it renames nothing
        (Place::Recents, true) => vec![MenuItem::Open, MenuItem::Delete],
        (Place::Recents, false) => Vec::new(),
        (Place::Gallery | Place::Assets, true) => {
            vec![MenuItem::Open, MenuItem::Rename, MenuItem::Delete]
        }
        (Place::Gallery | Place::Assets, false) => vec![MenuItem::NewFolder],
        (Place::Trash, true) if restorable => vec![MenuItem::Restore, MenuItem::DeleteForGood],
        (Place::Trash, true) => vec![MenuItem::DeleteForGood],
        (Place::Trash, false) => vec![MenuItem::EmptyTrash],
    }
}

/// Returns where a menu opened at `pointer` in a window `window` big sits: its corner at the
/// pointer, and the menu on the side of it with the most room, so it never runs off the
/// window. The drawer is at the bottom, so a menu there opens upward.
fn menu_place(pointer: Vec2, window: Vec2) -> Node {
    let mut place = Node {
        position_type: PositionType::Absolute,
        ..default()
    };
    if pointer.x > window.x / 2.0 {
        place.right = px(window.x - pointer.x);
    } else {
        place.left = px(pointer.x);
    }
    if pointer.y > window.y / 2.0 {
        place.bottom = px(window.y - pointer.y);
    } else {
        place.top = px(pointer.y);
    }
    place
}

// ==== Tests ==================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn each_place_offers_its_own_menu() {
        assert_eq!(
            menu_items(Place::Gallery, true, false),
            [MenuItem::Open, MenuItem::Rename, MenuItem::Delete]
        );
        assert_eq!(
            menu_items(Place::Assets, false, false),
            [MenuItem::NewFolder]
        );
        assert!(menu_items(Place::Recents, false, false).is_empty());
        assert_eq!(
            menu_items(Place::Trash, true, true),
            [MenuItem::Restore, MenuItem::DeleteForGood]
        );
        assert_eq!(
            menu_items(Place::Trash, true, false),
            [MenuItem::DeleteForGood]
        );
    }

    #[test]
    fn a_menu_opens_away_from_the_nearest_edges() {
        let window = Vec2::new(1000.0, 800.0);
        let low_right = menu_place(Vec2::new(900.0, 700.0), window);
        assert_eq!((low_right.right, low_right.bottom), (px(100), px(100)));
        let high_left = menu_place(Vec2::new(100.0, 50.0), window);
        assert_eq!((high_left.left, high_left.top), (px(100), px(50)));
    }
}
