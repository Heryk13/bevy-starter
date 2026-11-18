use bevy::prelude::*;
use bevy::ui_widgets::{Activate, Button};
use std::path::PathBuf;

use super::drawer::resize_grip;
use super::listing::{Place, path_parts};
use super::{DropTarget, Explorer, FileExplorer, Shown, actions, trash};
use crate::ui::{
    BODY_FONT_SIZE_PX, BORDER, BORDER_PX, ButtonKind, Icon, TEXT, TEXT_MUTED, custom_icon, font,
    icon, laid_out_button,
};

// ==== Config =================================================================

// ---- words: English, as every dev tool's, never read in a release build -----
const CLOSE_LABEL: &str = "Close";

// ---- the places, each an icon, in the order they sit ------------------------
const PLACES: [(Place, Icon); 4] = [
    (Place::Recents, Icon::Clock),
    (Place::Gallery, Icon::Images),
    (Place::Assets, Icon::Package),
    (Place::Trash, Icon::Trash2),
];
// a place is one button, its icon over its name, both inside its fill: small, so the title
// bar takes little of the drawer's room, and wide enough for the longest name, Recents
const PLACE_WIDTH_PX: f32 = 52.0;
// the close button beside them is as tall, a circle
const PLACE_HEIGHT_PX: f32 = 40.0;
const PLACE_RADIUS_PX: f32 = 10.0;
const PLACE_ICON_PX: f32 = 18.0;
// a place's name: smaller than the list's words, so it fits under the icon
const PLACE_NAME_FONT_SIZE_PX: f32 = 10.0;
const PLACE_GAP_PX: f32 = 4.0;

// ---- the bar ----------------------------------------------------------------
const TITLE_BAR_GAP_PX: f32 = 10.0;
// the X in the close button: larger than an icon beside text, which is all it is
const CLOSE_ICON_PX: f32 = 28.0;
// either side of the title bar, wide enough for the close button and the places on the
// left, the same on the right for Empty Trash, so the middle sits in the middle
const TITLE_SIDE_WIDTH_PX: f32 = 280.0;
// the lines either side of the places
const SEPARATOR_HEIGHT_PX: f32 = 24.0;
// between the parts of the path
const PATH_GAP_PX: f32 = 6.0;

// ==== Plugin =================================================================

/// The explorer's title bar: the close button and the places on the left, the grip over the
/// path in the middle, and "+ Folder" or Empty Trash on the right. A press on a place shows it,
/// and one on a part of the path goes back to it; an entry dragged onto either goes into it.
pub(super) fn plugin(app: &mut App) {
    app.add_systems(
        Update,
        (fill_places, show_path).run_if(resource_exists::<Explorer>),
    );
    // no run_if: the explorer opens over any screen
    app.add_observer(close_file_explorer);
    app.add_observer(go_back_along_path);
}

// ==== Widgets ================================================================

/// Returns the title bar: the close button and the places on the left, a line either side of
/// the places; in the middle the grip over the path, which `show_path` writes; and on the
/// right "+ Folder", or in Trash Empty Trash. The two sides are as wide as each other, so the
/// middle sits in the middle of the window. Everything lines up at the top.
pub(super) fn title_bar() -> impl Bundle {
    (
        Node {
            align_items: AlignItems::Start,
            column_gap: px(TITLE_BAR_GAP_PX),
            ..default()
        },
        children![
            (
                title_side(JustifyContent::Start, AlignItems::Start),
                children![
                    (
                        // a circle as tall as the places beside it, with a large X
                        laid_out_button(
                            Node {
                                width: px(PLACE_HEIGHT_PX),
                                height: px(PLACE_HEIGHT_PX),
                                flex_shrink: 0.0,
                                justify_content: JustifyContent::Center,
                                align_items: AlignItems::Center,
                                border_radius: BorderRadius::MAX,
                                ..default()
                            },
                            ButtonKind::Close,
                            custom_icon(Icon::X, CLOSE_ICON_PX, TEXT),
                        ),
                        AccessibleLabel::new(CLOSE_LABEL),
                        CloseFileExplorer,
                    ),
                    // straight lines setting the places apart, from closing the window on one
                    // side and from the path on the other
                    separator(),
                    (
                        Node {
                            column_gap: px(PLACE_GAP_PX),
                            ..default()
                        },
                        ExplorerPlaces,
                    ),
                    separator(),
                ],
            ),
            // the middle: the grip over the path, both centred
            (
                Node {
                    flex_grow: 1.0,
                    flex_direction: FlexDirection::Column,
                    align_items: AlignItems::Center,
                    ..default()
                },
                children![
                    resize_grip(),
                    (
                        Node {
                            align_items: AlignItems::Center,
                            column_gap: px(PATH_GAP_PX),
                            ..default()
                        },
                        ExplorerPath,
                    ),
                ],
            ),
            (
                title_side(JustifyContent::End, AlignItems::Center),
                // never both: "+ Folder" where folders are made, Empty Trash in Trash
                children![actions::new_folder_button(), trash::empty_trash_button()],
            ),
        ],
    )
}

// ==== Components =============================================================

/// Marks the explorer's close button.
#[derive(Component)]
struct CloseFileExplorer;

/// Marks the row on the title bar the places are spawned in.
#[derive(Component)]
struct ExplorerPlaces;

/// Marks the node in the title bar the path is spawned in.
#[derive(Component)]
struct ExplorerPath;

/// Marks a part of the path, with the folder it goes back to.
#[derive(Component)]
struct PathPart(PathBuf);

// ==== Systems ================================================================

/// Spawns the places, each an icon over its name, the one shown filled white, when the
/// window opens and when what is shown changes.
fn fill_places(
    mut commands: Commands,
    explorer: Res<Explorer>,
    rows: Query<(Entity, Ref<ExplorerPlaces>)>,
    mut shown: Local<Option<Shown>>,
) {
    let Ok((row, marker)) = rows.single() else {
        *shown = None;
        return;
    };
    if !marker.is_added() && shown.as_ref() == Some(&explorer.shown()) {
        return;
    }
    commands.entity(row).despawn_children();
    for (place, symbol) in PLACES {
        let mut button = commands.spawn((
            laid_out_button(
                Node {
                    width: px(PLACE_WIDTH_PX),
                    height: px(PLACE_HEIGHT_PX),
                    justify_content: JustifyContent::Center,
                    align_items: AlignItems::Center,
                    border_radius: BorderRadius::all(px(PLACE_RADIUS_PX)),
                    ..default()
                },
                ButtonKind::Quiet,
                place_content(symbol, place.name()),
            ),
            ChildOf(row),
        ));
        if place == explorer.place {
            button.insert(ButtonKind::Primary);
        }
        if matches!(place, Place::Gallery | Place::Assets) {
            button.insert(DropTarget {
                place,
                folder: PathBuf::new(),
            });
        }
        button.observe(move |_: On<Activate>, mut explorer: ResMut<Explorer>| {
            *explorer = Explorer::at(place);
        });
    }
    *shown = Some(explorer.shown());
}

/// Writes the path from the place to the folder shown into the title bar, when the window
/// opens and when what is shown changes. Each part is words alone, and a press on one goes
/// back to it; the folder shown is the brightest.
fn show_path(
    mut commands: Commands,
    explorer: Res<Explorer>,
    paths: Query<(Entity, Ref<ExplorerPath>)>,
    mut shown: Local<Option<Shown>>,
) {
    let Ok((path, marker)) = paths.single() else {
        *shown = None;
        return;
    };
    if !marker.is_added() && shown.as_ref() == Some(&explorer.shown()) {
        return;
    }
    commands.entity(path).despawn_children();
    let parts = path_parts(explorer.place.name(), &explorer.folder);
    let last = parts.len().saturating_sub(1);
    for (index, (name, folder)) in parts.into_iter().enumerate() {
        if index > 0 {
            commands.spawn((icon(Icon::ChevronRight), ChildOf(path)));
        }
        let color = if index == last { TEXT } else { TEXT_MUTED };
        let mut part = commands.spawn((
            Text::new(name),
            font(BODY_FONT_SIZE_PX),
            TextColor(color),
            // a button in all but looks, so a press goes back to the folder
            Button,
            PathPart(folder.clone()),
            ChildOf(path),
        ));
        if matches!(explorer.place, Place::Gallery | Place::Assets) {
            part.insert(DropTarget {
                place: explorer.place,
                folder,
            });
        }
    }
    *shown = Some(explorer.shown());
}

/// Shows the folder a part of the path leads back to.
fn go_back_along_path(
    activate: On<Activate>,
    parts: Query<&PathPart>,
    // an `Option`: this observer sees every button pressed in the game, the menu's
    // included, before the explorer has ever opened and made the resource
    explorer: Option<ResMut<Explorer>>,
) {
    let (Ok(PathPart(folder)), Some(mut explorer)) = (parts.get(activate.entity), explorer) else {
        return;
    };
    explorer.folder = folder.clone();
    explorer.selected = None;
}

/// Closes the file explorer when its close button is pressed, sliding it away.
fn close_file_explorer(
    activate: On<Activate>,
    close_buttons: Query<(), With<CloseFileExplorer>>,
    mut explorers: Query<&mut FileExplorer>,
) {
    if close_buttons.contains(activate.entity) {
        for mut explorer in &mut explorers {
            explorer.closing = true;
        }
    }
}

// ==== Helpers ================================================================

/// Returns a straight upright line between two groups of the title bar.
fn separator() -> impl Bundle {
    (
        Node {
            width: px(BORDER_PX),
            height: px(SEPARATOR_HEIGHT_PX),
            ..default()
        },
        BackgroundColor(BORDER),
    )
}

/// Returns one side of the title bar, its content pushed to `justify` and lined up by `align`.
fn title_side(justify: JustifyContent, align: AlignItems) -> Node {
    Node {
        width: px(TITLE_SIDE_WIDTH_PX),
        flex_shrink: 0.0,
        justify_content: justify,
        align_items: align,
        column_gap: px(TITLE_BAR_GAP_PX),
        ..default()
    }
}

/// Returns what a place's button holds: its icon, and its name right under it, small, so a
/// place is found without trying each icon.
fn place_content(symbol: Icon, name: &'static str) -> impl Bundle {
    (
        Node {
            flex_direction: FlexDirection::Column,
            align_items: AlignItems::Center,
            ..default()
        },
        children![
            custom_icon(symbol, PLACE_ICON_PX, TEXT),
            (
                Text::new(name),
                font(PLACE_NAME_FONT_SIZE_PX),
                TextColor(TEXT),
            ),
        ],
    )
}
