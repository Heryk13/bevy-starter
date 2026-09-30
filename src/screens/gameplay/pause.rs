use bevy::prelude::*;
use bevy::ui_widgets::Activate;

use crate::fade::Fade;
use crate::overlays::Overlay;
use crate::save::{Session, has_unsaved_progress};
use crate::screens::Screen;
use crate::ui::{button, dialog};

// ==== Config =================================================================

// below the overlays, so settings opened from the pause menu draws over it
const PAUSE_Z_INDEX: GlobalZIndex = GlobalZIndex(50);
const BACKDROP_COLOR: Color = Color::srgba(0.0, 0.0, 0.0, 0.5);

const QUIT_QUESTION: &str = "Quit without saving?";

// no delay and no fade in, so the player learns how the save went at once; only the
// leaving is gentle
const STATUS_FADE: Fade = Fade {
    delay_secs: 0.0,
    fade_in_secs: 0.0,
    hold_secs: 2.0,
    fade_out_secs: 0.2,
};

// ==== Plugin =================================================================

pub(super) fn plugin(app: &mut App) {
    app.add_sub_state::<Pause>();
    app.add_systems(OnEnter(Pause::On), (spawn_pause_menu, pause_time));
    app.add_systems(OnExit(Pause::On), unpause_time);
    app.add_observer(activate_pause_button);
    app.add_observer(confirm_quit);
    app.add_systems(Update, fade_save_status.run_if(in_state(Pause::On)));
}

// ==== State ==================================================================

/// Whether the game is frozen. It only exists during gameplay, and starts `Off`.
#[derive(SubStates, Clone, PartialEq, Eq, Hash, Debug, Default)]
#[source(Screen = Screen::Gameplay)]
pub(crate) enum Pause {
    #[default]
    Off,
    On,
}

// ==== Components =============================================================

/// Names what a pause menu button does when it is pressed.
#[derive(Component)]
enum PauseAction {
    Resume,
    Save,
    Settings,
    QuitToMenu,
    QuitToDesktop,
}

/// Where a quit from the pause menu leaves for.
enum QuitTo {
    Menu,
    Desktop,
}

/// Marks the Yes of `QUIT_QUESTION`, with where to go once it is pressed.
#[derive(Component)]
struct ConfirmQuit(QuitTo);

/// Marks the pause menu's line of text that says how the last save went, with how long it
/// has been shown. Whoever writes the line sets `shown_secs` back to zero.
#[derive(Component)]
pub(crate) struct SaveStatus {
    pub(crate) shown_secs: f32,
}

// ==== Systems ================================================================

/// Spawns the pause menu over a backdrop that dims the frozen game and blocks clicks to it.
fn spawn_pause_menu(mut commands: Commands) {
    commands.spawn((
        Node {
            width: percent(100),
            height: percent(100),
            flex_direction: FlexDirection::Column,
            justify_content: JustifyContent::Center,
            align_items: AlignItems::Center,
            ..default()
        },
        children![
            Text::new("Paused"),
            (button("Resume"), PauseAction::Resume),
            (button("Save"), PauseAction::Save),
            (button("Settings"), PauseAction::Settings),
            (button("Quit to Menu"), PauseAction::QuitToMenu),
            (button("Quit to Desktop"), PauseAction::QuitToDesktop),
            (Text::new(""), SaveStatus { shown_secs: 0.0 }),
        ],
        BackgroundColor(BACKDROP_COLOR),
        PAUSE_Z_INDEX,
        DespawnOnExit(Pause::On),
    ));
}

/// Freezes virtual time, so everything that reads `Res<Time>` stops with the game.
fn pause_time(mut time: ResMut<Time<Virtual>>) {
    time.pause();
}

/// Runs the action of the pause menu button that was pressed.
fn activate_pause_button(
    activate: On<Activate>,
    mut commands: Commands,
    actions: Query<&PauseAction>,
    mut next_pause: ResMut<NextState<Pause>>,
    mut next_overlay: ResMut<NextState<Overlay>>,
    mut next_screen: ResMut<NextState<Screen>>,
    session: Res<Session>,
) {
    if let Ok(action) = actions.get(activate.entity) {
        match action {
            PauseAction::Resume => {
                next_pause.set(Pause::Off);
            }
            PauseAction::Save => {
                next_overlay.set(Overlay::SaveGame);
            }
            PauseAction::Settings => {
                next_overlay.set(Overlay::Settings);
            }
            PauseAction::QuitToMenu => {
                if has_unsaved_progress(&session) {
                    commands.spawn((
                        dialog(QUIT_QUESTION, ConfirmQuit(QuitTo::Menu)),
                        DespawnOnExit(Pause::On),
                    ));
                } else {
                    next_screen.set(Screen::Menu);
                }
            }
            PauseAction::QuitToDesktop => {
                if has_unsaved_progress(&session) {
                    commands.spawn((
                        dialog(QUIT_QUESTION, ConfirmQuit(QuitTo::Desktop)),
                        DespawnOnExit(Pause::On),
                    ));
                } else {
                    commands.write_message(AppExit::Success);
                }
            }
        }
    }
}

/// Quits for real once `QUIT_QUESTION` is answered Yes.
fn confirm_quit(
    activate: On<Activate>,
    mut commands: Commands,
    confirms: Query<&ConfirmQuit>,
    mut next_screen: ResMut<NextState<Screen>>,
) {
    if let Ok(confirm) = confirms.get(activate.entity) {
        match confirm {
            ConfirmQuit(QuitTo::Menu) => {
                next_screen.set(Screen::Menu);
            }
            ConfirmQuit(QuitTo::Desktop) => {
                commands.write_message(AppExit::Success);
            }
        }
    }
}

/// Fades the save status line out by `STATUS_FADE`. It counts real time, because virtual
/// time is frozen while paused and a line timed by it would never leave.
fn fade_save_status(time: Res<Time<Real>>, mut statuses: Query<(&mut SaveStatus, &mut TextColor)>) {
    for (mut status, mut color) in &mut statuses {
        status.shown_secs += time.delta_secs();
        if status.shown_secs > STATUS_FADE.total_secs() {
            status.shown_secs = STATUS_FADE.total_secs();
        }
        color.0.set_alpha(STATUS_FADE.alpha(status.shown_secs));
    }
}

/// Starts virtual time again. Also runs when gameplay is left while paused, so the menu
/// never inherits a frozen clock.
fn unpause_time(mut time: ResMut<Time<Virtual>>) {
    time.unpause();
}
