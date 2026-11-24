use bevy::prelude::*;
use std::path::{Path, PathBuf};

use super::{DEV_TOOLS_Z_INDEX, DevUi, dev_window};
use crate::input::{Action, ActionName, Binding, Page, action_just_pressed, register_action};
use crate::screens::Pause;
use crate::ui::{BORDER, BORDER_PX, ClosesOnEsc, Dismiss, floating_root};

mod actions;
mod dialog;
mod drawer;
mod list;
mod listing;
mod menu;
mod preview;
mod rename;
mod scroll;
mod title_bar;
mod trash;
mod tree;
mod usage;

use drawer::{DrawerHeight, ExplorerDrawer, slide_transform};
use listing::Place;

// the prop editor copies the files dropped on it, makes and deletes its folders and files, and
// names them, the way the explorer does
pub(super) use actions::{copy_entry, new_folder, trash_asset};
pub(super) use rename::is_name_allowed;
// and opens its menus where they fit, as the explorer's do
pub(super) use menu::menu_place;

// ==== Config =================================================================

// ---- action: opens and closes the explorer, and the key it starts on --------
const FILE_EXPLORER: ActionName = ActionName("file_explorer");
const FILE_EXPLORER_KEY: KeyCode = KeyCode::Numpad0;

// ---- words: English, as every dev tool's, never read in a release build -----
const FILE_EXPLORER_LABEL: &str = "File Explorer";

// ---- window: a drawer along the bottom, a title bar over its body -----------
const WINDOW_MARGIN_PX: f32 = 12.0;
// between the tree, the list and the preview
const BODY_GAP_PX: f32 = 16.0;

// ==== Plugin =================================================================

/// The file explorer on Numpad 0: a glass drawer that slides up over the bottom of the
/// screen, whatever is on it, the running game included, which goes on sharp above it. Esc
/// closes it before anything under it. Its parts are files of their own: the drawer, its
/// title bar, the tree, the list, the preview, the actions on files, the right-click menu,
/// renaming, Trash, where an asset is used, the questions asked, what scrolls, and
/// `listing.rs`, which reads the disk for them.
pub(super) fn plugin(app: &mut App) {
    register_action(
        app,
        Action {
            name: FILE_EXPLORER,
            default: Binding::Key(FILE_EXPLORER_KEY),
            page: Page::Dev,
            label: Some(FILE_EXPLORER_LABEL),
        },
    );
    app.add_plugins((
        drawer::plugin,
        title_bar::plugin,
        tree::plugin,
        list::plugin,
        preview::plugin,
        actions::plugin,
        menu::plugin,
        rename::plugin,
        trash::plugin,
        usage::plugin,
        dialog::plugin,
        scroll::plugin,
    ));
    app.add_systems(
        Update,
        toggle_file_explorer
            .run_if(action_just_pressed(FILE_EXPLORER))
            .run_if(not(in_state(Pause::Photo))),
    );
    // no run_if: the explorer opens over any screen
    app.add_observer(dismiss_file_explorer);
}

// ==== Components =============================================================

/// Marks the root of the open file explorer, with how far it has slid up into view, 0.0 to
/// 1.0, and whether it is sliding back down to close.
#[derive(Component, Default)]
struct FileExplorer {
    shown: f32,
    closing: bool,
}

/// Marks what an entry dragged from the list can be dropped on: a folder, by its place and
/// its path inside it. Folders in the list and the tree, the places, the parts of the path.
#[derive(Component, Clone)]
struct DropTarget {
    place: Place,
    folder: PathBuf,
}

// ==== Resources ==============================================================

/// What the explorer shows: a place, a folder inside it, and the entry selected there, by
/// its name in the list. Every part of the window redraws from it, so moving around is
/// changing it.
// a resource, not a component on the drawer as `CONTRIBUTING.md` would have it: the list,
// the tree, the path, the preview and Trash all read it, from systems and from observers on
// rows the window keeps respawning, and it is made new each time the drawer opens
#[derive(Resource)]
struct Explorer {
    place: Place,
    // empty for the place's own folder, and always for Recents, which has no folders
    folder: PathBuf,
    selected: Option<String>,
    // the entry whose name the list shows as a text field, by its name
    renaming: Option<String>,
    // counts the changes the explorer made to the disk, so what lists the folder lists it
    // again after one, though the folder shown is the same
    disk_changes: u32,
}

/// What tells one listing from another: the place, the folder, and how many times the disk
/// changed under it. The parts that redraw only when it changes keep the last one they drew.
type Shown = (Place, PathBuf, u32);

impl Explorer {
    /// Returns the explorer on `place`'s own folder, nothing selected.
    fn at(place: Place) -> Self {
        Self {
            place,
            folder: PathBuf::new(),
            selected: None,
            renaming: None,
            disk_changes: 0,
        }
    }

    /// Returns the folder shown, on the disk, or `None` when its place has no folder here.
    fn folder_on_disk(&self) -> Option<PathBuf> {
        self.place.root().map(|root| root.join(&self.folder))
    }

    /// Returns the place the entries shown are in on the disk: Recents' are the gallery's.
    fn disk_place(&self) -> Place {
        match self.place {
            Place::Recents => Place::Gallery,
            place => place,
        }
    }

    /// Returns the path inside `disk_place` of the entry named `name` in the list.
    fn inside(&self, name: &str) -> PathBuf {
        // part by part: a name in Recents is a path written with `/`, which Windows' explorer
        // would not read as a path if it were joined as it is
        let name: PathBuf = Path::new(name).components().collect();
        self.folder.join(name)
    }

    /// Returns the entry named `name` in the list, on the disk.
    fn on_disk(&self, name: &str) -> Option<PathBuf> {
        self.disk_place()
            .root()
            .map(|root| root.join(self.inside(name)))
    }

    /// Returns the selected entry, on the disk.
    fn selected_on_disk(&self) -> Option<PathBuf> {
        self.on_disk(self.selected.as_ref()?)
    }

    /// Returns what is shown, for the parts that redraw only when it changes.
    fn shown(&self) -> Shown {
        (self.place, self.folder.clone(), self.disk_changes)
    }
}

// ==== Systems ================================================================

/// Opens the file explorer on the gallery, sliding up, or slides it back down if it is open.
fn toggle_file_explorer(
    mut commands: Commands,
    mut open: Option<Single<&mut FileExplorer>>,
    height: Res<DrawerHeight>,
) {
    if let Some(explorer) = open.as_deref_mut() {
        explorer.closing = true;
        return;
    }
    commands.insert_resource(Explorer::at(Place::Gallery));
    commands.spawn((
        // on the dev tools' layer, and letting the pointer through around the drawer, so
        // the game above it stays sharp, with no backdrop: it is used while playing, to put
        // things into the game. The drawer itself takes the pointer, as every node does
        floating_root(DEV_TOOLS_Z_INDEX),
        Pickable::IGNORE,
        DevUi,
        ClosesOnEsc,
        FileExplorer::default(),
        slide_transform(0.0, height.0),
        children![explorer_window(height.0)],
    ));
}

/// Slides the explorer away when Esc dismisses it, as its close button does.
fn dismiss_file_explorer(dismiss: On<Dismiss>, mut explorers: Query<&mut FileExplorer>) {
    if let Ok(mut explorer) = explorers.get_mut(dismiss.entity) {
        explorer.closing = true;
    }
}

// ==== Helpers ================================================================

/// Returns the explorer's window: a drawer along the bottom of the screen, `height_percent`
/// of its height: its title bar, a line under it, and the tree, the list and the preview,
/// side by side.
fn explorer_window(height_percent: f32) -> impl Bundle {
    let margin = px(WINDOW_MARGIN_PX);
    (
        dev_window(Node {
            bottom: margin,
            left: margin,
            right: margin,
            height: percent(height_percent),
            ..default()
        }),
        ExplorerDrawer,
        children![
            title_bar::title_bar(),
            // a straight line between the title bar and what the drawer shows
            (
                Node {
                    height: px(BORDER_PX),
                    flex_shrink: 0.0,
                    ..default()
                },
                BackgroundColor(BORDER),
            ),
            (
                Node {
                    flex_grow: 1.0,
                    // so the body can be shorter than what it holds, and the list scroll
                    min_height: px(0),
                    column_gap: px(BODY_GAP_PX),
                    ..default()
                },
                children![tree::tree(), list::list(), preview::preview()],
            ),
        ],
    )
}
