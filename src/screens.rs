use bevy::prelude::*;

mod splash;

// ==== Plugin =================================================================

pub(super) fn plugin(app: &mut App) {
    app.init_state::<Screen>();
    app.add_plugins(splash::plugin);
}

// ==== State ==================================================================

/// Top-level screens of the game.
/// Each screen spawns its entities on enter and despawns them on exit.
#[derive(Debug, Clone, Copy, Default, Eq, PartialEq, Hash, States)]
enum Screen {
    #[default]
    Splash,
    Menu,
}
