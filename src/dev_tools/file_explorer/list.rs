use bevy::prelude::*;
use bevy::ui_widgets::ScrollArea;

use super::listing::{
    Entry, Place, date_text, kind_of, open_with_os, read_folder, read_recents, size_text,
};
use super::rename::rename_field;
use super::scroll::{ExplorerScroll, scroll_frame, scrolling_column};
use super::trash::read_trash;
use super::{DropTarget, Explorer, Shown};
use crate::ui::{
    BUTTON_PADDING_PX, ButtonKind, ROW_BUTTON_PADDING_PX, SMALL_FONT_SIZE_PX, TEXT, TEXT_MUTED,
    font, icon, muted_text, row_button,
};

// ==== Config =================================================================

// ---- words: English, as every dev tool's, never read in a release build -----
const NAME_HEADING: &str = "Name";
const DATE_HEADING: &str = "Date";
const SIZE_HEADING: &str = "Size";
// a folder's size, which the list does not add up
const NO_SIZE: &str = "—";

// ---- columns: the name takes what the date and the size leave ---------------
const DATE_COLUMN_WIDTH_PX: f32 = 150.0;
const SIZE_COLUMN_WIDTH_PX: f32 = 80.0;
// between an entry's icon and its name
const ICON_GAP_PX: f32 = 8.0;

// ==== Plugin =================================================================

/// The explorer's list: the folder shown, a row an entry, selected with a click and entered
/// with a double click.
pub(super) fn plugin(app: &mut App) {
    app.add_systems(
        Update,
        (fill_list, mark_selected_row)
            .chain()
            .run_if(resource_exists::<Explorer>),
    );
}

// ==== Widgets ================================================================

/// Returns the list's heading row and the list itself, which `fill_list` fills, scrolling
/// when the folder holds more than the window shows.
pub(super) fn list() -> impl Bundle {
    (
        Node {
            flex_direction: FlexDirection::Column,
            flex_grow: 1.0,
            // so the list can be shorter than its rows and scroll, instead of pushing the
            // window past the screen
            min_height: px(0),
            ..default()
        },
        children![
            heading_row(),
            (
                scroll_frame(),
                children![(scrolling_column(), ScrollArea, ExplorerList, ExplorerScroll)],
            ),
        ],
    )
}

// ==== Components =============================================================

/// Marks the node the list's rows are spawned in, whose empty space has a menu of its own.
#[derive(Component)]
pub(super) struct ExplorerList;

/// Marks a row of the list, with the name of the entry it shows.
#[derive(Component)]
pub(super) struct ListRow(pub(super) String);

// ==== Systems ================================================================

/// Lists the folder shown, or Recents, or Trash with where each entry came from, each entry
/// a row, when the window opens, when what is shown changes and when a rename starts or ends:
/// the entry renamed shows its name as a text field. A click selects a row; a double click
/// enters a folder or opens a file in the OS's own app. Not rebuilt when only the selection
/// changes: a row rebuilt between the two clicks of a double click would be a new entity, and
/// its second click a first one.
fn fill_list(
    mut commands: Commands,
    explorer: Res<Explorer>,
    lists: Query<(Entity, Ref<ExplorerList>)>,
    mut drawn: Local<Option<(Shown, Option<String>)>>,
) {
    let Ok((list, marker)) = lists.single() else {
        *drawn = None;
        return;
    };
    let wanted = (explorer.shown(), explorer.renaming.clone());
    if !marker.is_added() && drawn.as_ref() == Some(&wanted) {
        return;
    }
    // a new folder opens at its top, not scrolled as far down as the one before; a rename
    // keeps the list where it was
    let same_folder = drawn.as_ref().is_some_and(|(shown, _)| *shown == wanted.0);
    let mut list_commands = commands.entity(list);
    list_commands.despawn_children();
    if !same_folder {
        list_commands.insert(ScrollPosition::default());
    }
    let entries: Vec<(Entry, Option<String>)> = match (explorer.place, explorer.place.root()) {
        (_, None) => Vec::new(),
        (Place::Recents, Some(gallery)) => with_no_origin(read_recents(&gallery)),
        (Place::Trash, Some(trash)) => read_trash(&trash),
        (Place::Gallery | Place::Assets, Some(_)) => with_no_origin(
            explorer
                .folder_on_disk()
                .map(|folder| read_folder(&folder))
                .unwrap_or_default(),
        ),
    };
    let names: Vec<String> = entries
        .iter()
        .map(|(entry, _)| entry.name.clone())
        .collect();
    let has_folders = matches!(explorer.place, Place::Gallery | Place::Assets);
    for (entry, from) in entries {
        let name = entry.name.clone();
        let is_folder = entry.is_folder;
        // a folder in Trash is one entry deleted whole, not a folder to go into
        let can_enter = is_folder && has_folders;
        // a line around the button: a row button grows to fill what holds it, which in this
        // column would stretch a few rows down the whole list; a line grows only across
        let line = commands.spawn((Node::default(), ChildOf(list))).id();
        if explorer.renaming.as_ref() == Some(&entry.name) {
            commands.spawn((renamed_row(&entry, names.clone()), ChildOf(line)));
            continue;
        }
        let mut row = commands.spawn((
            row_button(row_content(&entry, from)),
            ListRow(entry.name),
            ChildOf(line),
        ));
        if is_folder && has_folders {
            row.insert(DropTarget {
                place: explorer.place,
                folder: explorer.folder.join(&name),
            });
        }
        row
            // the press lands on the row, never on a text in it: Bevy counts a double click
            // on the one entity hit, so a first click on the name and a second on the date
            // would count as two first clicks
            .insert_recursive::<Children>(Pickable::IGNORE)
            .insert(Pickable::default())
            .observe(
                move |click: On<PointerClick>, mut explorer: ResMut<Explorer>| {
                    if click.button != PointerButton::Primary {
                        return;
                    }
                    explorer.selected = Some(name.clone());
                    // exactly the second: a third click is not another double click
                    if click.count != 2 {
                        return;
                    }
                    if can_enter {
                        explorer.folder.push(&name);
                        explorer.selected = None;
                    } else if let Some(path) = explorer.selected_on_disk() {
                        open_with_os(&path);
                    }
                },
            );
    }
    *drawn = Some(wanted);
}

/// Fills the selected row white, as a Primary button is, and leaves the rest as Choices,
/// which `paint_buttons` then paints. Writes only the rows that change.
fn mark_selected_row(explorer: Res<Explorer>, mut rows: Query<(&ListRow, &mut ButtonKind)>) {
    for (ListRow(name), mut kind) in &mut rows {
        let selected = explorer.selected.as_ref() == Some(name);
        let wanted = if selected {
            ButtonKind::Primary
        } else {
            ButtonKind::Choice
        };
        if *kind != wanted {
            *kind = wanted;
        }
    }
}

// ==== Helpers ================================================================

/// Returns the row of headings over the list's columns.
fn heading_row() -> impl Bundle {
    (
        Node {
            // the padding of a row's button, so each heading sits over its column
            padding: UiRect {
                left: px(ROW_BUTTON_PADDING_PX),
                right: px(BUTTON_PADDING_PX),
                ..default()
            },
            ..default()
        },
        children![
            (
                Node {
                    flex_grow: 1.0,
                    ..default()
                },
                Text::new(NAME_HEADING),
                muted_text(SMALL_FONT_SIZE_PX),
            ),
            column(
                DATE_COLUMN_WIDTH_PX,
                (Text::new(DATE_HEADING), muted_text(SMALL_FONT_SIZE_PX))
            ),
            column(
                SIZE_COLUMN_WIDTH_PX,
                (Text::new(SIZE_HEADING), muted_text(SMALL_FONT_SIZE_PX))
            ),
        ],
    )
}

/// Returns the entries of a place that keeps no record of where they came from.
fn with_no_origin(entries: Vec<Entry>) -> Vec<(Entry, Option<String>)> {
    entries.into_iter().map(|entry| (entry, None)).collect()
}

/// Returns what a row shows: the entry's icon and name, and in Trash where it came from,
/// muted after it.
fn row_content(entry: &Entry, from: Option<String>) -> impl Bundle {
    let from = from.map(|from| format!("from {from}")).unwrap_or_default();
    row_columns(
        entry,
        (
            Node {
                column_gap: px(ICON_GAP_PX),
                ..default()
            },
            children![
                (
                    Text::new(entry.name.clone()),
                    font(SMALL_FONT_SIZE_PX),
                    TextColor(TEXT),
                ),
                (
                    Text::new(from),
                    font(SMALL_FONT_SIZE_PX),
                    TextColor(TEXT_MUTED)
                ),
            ],
        ),
    )
}

/// Returns the row of the entry being renamed: its name a text field, where a button's row
/// would be. Not a button, so a click in the field moves its cursor and selects nothing.
/// `taken` is every name in the folder.
fn renamed_row(entry: &Entry, taken: Vec<String>) -> impl Bundle {
    (
        Node {
            flex_grow: 1.0,
            // a row button's padding, so the columns stay under their headings
            padding: UiRect {
                right: px(BUTTON_PADDING_PX),
                ..UiRect::all(px(ROW_BUTTON_PADDING_PX))
            },
            ..default()
        },
        children![row_columns(entry, rename_field(&entry.name, taken))],
    )
}

/// Returns the columns of a row: the entry's icon beside `name`, its date, its size.
fn row_columns(entry: &Entry, name: impl Bundle) -> impl Bundle {
    let size = if entry.is_folder {
        NO_SIZE.to_string()
    } else {
        size_text(entry.size_bytes)
    };
    (
        Node {
            flex_grow: 1.0,
            align_items: AlignItems::Center,
            ..default()
        },
        children![
            (
                Node {
                    flex_grow: 1.0,
                    align_items: AlignItems::Center,
                    column_gap: px(ICON_GAP_PX),
                    ..default()
                },
                children![icon(kind_of(entry).icon()), name],
            ),
            column(
                DATE_COLUMN_WIDTH_PX,
                (
                    Text::new(date_text(entry.changed_secs)),
                    font(SMALL_FONT_SIZE_PX),
                    TextColor(TEXT),
                )
            ),
            column(
                SIZE_COLUMN_WIDTH_PX,
                (Text::new(size), font(SMALL_FONT_SIZE_PX), TextColor(TEXT),)
            ),
        ],
    )
}

/// Returns a column of a fixed width holding `text`.
fn column(width_px: f32, text: impl Bundle) -> impl Bundle {
    (
        Node {
            width: px(width_px),
            flex_shrink: 0.0,
            ..default()
        },
        children![text],
    )
}
