use bevy::prelude::*;

use super::CloseOverlay;
use super::Overlay;
use super::overlay_root;
use crate::i18n::Localized;
use crate::ui::button;

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
        children![
            Localized("credits-made-with"),
            Text::new(CREDITS_NAMES),
            (button("overlay-close"), CloseOverlay),
        ],
        DespawnOnExit(Overlay::Credits),
    ));
}
