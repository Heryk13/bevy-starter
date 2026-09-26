use bevy::prelude::*;

use crate::fade::Fade;
use crate::screens::Screen;

// ==== Config =================================================================

// ---- veil: black cover that fades the menu in and out -----------------------
const VEIL_FADE_SECS: f32 = 0.8;
pub(super) const VEIL_Z_INDEX: GlobalZIndex = GlobalZIndex(1);

// ---- veil fades: the closing one holds black until the screen changes -------
const VEIL_OPENING: Fade = Fade {
    delay_secs: 0.0,
    fade_in_secs: 0.0,
    hold_secs: 0.0,
    fade_out_secs: VEIL_FADE_SECS,
};
const VEIL_CLOSING: Fade = Fade {
    delay_secs: 0.0,
    fade_in_secs: VEIL_FADE_SECS,
    hold_secs: f32::INFINITY,
    fade_out_secs: 0.0,
};

// ==== Components =============================================================

/// A black cover over the menu: it fades out as the menu opens and in as it leaves.
#[derive(Component)]
pub(super) struct Veil {
    pub(super) timer: Timer,
    pub(super) transition: MenuTransition,
}

/// Whether a veil is opening the menu or closing it, which decides its fade.
#[derive(Clone, Copy)]
pub(super) enum MenuTransition {
    Opening,
    Closing,
}

// ==== Systems ================================================================

/// Fades every veil along its timer, then removes it or leaves the menu when it ends.
pub(super) fn animate_veils(
    mut veils: Query<(Entity, &mut Veil, &mut BackgroundColor)>,
    time: Res<Time>,
    mut commands: Commands,
    mut next_screen: ResMut<NextState<Screen>>,
) {
    for (entity, mut veil, mut background_color) in &mut veils {
        veil.timer.tick(time.delta());
        let alpha = veil_alpha(veil.timer.elapsed_secs(), veil.transition);
        background_color.0.set_alpha(alpha);

        if veil.timer.is_finished() {
            match screen_after_veil(veil.transition) {
                Some(screen) => next_screen.set(screen),
                None => commands.entity(entity).despawn(),
            }
        }
    }
}

// ==== Helpers ================================================================

/// Returns a full-screen veil for the given transition, ready to spawn.
pub(super) fn veil(transition: MenuTransition) -> impl Bundle {
    (
        Node {
            width: percent(100),
            height: percent(100),
            position_type: PositionType::Absolute,
            ..default()
        },
        BackgroundColor(Color::BLACK.with_alpha(veil_alpha(0.0, transition))),
        Veil {
            timer: Timer::from_seconds(VEIL_FADE_SECS, TimerMode::Once),
            transition,
        },
    )
}

/// Returns the veil alpha at the given time, from the opening or the closing fade.
pub(super) fn veil_alpha(elapsed_secs: f32, transition: MenuTransition) -> f32 {
    match transition {
        MenuTransition::Opening => VEIL_OPENING.alpha(elapsed_secs),
        MenuTransition::Closing => VEIL_CLOSING.alpha(elapsed_secs),
    }
}

/// Returns the screen to leave for once a veil has finished, or `None` when the menu
/// stays and the veil is simply removed.
fn screen_after_veil(transition: MenuTransition) -> Option<Screen> {
    match transition {
        MenuTransition::Opening => None,
        MenuTransition::Closing => Some(Screen::Loading),
    }
}

// ==== Tests ==================================================================

#[cfg(test)]
mod tests {
    use super::*;

    // the ends are compared exactly: `sample_clamped` returns exactly 0.0 and 1.0 there,
    // so only the points in between need the tolerance
    const TOLERANCE: f32 = 1e-5;

    #[test]
    fn veil_is_black_when_menu_starts_opening() {
        assert_eq!(veil_alpha(0.0, MenuTransition::Opening), 1.0);
    }

    #[test]
    fn veil_is_half_dark_while_menu_opens() {
        let alpha = veil_alpha(VEIL_FADE_SECS / 2.0, MenuTransition::Opening);
        assert!(
            (alpha - 0.5).abs() < TOLERANCE,
            "{alpha} should be approximately 0.5"
        );
    }

    #[test]
    fn veil_is_transparent_when_menu_finishes_opening() {
        assert_eq!(veil_alpha(VEIL_FADE_SECS, MenuTransition::Opening), 0.0);
    }

    #[test]
    fn veil_is_transparent_when_menu_starts_closing() {
        assert_eq!(veil_alpha(0.0, MenuTransition::Closing), 0.0);
    }

    #[test]
    fn veil_is_half_dark_while_menu_closes() {
        let alpha = veil_alpha(VEIL_FADE_SECS / 2.0, MenuTransition::Closing);
        assert!(
            (alpha - 0.5).abs() < TOLERANCE,
            "{alpha} should be approximately 0.5"
        );
    }

    #[test]
    fn veil_is_black_when_menu_finishes_closing() {
        assert_eq!(veil_alpha(VEIL_FADE_SECS, MenuTransition::Closing), 1.0);
    }

    #[test]
    fn menu_stays_once_it_has_opened() {
        assert_eq!(screen_after_veil(MenuTransition::Opening), None);
    }

    #[test]
    fn menu_leaves_for_loading_once_it_has_closed() {
        assert_eq!(
            screen_after_veil(MenuTransition::Closing),
            Some(Screen::Loading)
        );
    }
}
