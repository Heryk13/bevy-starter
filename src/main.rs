// hide the console window on Windows release builds
#![cfg_attr(not(feature = "dev"), windows_subsystem = "windows")]

use avian3d::prelude::PhysicsPlugins;
#[cfg(feature = "dev")]
use bevy::dev_tools::render_debug::RenderDebugOverlayPlugin;
use bevy::log::{DEFAULT_FILTER, LogPlugin};
use bevy::prelude::*;

mod assets;
#[cfg(feature = "dev")]
mod dev_tools;
mod disk;
mod fade;
mod i18n;
mod input;
mod overlays;
mod save;
mod screens;
mod ui;

// ==== Config =================================================================

// the game's name, on the window and the menu; a proper noun, so never translated
pub(crate) const GAME_TITLE: &str = "Bevy Starter";

// ==== App ====================================================================

fn main() {
    let mut app = App::new();

    // ---- engine -------------------------------------------------------------
    let default_plugins = DefaultPlugins
        .set(WindowPlugin {
            primary_window: Some(Window {
                title: GAME_TITLE.into(),
                ..default()
            }),
            ..default()
        })
        .set(LogPlugin {
            // parley leaves out ICU4X's Chinese/Japanese word dictionary on purpose, so
            // every Japanese text it lays out warns that the model is missing. Harmless:
            // line breaks come from separate rules, only word boundaries are lost. It reaches
            // `log` only through icu_provider's `logging` feature, set in Cargo.toml
            filter: format!("{DEFAULT_FILTER}icu_provider=error"),
            ..default()
        });
    // bevy/dev brings this render debug overlay, which needs the 3d renderer and panics on
    // its first frame in a game trimmed to 2d. Its keys are off by default, so nothing here
    // used it, and the FPS overlay keeps bevy/dev in the build
    #[cfg(feature = "dev")]
    let default_plugins = default_plugins.disable::<RenderDebugOverlayPlugin>();
    app.add_plugins(default_plugins);
    app.add_plugins(PhysicsPlugins::default());

    // ---- dev tools, left out of release builds ------------------------------
    #[cfg(feature = "dev")]
    app.add_plugins(dev_tools::plugin);

    // ---- game ---------------------------------------------------------------
    app.add_plugins(i18n::plugin);
    app.add_plugins(input::plugin);
    app.add_plugins(save::plugin);
    app.add_plugins(screens::plugin);
    app.add_plugins(ui::plugin);
    app.add_plugins(overlays::plugin);
    app.add_systems(Startup, spawn_screen_camera);

    app.run();
}

// ==== Systems ================================================================

/// Spawns the camera that renders the screens: the game and every UI root that does not
/// float. Its image is what gets blurred behind a floating panel.
fn spawn_screen_camera(mut commands: Commands) {
    commands.spawn((Camera2d, IsDefaultUiCamera));
}
