use bevy::prelude::*;

use crate::screens::Screen;

// ==== Plugin =================================================================

pub(super) fn plugin(app: &mut App) {
    app.add_sub_state::<Pause>();
    app.add_systems(OnEnter(Pause::On), pause_time);
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

/// Freezes virtual time, so everything that reads `Res<Time>` stops with the game.
fn pause_time(mut time: ResMut<Time<Virtual>>) {
    time.pause();
}

/// Starts virtual time again. Also runs when gameplay is left while paused, so the menu
/// never inherits a frozen clock.
fn unpause_time(mut time: ResMut<Time<Virtual>>) {
    time.unpause();
}
