use bevy::prelude::*;
use bevy::ui_widgets::Activate;

use crate::overlays::Overlay;
use crate::screens::Screen;
use crate::ui::button;

// ==== Config =================================================================

// below the overlays, so settings opened from the pause menu draws over it
const PAUSE_Z_INDEX: GlobalZIndex = GlobalZIndex(50);
const BACKDROP_COLOR: Color = Color::srgba(0.0, 0.0, 0.0, 0.5);

// ==== Plugin =================================================================

pub(super) fn plugin(app: &mut App) {
    app.add_sub_state::<Pause>();
    app.add_systems(OnEnter(Pause::On), (spawn_pause_menu, pause_time));
    app.add_systems(OnExit(Pause::On), unpause_time);
    app.add_observer(activate_pause_button);
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
    Settings,
    QuitToMenu,
    QuitToDesktop,
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
            (button("Settings"), PauseAction::Settings),
            (button("Quit to Menu"), PauseAction::QuitToMenu),
            (button("Quit to Desktop"), PauseAction::QuitToDesktop),
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
) {
    if let Ok(action) = actions.get(activate.entity) {
        match action {
            PauseAction::Resume => {
                next_pause.set(Pause::Off);
            }
            PauseAction::Settings => {
                next_overlay.set(Overlay::Settings);
            }
            PauseAction::QuitToMenu => {
                next_screen.set(Screen::Menu);
            }
            PauseAction::QuitToDesktop => {
                commands.write_message(AppExit::Success);
            }
        }
    }
}

/// Starts virtual time again. Also runs when gameplay is left while paused, so the menu
/// never inherits a frozen clock.
fn unpause_time(mut time: ResMut<Time<Virtual>>) {
    time.unpause();
}
