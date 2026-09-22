use bevy::prelude::*;

// ==== Config =================================================================

// the same curve on every fade in the game, so none of them feels different
const FADE_EASING: EaseFunction = EaseFunction::SmoothStep;

// ==== Types ==================================================================

/// The timing of a fade: invisible for a while, then in, held, and out again.
/// Each screen builds its own and applies the alpha to whatever it is fading.
pub(crate) struct Fade {
    pub(crate) delay_secs: f32,
    pub(crate) fade_in_secs: f32,
    pub(crate) hold_secs: f32,
    pub(crate) fade_out_secs: f32,
}

impl Fade {
    /// Returns the alpha at the given time into the fade, eased, from 0.0 to 1.0.
    pub(crate) fn alpha(&self, elapsed_secs: f32) -> f32 {
        let fade_in_end_secs = self.delay_secs + self.fade_in_secs;
        let hold_end_secs = fade_in_end_secs + self.hold_secs;
        let total_secs = hold_end_secs + self.fade_out_secs;

        let linear_alpha = if elapsed_secs < self.delay_secs {
            // waiting
            0.0
        } else if elapsed_secs < fade_in_end_secs {
            // fading in
            (elapsed_secs - self.delay_secs) / self.fade_in_secs
        } else if elapsed_secs < hold_end_secs {
            // holding
            1.0
        } else if elapsed_secs < total_secs {
            // fading out
            let remaining_secs = total_secs - elapsed_secs;
            remaining_secs / self.fade_out_secs
        } else {
            // done
            0.0
        };
        FADE_EASING.sample_clamped(linear_alpha)
    }
}

// ==== Tests ==================================================================

#[cfg(test)]
mod tests {
    use super::*;

    const TOLERANCE: f32 = 1e-5;
    const FADE: Fade = Fade {
        delay_secs: 1.0,
        fade_in_secs: 1.0,
        hold_secs: 1.0,
        fade_out_secs: 1.0,
    };

    #[test]
    fn fade_is_invisible_during_delay() {
        assert!((FADE.alpha(0.5) - 0.0).abs() < TOLERANCE);
    }

    #[test]
    fn fade_is_half_visible_mid_fade_in() {
        assert!((FADE.alpha(1.5) - 0.5).abs() < TOLERANCE);
    }

    #[test]
    fn fade_is_visible_during_hold() {
        assert!((FADE.alpha(2.5) - 1.0).abs() < TOLERANCE);
    }

    #[test]
    fn fade_is_half_visible_mid_fade_out() {
        assert!((FADE.alpha(3.5) - 0.5).abs() < TOLERANCE);
    }

    #[test]
    fn fade_is_invisible_after_fade_out() {
        assert!((FADE.alpha(4.5) - 0.0).abs() < TOLERANCE);
    }

    #[test]
    fn fade_starts_visible_without_fade_in() {
        let fade = Fade {
            delay_secs: 0.0,
            fade_in_secs: 0.0,
            hold_secs: 1.0,
            fade_out_secs: 1.0,
        };
        assert!((fade.alpha(0.0) - 1.0).abs() < TOLERANCE);
    }

    #[test]
    fn fade_ends_invisible_without_fade_out() {
        let fade = Fade {
            delay_secs: 1.0,
            fade_in_secs: 1.0,
            hold_secs: 1.0,
            fade_out_secs: 0.0,
        };
        assert!((fade.alpha(3.0) - 0.0).abs() < TOLERANCE);
    }
}
