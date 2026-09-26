use bevy::prelude::*;

use crate::screens::Screen;

// ==== Config =================================================================

// below the overlays, so settings opened from the pause menu draws over it
const PAUSE_Z_INDEX: GlobalZIndex = GlobalZIndex(50);
const BACKDROP_COLOR: Color = Color::srgba(0.0, 0.0, 0.0, 0.5);

// ==== Plugin =================================================================

pub(super) fn plugin(app: &mut App) {
    app.add_sub_state::<Pause>();
    app.add_systems(OnEnter(Pause::On), (spawn_pause_menu, pause_time));
    app.add_systems(OnExit(Pause::On), unpause_time);
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
        children![Text::new("Paused")],
        BackgroundColor(BACKDROP_COLOR),
        PAUSE_Z_INDEX,
        DespawnOnExit(Pause::On),
    ));
}

/// Freezes virtual time, so everything that reads `Res<Time>` stops with the game.
fn pause_time(mut time: ResMut<Time<Virtual>>) {
    time.pause();
}

/// Starts virtual time again. Also runs when gameplay is left while paused, so the menu
/// never inherits a frozen clock.
fn unpause_time(mut time: ResMut<Time<Virtual>>) {
    time.unpause();
}
