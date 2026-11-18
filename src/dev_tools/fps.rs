use bevy::dev_tools::fps_overlay::{FpsOverlayConfig, FpsOverlayPlugin};
use bevy::prelude::*;

use crate::input::{Action, ActionName, Binding, Page, action_just_pressed, register_action};
use crate::screens::Pause;

// ==== Config =================================================================

// shows and hides the FPS overlay: the action, and the key it starts on
const FPS_OVERLAY: ActionName = ActionName("fps_overlay");
const FPS_OVERLAY_KEY: KeyCode = KeyCode::F2;

// ==== Plugin =================================================================

/// Bevy's FPS overlay, on from the start, shown and hidden by its action, and hidden while
/// photo mode takes the game alone.
pub(super) fn plugin(app: &mut App) {
    app.add_plugins(FpsOverlayPlugin::default());
    register_action(
        app,
        Action {
            name: FPS_OVERLAY,
            default: Binding::Key(FPS_OVERLAY_KEY),
            page: Page::Dev,
            label: Some("FPS Overlay"),
        },
    );
    app.add_systems(
        Update,
        toggle_fps_overlay
            .run_if(action_just_pressed(FPS_OVERLAY))
            .run_if(not(in_state(Pause::Photo))),
    );
    app.add_systems(OnEnter(Pause::Photo), hide_fps_overlay_for_photo);
    app.add_systems(OnExit(Pause::Photo), restore_fps_overlay_after_photo);
}

// ==== Resources ==============================================================

/// Whether the FPS overlay showed as photo mode began, so it comes back as it was: one
/// turned off before stays off.
#[derive(Resource)]
struct FpsShownBeforePhoto(bool);

// ==== Systems ================================================================

/// Shows or hides the FPS overlay.
fn toggle_fps_overlay(mut config: ResMut<FpsOverlayConfig>) {
    let shown = !config.enabled;
    show_fps_overlay(&mut config, shown);
}

/// Hides the FPS overlay for photo mode, remembering whether it showed.
fn hide_fps_overlay_for_photo(mut commands: Commands, mut config: ResMut<FpsOverlayConfig>) {
    commands.insert_resource(FpsShownBeforePhoto(config.enabled));
    show_fps_overlay(&mut config, false);
}

/// Brings the FPS overlay back as it was before photo mode.
fn restore_fps_overlay_after_photo(
    shown_before: Option<Res<FpsShownBeforePhoto>>,
    mut config: ResMut<FpsOverlayConfig>,
) {
    if let Some(shown_before) = shown_before {
        show_fps_overlay(&mut config, shown_before.0);
    }
}

// ==== Helpers ================================================================

/// Shows or hides the FPS overlay, its frame time graph along with its text.
fn show_fps_overlay(config: &mut FpsOverlayConfig, shown: bool) {
    config.enabled = shown;
    config.frame_time_graph_config.enabled = shown;
}
