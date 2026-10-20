// hide the console window on Windows release builds
#![cfg_attr(not(feature = "dev"), windows_subsystem = "windows")]

#[cfg(feature = "dev")]
use bevy::dev_tools::fps_overlay::{FpsOverlayConfig, FpsOverlayPlugin};
#[cfg(feature = "dev")]
use bevy::dev_tools::render_debug::RenderDebugOverlayPlugin;
#[cfg(feature = "dev")]
use bevy::input::common_conditions::input_just_pressed;
use bevy::log::{DEFAULT_FILTER, LogPlugin};
use bevy::prelude::*;

use ui::blur::BlurStrength;

mod assets;
mod disk;
mod fade;
mod i18n;
mod overlays;
mod save;
mod screens;
mod ui;

// ==== Config =================================================================

// the game's name, on the window and the menu; a proper noun, so never translated
pub(crate) const GAME_TITLE: &str = "Bevy Starter";
// shows and hides the FPS overlay in dev builds
#[cfg(feature = "dev")]
const FPS_TOGGLE_KEY: KeyCode = KeyCode::F3;

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

    // ---- dev tools, left out of release builds ------------------------------
    #[cfg(feature = "dev")]
    app.add_plugins(FpsOverlayPlugin::default());
    #[cfg(feature = "dev")]
    app.add_systems(
        Update,
        toggle_fps_overlay.run_if(input_just_pressed(FPS_TOGGLE_KEY)),
    );

    // ---- game ---------------------------------------------------------------
    app.add_plugins(i18n::plugin);
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
    commands.spawn((Camera2d, IsDefaultUiCamera, BlurStrength::default()));
}

/// Shows or hides the FPS overlay, its frame time graph along with its text.
#[cfg(feature = "dev")]
fn toggle_fps_overlay(mut config: ResMut<FpsOverlayConfig>) {
    config.enabled = !config.enabled;
    config.frame_time_graph_config.enabled = config.enabled;
}
