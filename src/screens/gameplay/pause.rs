use bevy::prelude::*;

use crate::screens::Screen;

// ==== Plugin =================================================================

pub(super) fn plugin(app: &mut App) {
    app.add_sub_state::<Pause>();
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
