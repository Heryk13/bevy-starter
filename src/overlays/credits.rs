use bevy::prelude::*;

use super::CloseOverlay;
use super::OVERLAY_Z_INDEX;
use super::Overlay;
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
        Node {
            width: percent(100),
            height: percent(100),
            flex_direction: FlexDirection::Column,
            justify_content: JustifyContent::Center,
            align_items: AlignItems::Center,
            ..default()
        },
        children![Text::new(CREDITS_TEXT), (button("Close"), CloseOverlay),],
        OVERLAY_Z_INDEX,
        DespawnOnExit(Overlay::Credits),
    ));
}
