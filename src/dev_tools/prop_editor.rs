use bevy::prelude::*;

use super::{DEV_TOOLS_Z_INDEX, DevUi, dev_window};
use crate::input::{Action, ActionName, Binding, Page, action_just_pressed, register_action};
use crate::screens::Pause;
use crate::ui::{ClosesOnEsc, Dismiss, floating_root};

// its thumbnails are made by each genre
pub(crate) mod library;
mod window;

// ==== Config =================================================================

// opens and closes the editor: the action, and the key it starts on
const PROP_EDITOR: ActionName = ActionName("prop_editor");
const PROP_EDITOR_KEY: KeyCode = KeyCode::F10;

// ---- words: English, as every dev tool's, never read in a release build -----
const PROP_EDITOR_LABEL: &str = "Prop Editor";

// ---- window: a drawer along the bottom, as Unreal's content drawer ----------
const WINDOW_MARGIN_PX: f32 = 12.0;
const WINDOW_HEIGHT_PX: f32 = 280.0;

// ==== Plugin =================================================================

/// The prop editor on F10: a drawer of props along the bottom of the screen while the game
/// keeps running, each prop a file in `assets/props/`, placed into the map file that
/// `screens/gameplay/map.rs` reads. It knows no genre: a genre inserts its `PropKind`, shows
/// the props and moves the camera. Esc closes it, and so does anything that leaves
/// `Pause::Off`, since its state lives inside it. Its parts are files of their own: the
/// library of props on disk, and the window that lists them.
pub(super) fn plugin(app: &mut App) {
    register_action(
        app,
        Action {
            name: PROP_EDITOR,
            default: Binding::Key(PROP_EDITOR_KEY),
            page: Page::Dev,
            label: Some(PROP_EDITOR_LABEL),
        },
    );
    app.add_sub_state::<PropEditor>();
    app.add_plugins((library::plugin, window::plugin));
    app.add_systems(
        Update,
        toggle_prop_editor.run_if(action_just_pressed(PROP_EDITOR)),
    );
    app.add_systems(OnEnter(PropEditor::On), spawn_props_window);
    app.add_observer(dismiss_prop_editor.run_if(in_state(PropEditor::On)));
}

// ==== Prop Kind ==============================================================

/// What a genre's props are: the extensions of the files it takes. The genre inserts it; with
/// none, as in an Empty game, the window lists nothing.
#[derive(Resource, Clone, Copy)]
pub(crate) struct PropKind {
    pub(crate) extensions: &'static [&'static str],
}

// ==== State ==================================================================

/// Whether the prop editor is open. It lives inside `Pause::Off`, beside the game running
/// rather than freezing it, so the pause menu, photo mode and leaving gameplay each close it
/// with no code of its own; whatever it spawns carries `DespawnOnExit(PropEditor::On)`.
#[derive(SubStates, Clone, Copy, PartialEq, Eq, Hash, Debug, Default)]
#[source(Pause = Pause::Off)]
pub(crate) enum PropEditor {
    #[default]
    Off,
    On,
}

// ==== Components =============================================================

/// Marks the root of the props window, which Esc dismisses.
#[derive(Component)]
struct PropsWindow;

// ==== Systems ================================================================

/// Opens the editor from the running game, or closes it. Does nothing outside `Pause::Off`,
/// where the state does not exist.
fn toggle_prop_editor(
    editor: Option<Res<State<PropEditor>>>,
    mut next_editor: ResMut<NextState<PropEditor>>,
) {
    let Some(editor) = editor else {
        return;
    };
    next_editor.set(match editor.get() {
        PropEditor::Off => PropEditor::On,
        PropEditor::On => PropEditor::Off,
    });
}

/// Spawns the props window: a drawer along the bottom, on the dev tools' layer, letting the
/// pointer through around it so the game above takes clicks. It is a modal for Esc, so a
/// genre whose camera holds the cursor gives it back while the editor is open.
fn spawn_props_window(mut commands: Commands) {
    let margin = px(WINDOW_MARGIN_PX);
    commands.spawn((
        floating_root(DEV_TOOLS_Z_INDEX),
        Pickable::IGNORE,
        DevUi,
        ClosesOnEsc,
        PropsWindow,
        DespawnOnExit(PropEditor::On),
        children![(
            dev_window(Node {
                bottom: margin,
                left: margin,
                right: margin,
                height: px(WINDOW_HEIGHT_PX),
                ..default()
            }),
            children![window::window_body()],
        )],
    ));
}

/// Closes the editor when Esc dismisses its window.
fn dismiss_prop_editor(
    dismiss: On<Dismiss>,
    windows: Query<(), With<PropsWindow>>,
    mut next_editor: ResMut<NextState<PropEditor>>,
) {
    if windows.contains(dismiss.entity) {
        next_editor.set(PropEditor::Off);
    }
}
