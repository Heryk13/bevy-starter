use bevy::prelude::*;

use crate::save::Session;

mod gameplay;
mod intro;
mod loading;
mod menu;
mod splash;

pub(crate) use gameplay::pause::Pause;

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

// ==== Events =================================================================

/// Asks the current screen to leave for the given game, so whoever starts one never
/// learns how each screen leaves.
#[derive(Event)]
pub(crate) struct StartGame {
    pub(crate) session: Session,
    pub(crate) destination: LoadingDestination,
}

// ==== Resources ==============================================================

/// Where the loading screen goes once its work is done, set by whoever sends the
/// player there.
#[derive(Resource, Clone, Copy, PartialEq)]
pub(crate) enum LoadingDestination {
    Intro,
    Gameplay,
}
