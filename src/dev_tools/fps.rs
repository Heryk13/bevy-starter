use bevy::dev_tools::fps_overlay::{FpsOverlayConfig, FpsOverlayPlugin};
use bevy::prelude::*;

use crate::input::{Action, ActionName, Binding, Page, action_just_pressed, register_action};

// ==== Config =================================================================

// shows and hides the FPS overlay: the action, and the key it starts on
const FPS_OVERLAY: ActionName = ActionName("fps_overlay");
const FPS_OVERLAY_KEY: KeyCode = KeyCode::F2;

// ==== Plugin =================================================================

/// Bevy's FPS overlay, on from the start, shown and hidden by its action.
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
        toggle_fps_overlay.run_if(action_just_pressed(FPS_OVERLAY)),
    );
}

// ==== Systems ================================================================

/// Shows or hides the FPS overlay, its frame time graph along with its text.
fn toggle_fps_overlay(mut config: ResMut<FpsOverlayConfig>) {
    config.enabled = !config.enabled;
    config.frame_time_graph_config.enabled = config.enabled;
}
