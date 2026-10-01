// hide the console window on Windows release builds
#![cfg_attr(not(feature = "dev"), windows_subsystem = "windows")]

#[cfg(feature = "dev")]
use bevy::dev_tools::fps_overlay::FpsOverlayPlugin;
use bevy::log::{DEFAULT_FILTER, LogPlugin};
use bevy::prelude::*;

mod assets;
mod fade;
mod i18n;
mod overlays;
mod save;
mod screens;
mod ui;

// ==== App ====================================================================

fn main() {
    let mut app = App::new();

    // ---- engine -------------------------------------------------------------
    app.add_plugins(
        DefaultPlugins
            .set(WindowPlugin {
                primary_window: Some(Window {
                    title: "Bevy Starter".into(),
                    ..default()
                }),
                ..default()
            })
            .set(LogPlugin {
                // parley leaves out ICU4X's Chinese/Japanese word dictionary on purpose, so
                // every Japanese text it lays out warns that the model is missing. Harmless:
                // line breaks come from separate rules, only word boundaries are lost
                filter: format!("{DEFAULT_FILTER}icu_provider=error"),
                ..default()
            }),
    );

    // ---- dev tools, left out of release builds ------------------------------
    #[cfg(feature = "dev")]
    app.add_plugins(FpsOverlayPlugin::default());

    // ---- game ---------------------------------------------------------------
    app.add_plugins(i18n::plugin);
    app.add_plugins(screens::plugin);
    app.add_plugins(ui::plugin);
    app.add_plugins(overlays::plugin);
    app.add_systems(Startup, spawn_camera);

    app.run();
}

// ==== Systems ================================================================

/// Spawns the camera that renders the game and the UI.
fn spawn_camera(mut commands: Commands) {
    commands.spawn(Camera2d);
}
