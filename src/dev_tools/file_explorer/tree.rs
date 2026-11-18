use bevy::picking::hover::Hovered;
use bevy::prelude::*;
use bevy::ui_widgets::ScrollArea;
use std::collections::HashSet;
use std::path::{Path, PathBuf};

use super::listing::{Place, kind_of, read_folder};
use super::scroll::{ExplorerScroll, scroll_frame, scrolling_column};
use super::{DropTarget, Explorer, Shown};
use crate::ui::{Icon, SMALL_FONT_SIZE_PX, TEXT, TEXT_MUTED, custom_icon, font, muted_text};

// ==== Config =================================================================

// ---- words: English, as every dev tool's, never read in a release build -----
const LOCATIONS_HEADING: &str = "Locations";

// ---- the tree: the place's folders and files, as a code editor shows them ---
const TREE_WIDTH_PX: f32 = 240.0;
const TREE_ROW_HEIGHT_PX: f32 = 24.0;
// how far each level sits in from the one holding it, a guide line in its middle
const TREE_INDENT_PX: f32 = 14.0;
// the arrow a folder opens and closes by, and the room a file leaves for one
const TREE_ARROW_PX: f32 = 16.0;
const TREE_ICON_PX: f32 = 16.0;
// between the arrow, the icon and the name
const TREE_GAP_PX: f32 = 4.0;
// a row has no fill of its own: a faint one under the pointer, a brighter one for what is
// shown, both from edge to edge, with no corner
const TREE_HOVER_FILL: Color = Color::srgba(1.0, 1.0, 1.0, 0.06);
const TREE_SHOWN_FILL: Color = Color::srgba(1.0, 1.0, 1.0, 0.14);
const TREE_GUIDE: Color = Color::srgba(1.0, 1.0, 1.0, 0.12);

// ==== Plugin =================================================================

/// The explorer's tree, on its left: Gallery's or Assets' folders and files as a code editor
/// lists them, which is not there at all for a place with none. A press on a folder opens and
/// shows it; a press on a file shows its folder with it selected.
pub(super) fn plugin(app: &mut App) {
    app.init_resource::<OpenFolders>();
    app.add_systems(
        Update,
        (fill_tree, paint_tree_rows).run_if(resource_exists::<Explorer>),
    );
}

// ==== Widgets ================================================================

/// Returns the sidebar, which `fill_tree` fills, scrolling when the tree is taller than the
/// window shows.
pub(super) fn tree() -> impl Bundle {
    (
        Node {
            width: px(TREE_WIDTH_PX),
            flex_shrink: 0.0,
            // beside the list, in a row: its width is fixed, the list takes the rest
            flex_grow: 0.0,
            ..scroll_frame()
        },
        TreeFrame,
        children![(
            Node {
                // rows touch, so the guide lines run on from one to the next
                row_gap: px(0),
                ..scrolling_column()
            },
            ScrollArea,
            TreeRows,
            ExplorerScroll,
        )],
    )
}

// ==== Components =============================================================

/// Marks the node the tree's rows are spawned in.
#[derive(Component)]
struct TreeRows;

/// Marks the frame around the tree, which is hidden with it.
#[derive(Component)]
struct TreeFrame;

/// A row of the tree: the folder or file it shows, as a path inside its place.
#[derive(Component)]
struct TreeRow {
    path: PathBuf,
    is_folder: bool,
}

// ==== Resources ==============================================================

/// The folders opened in the tree, each with its place, kept while the game runs, so the tree
/// opens as it was left. The folder shown, and every one holding it, is open as well.
#[derive(Resource, Default)]
struct OpenFolders(HashSet<(Place, PathBuf)>);

// ==== Systems ================================================================

/// Lists the place's folders and files as a tree, each open folder's under it, when the
/// window opens, when what is shown changes, and when a folder opens or closes.
/// A place with nothing in it, Recents and Trash always, hides the sidebar, so the list
/// takes its room instead of leaving it empty.
fn fill_tree(
    mut commands: Commands,
    explorer: Res<Explorer>,
    open_folders: Res<OpenFolders>,
    trees: Query<(Entity, Ref<TreeRows>)>,
    mut frames: Query<&mut Node, With<TreeFrame>>,
    mut shown: Local<Option<Shown>>,
) {
    let (Ok((tree, marker)), Ok(mut frame)) = (trees.single(), frames.single_mut()) else {
        *shown = None;
        return;
    };
    // not on a selection alone, which only moves the highlight `paint_tree_rows` paints: each
    // rebuild reads every open folder from the disk again
    let same_folder = shown.as_ref() == Some(&explorer.shown());
    if !marker.is_added() && same_folder && !open_folders.is_changed() {
        return;
    }
    *shown = Some(explorer.shown());
    commands.entity(tree).despawn_children();
    let root = match explorer.place {
        Place::Gallery | Place::Assets => explorer.place.root(),
        Place::Recents | Place::Trash => None,
    };
    let rows = root
        .map(|root| tree_rows(&root, explorer.place, &explorer.folder, &open_folders.0))
        .unwrap_or_default();
    frame.display = if rows.is_empty() {
        Display::None
    } else {
        Display::Flex
    };
    if rows.is_empty() {
        return;
    }
    commands.spawn((
        Text::new(LOCATIONS_HEADING),
        muted_text(SMALL_FONT_SIZE_PX),
        ChildOf(tree),
    ));
    for row in rows {
        spawn_tree_row(&mut commands, tree, explorer.place, row);
    }
}

/// Fills each row of the tree: brighter for what is shown, the folder in the list or the
/// file selected in it, fainter under the pointer, and nothing the rest of the time.
fn paint_tree_rows(
    explorer: Res<Explorer>,
    mut rows: Query<(&TreeRow, &Hovered, &mut BackgroundColor)>,
) {
    for (row, hovered, mut fill) in &mut rows {
        let is_shown = if row.is_folder {
            explorer.folder == row.path
        } else {
            row.path.parent() == Some(explorer.folder.as_path())
                && explorer.selected.as_deref()
                    == row.path.file_name().and_then(|name| name.to_str())
        };
        let wanted = if is_shown {
            TREE_SHOWN_FILL
        } else if hovered.get() {
            TREE_HOVER_FILL
        } else {
            Color::NONE
        };
        if fill.0 != wanted {
            fill.0 = wanted;
        }
    }
}

// ==== Helpers ================================================================

/// One row of the tree, as `tree_rows` lays them out: what it shows, how deep it sits, and
/// for a folder whether it is open.
struct LaidRow {
    path: PathBuf,
    name: String,
    is_folder: bool,
    is_open: bool,
    depth: usize,
    icon: Icon,
}

/// Returns the rows of the tree under `root`, top to bottom: each entry of a folder, folders
/// first, and under each open folder its own entries one level deeper. A folder is open when
/// `open_folders` holds it, or when it is `shown` or holds it, so the tree always reaches the
/// folder in the list.
fn tree_rows(
    root: &Path,
    place: Place,
    shown: &Path,
    open_folders: &HashSet<(Place, PathBuf)>,
) -> Vec<LaidRow> {
    let mut rows = Vec::new();
    // depth first, by a stack of (folder inside the place, depth), each folder's entries
    // pushed in reverse so they come off in order
    let mut pending = vec![(PathBuf::new(), 0)];
    while let Some((folder, depth)) = pending.pop() {
        let mut level = Vec::new();
        for entry in read_folder(&root.join(&folder)) {
            let path = folder.join(&entry.name);
            let is_open = entry.is_folder
                && (open_folders.contains(&(place, path.clone())) || shown.starts_with(&path));
            level.push((
                LaidRow {
                    icon: kind_of(&entry).icon(),
                    path,
                    name: entry.name,
                    is_folder: entry.is_folder,
                    is_open,
                    depth,
                },
                is_open,
            ));
        }
        // a level's rows go in now, and an open folder's own come right after its row: so
        // each level is put in front of the rows that followed the folder it opened from
        let insert_at = rows
            .iter()
            .position(|row: &LaidRow| row.path == folder)
            .map_or(rows.len(), |index| index + 1);
        let mut opened = Vec::new();
        for (offset, (row, is_open)) in level.into_iter().enumerate() {
            if is_open {
                opened.push((row.path.clone(), depth + 1));
            }
            rows.insert(insert_at + offset, row);
        }
        pending.extend(opened.into_iter().rev());
    }
    rows
}

/// Spawns one row of the tree: a guide line for each level it sits in, an arrow for a
/// folder, its icon and its name. A press on a folder opens or closes it and shows it; a
/// press on a file shows its folder with it selected.
fn spawn_tree_row(commands: &mut Commands, tree: Entity, place: Place, row: LaidRow) {
    let arrow = match (row.is_folder, row.is_open) {
        (true, true) => Some(Icon::ChevronDown),
        (true, false) => Some(Icon::ChevronRight),
        (false, _) => None,
    };
    let line = commands
        .spawn((
            Node {
                height: px(TREE_ROW_HEIGHT_PX),
                flex_shrink: 0.0,
                align_items: AlignItems::Center,
                column_gap: px(TREE_GAP_PX),
                ..default()
            },
            // no `Button`: the buttons' sounds would tick on every row the pointer crosses
            Hovered::default(),
            BackgroundColor(Color::NONE),
            TreeRow {
                path: row.path.clone(),
                is_folder: row.is_folder,
            },
            ChildOf(tree),
        ))
        .id();
    if row.is_folder {
        commands.entity(line).insert(DropTarget {
            place,
            folder: row.path.clone(),
        });
    }
    for _ in 0..row.depth {
        commands.spawn((guide(), ChildOf(line)));
    }
    let arrow_cell = commands
        .spawn((
            Node {
                width: px(TREE_ARROW_PX),
                flex_shrink: 0.0,
                justify_content: JustifyContent::Center,
                ..default()
            },
            Pickable::IGNORE,
            ChildOf(line),
        ))
        .id();
    if let Some(arrow) = arrow {
        commands.spawn((
            custom_icon(arrow, TREE_ARROW_PX, TEXT_MUTED),
            Pickable::IGNORE,
            ChildOf(arrow_cell),
        ));
    }
    commands.spawn((
        custom_icon(row.icon, TREE_ICON_PX, TEXT),
        Pickable::IGNORE,
        ChildOf(line),
    ));
    commands.spawn((
        Text::new(row.name),
        font(SMALL_FONT_SIZE_PX),
        TextColor(TEXT),
        Pickable::IGNORE,
        ChildOf(line),
    ));
    let path = row.path;
    let is_folder = row.is_folder;
    commands.entity(line).observe(
        move |click: On<PointerClick>,
              mut explorer: ResMut<Explorer>,
              mut open: ResMut<OpenFolders>| {
            if click.button != PointerButton::Primary {
                return;
            }
            if is_folder {
                let key = (place, path.clone());
                // the folder shown closes on a press, and its parent is shown, since the
                // folder shown is always open; any other opens and is shown
                if explorer.folder == path {
                    open.0.remove(&key);
                    explorer.folder = path.parent().map(Path::to_path_buf).unwrap_or_default();
                } else {
                    open.0.insert(key);
                    explorer.folder = path.clone();
                }
                explorer.selected = None;
            } else {
                explorer.folder = path.parent().map(Path::to_path_buf).unwrap_or_default();
                explorer.selected = path
                    .file_name()
                    .map(|name| name.to_string_lossy().into_owned());
            }
        },
    );
}

/// Returns one level's guide: a cell as wide as the indent, a thin upright line in its
/// middle, which runs on into the rows above and below of the same level.
fn guide() -> impl Bundle {
    (
        Node {
            width: px(TREE_INDENT_PX),
            height: percent(100),
            flex_shrink: 0.0,
            justify_content: JustifyContent::Center,
            ..default()
        },
        Pickable::IGNORE,
        children![(
            Node {
                width: px(1),
                height: percent(100),
                ..default()
            },
            BackgroundColor(TREE_GUIDE),
            Pickable::IGNORE,
        )],
    )
}

// ==== Tests ==================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::disk::fresh_test_dir;

    /// Returns the rows' paths and depths, as the tree lays them out from top to bottom.
    fn laid_out(rows: &[LaidRow]) -> Vec<(String, usize)> {
        rows.iter()
            .map(|row| (row.path.to_string_lossy().replace('\\', "/"), row.depth))
            .collect()
    }

    #[test]
    fn a_closed_folder_lists_nothing_under_it() {
        let dir = fresh_test_dir("bevy-starter-explorer-tree-closed");
        std::fs::create_dir_all(dir.join("menu")).unwrap();
        std::fs::write(dir.join("menu").join("music.ogg"), [0_u8]).unwrap();
        std::fs::write(dir.join("logo.png"), [0_u8]).unwrap();
        let rows = tree_rows(&dir, Place::Assets, Path::new(""), &HashSet::new());
        assert_eq!(
            laid_out(&rows),
            [("menu".into(), 0), ("logo.png".into(), 0)]
        );
    }

    #[test]
    fn an_open_folder_lists_its_entries_one_level_deeper_right_under_it() {
        let dir = fresh_test_dir("bevy-starter-explorer-tree-open");
        std::fs::create_dir_all(dir.join("menu")).unwrap();
        std::fs::create_dir_all(dir.join("ui")).unwrap();
        std::fs::write(dir.join("menu").join("music.ogg"), [0_u8]).unwrap();
        let open = HashSet::from([(Place::Assets, PathBuf::from("menu"))]);
        let rows = tree_rows(&dir, Place::Assets, Path::new(""), &open);
        assert_eq!(
            laid_out(&rows),
            [
                ("menu".into(), 0),
                ("menu/music.ogg".into(), 1),
                ("ui".into(), 0)
            ]
        );
    }

    #[test]
    fn the_folder_shown_is_open_with_every_folder_holding_it() {
        let dir = fresh_test_dir("bevy-starter-explorer-tree-shown");
        std::fs::create_dir_all(dir.join("intro").join("frames")).unwrap();
        std::fs::write(dir.join("intro").join("frames").join("000.png"), [0_u8]).unwrap();
        let rows = tree_rows(
            &dir,
            Place::Assets,
            Path::new("intro/frames"),
            &HashSet::new(),
        );
        assert_eq!(
            laid_out(&rows),
            [
                ("intro".into(), 0),
                ("intro/frames".into(), 1),
                ("intro/frames/000.png".into(), 2)
            ]
        );
    }
}
