use bevy::prelude::*;

use super::Overlay;
use super::{close_footer, overlay_card, overlay_root};
use crate::ui::{BODY_FONT_SIZE_PX, TEXT_MUTED, font, heading, label};

// ==== Config =================================================================

// names are never translated
const CREDITS_NAMES: &str = "Heryk13";

// ==== Plugin =================================================================

pub(super) fn plugin(app: &mut App) {
    app.add_systems(OnEnter(Overlay::Credits), spawn_credits_overlay);
}

// ==== Systems ================================================================

/// Spawns the credits panel over the current screen, blocking clicks to it.
fn spawn_credits_overlay(mut commands: Commands) {
    commands.spawn((
        overlay_root(),
        children![(
            overlay_card(),
            children![
                heading("credits-title"),
                label("credits-made-with"),
                (
                    Text::new(CREDITS_NAMES),
                    font(BODY_FONT_SIZE_PX),
                    TextColor(TEXT_MUTED),
                ),
                close_footer(),
            ],
        )],
        DespawnOnExit(Overlay::Credits),
    ));
}
