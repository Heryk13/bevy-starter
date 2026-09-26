use bevy::prelude::*;

use super::CloseOverlay;
use super::Overlay;
use super::overlay_root;
use crate::ui::button;

// ==== Config =================================================================

const CREDITS_TEXT: &str = "Made with Bevy\nHeryk13";

// ==== Plugin =================================================================

pub(super) fn plugin(app: &mut App) {
    app.add_systems(OnEnter(Overlay::Credits), spawn_credits_overlay);
}

// ==== Systems ================================================================

/// Spawns the credits panel over the current screen, blocking clicks to it.
fn spawn_credits_overlay(mut commands: Commands) {
    commands.spawn((
        overlay_root(),
        children![Text::new(CREDITS_TEXT), (button("Close"), CloseOverlay),],
        DespawnOnExit(Overlay::Credits),
    ));
}
