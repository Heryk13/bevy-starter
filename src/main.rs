// hide the console window on Windows release builds
#![cfg_attr(not(feature = "dev"), windows_subsystem = "windows")]

#[cfg(feature = "dev")]
use bevy::dev_tools::fps_overlay::FpsOverlayPlugin;
use bevy::log::{DEFAULT_FILTER, LogPlugin};
use bevy::prelude::*;

use blur::BlurStrength;

mod assets;
mod blur;
mod fade;
mod i18n;
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
    app.add_plugins(
        DefaultPlugins
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
            }),
    );

    // ---- dev tools, left out of release builds ------------------------------
    #[cfg(feature = "dev")]
    app.add_plugins(FpsOverlayPlugin::default());

    // ---- game ---------------------------------------------------------------
    app.add_plugins(i18n::plugin);
    app.add_plugins(blur::plugin);
    app.add_plugins(screens::plugin);
    app.add_plugins(ui::plugin);
    app.add_plugins(overlays::plugin);
    app.add_systems(Startup, spawn_camera);

    app.run();
}

// ==== Systems ================================================================

/// Spawns the camera that renders the screens: the game and every UI root that does not
/// float. Its image is what gets blurred behind a floating panel.
fn spawn_camera(mut commands: Commands) {
    commands.spawn((Camera2d, IsDefaultUiCamera, BlurStrength::default()));
}
