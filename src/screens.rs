use bevy::prelude::*;

mod gameplay;
mod intro;
mod loading;
mod menu;
mod splash;

// ==== Plugin =================================================================

pub(super) fn plugin(app: &mut App) {
    app.init_state::<Screen>();
    app.add_plugins(splash::plugin);
    app.add_plugins(menu::plugin);
    app.add_plugins(loading::plugin);
    app.add_plugins(intro::plugin);
    app.add_plugins(gameplay::plugin);
}

// ==== State ==================================================================

/// Top-level screens of the game.
/// Each screen spawns its entities on enter and despawns them on exit.
#[derive(Debug, Clone, Copy, Default, Eq, PartialEq, Hash, States)]
pub(crate) enum Screen {
    #[default]
    Splash,
    Menu,
    Loading,
    Intro,
    Gameplay,
}
