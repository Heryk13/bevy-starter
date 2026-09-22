use bevy::prelude::*;

use super::Screen;

// ==== Config =================================================================

// ---- skip input, shared with the intro so both skip the same way ------------
pub(super) const SKIP_KEYS: [KeyCode; 5] = [
    KeyCode::Escape,
    KeyCode::Space,
    KeyCode::Enter,
    KeyCode::KeyE,
    KeyCode::KeyF,
];
pub(super) const SKIP_MOUSE_BUTTONS: [MouseButton; 2] = [MouseButton::Left, MouseButton::Right];

// ---- logo animation: wait, fade in, hold, fade out --------------------------
const LOGO_WAIT_SECS: f32 = 0.5;
const LOGO_FADE_IN_SECS: f32 = 0.8;
const LOGO_HOLD_SECS: f32 = 1.2;
const LOGO_FADE_OUT_SECS: f32 = 0.5;
const LOGO_EASING: EaseFunction = EaseFunction::SmoothStep;

// ---- when each phase ends, derived from the durations above -----------------
const LOGO_FADE_IN_END_SECS: f32 = LOGO_WAIT_SECS + LOGO_FADE_IN_SECS;
const LOGO_HOLD_END_SECS: f32 = LOGO_FADE_IN_END_SECS + LOGO_HOLD_SECS;
const LOGO_TOTAL_SECS: f32 = LOGO_HOLD_END_SECS + LOGO_FADE_OUT_SECS;

// ==== Plugin =================================================================

pub(super) fn plugin(app: &mut App) {
    app.add_systems(OnEnter(Screen::Splash), spawn_splash_screen);
    app.add_systems(
        Update,
        (
            tick_splash_timer,
            skip_splash_screen,
            // read the timer after this frame's tick, so the fade never lags a frame behind
            fade_splash_logo.after(tick_splash_timer),
        )
            .run_if(in_state(Screen::Splash)),
    );
}

// ==== Components =============================================================

/// Drives the logo animation and the transition out of the splash screen.
#[derive(Component)]
struct SplashTimer {
    timer: Timer,
}

// ==== Systems ================================================================

/// Spawns the splash screen: a full-screen container with the logo centered in it.
fn spawn_splash_screen(mut commands: Commands, asset_server: Res<AssetServer>) {
    commands.spawn((
        Node {
            width: percent(100),
            height: percent(100),
            align_items: AlignItems::Center,
            justify_content: JustifyContent::Center,
            ..default()
        },
        children![(
            Node {
                width: percent(30),
                ..default()
            },
            // starts invisible, `fade_splash_logo` fades it in
            ImageNode::new(asset_server.load("splash/bevy.png"))
                .with_color(Color::WHITE.with_alpha(0.0)),
            SplashTimer {
                timer: Timer::from_seconds(LOGO_TOTAL_SECS, TimerMode::Once),
            },
        )],
        // despawns the whole screen, logo and timer included, when leaving the splash
        DespawnOnExit(Screen::Splash),
    ));
}

/// Ticks the splash timer and moves to the menu when it finishes.
fn tick_splash_timer(
    mut splash_timer: Single<&mut SplashTimer>,
    time: Res<Time>,
    mut next_screen: ResMut<NextState<Screen>>,
) {
    splash_timer.timer.tick(time.delta());
    if splash_timer.timer.is_finished() {
        next_screen.set(Screen::Menu);
    }
}

/// Skips to the menu when a skip key or mouse button is pressed.
fn skip_splash_screen(
    mouse_input: Res<ButtonInput<MouseButton>>,
    keyboard_input: Res<ButtonInput<KeyCode>>,
    mut next_screen: ResMut<NextState<Screen>>,
) {
    if keyboard_input.any_just_pressed(SKIP_KEYS)
        || mouse_input.any_just_pressed(SKIP_MOUSE_BUTTONS)
    {
        next_screen.set(Screen::Menu);
    }
}

/// Fades the logo in and out following the splash timer.
fn fade_splash_logo(logo: Single<(&mut ImageNode, &SplashTimer)>) {
    let (mut logo, splash_timer) = logo.into_inner();
    let elapsed_secs = splash_timer.timer.elapsed_secs();
    let alpha = logo_alpha(elapsed_secs);
    logo.color.set_alpha(alpha);
}

// ==== Helpers ================================================================

/// Returns the logo alpha at the given time of the animation, with easing applied.
fn logo_alpha(elapsed_secs: f32) -> f32 {
    let linear_alpha = if elapsed_secs < LOGO_WAIT_SECS {
        // waiting
        0.0
    } else if elapsed_secs < LOGO_FADE_IN_END_SECS {
        // fading in
        (elapsed_secs - LOGO_WAIT_SECS) / LOGO_FADE_IN_SECS
    } else if elapsed_secs < LOGO_HOLD_END_SECS {
        // holding
        1.0
    } else {
        // fading out
        let remaining_secs = LOGO_TOTAL_SECS - elapsed_secs;
        remaining_secs / LOGO_FADE_OUT_SECS
    };
    LOGO_EASING.sample_clamped(linear_alpha)
}

// ==== Tests ==================================================================

#[cfg(test)]
mod tests {
    use super::*;

    // the ends are compared exactly: `sample_clamped` returns exactly 0.0 and 1.0 there,
    // so only the points in between need the tolerance
    const TOLERANCE: f32 = 1e-5;

    #[test]
    fn logo_is_invisible_at_the_start() {
        assert_eq!(logo_alpha(0.0), 0.0);
    }

    #[test]
    fn logo_is_half_visible_mid_fade_in() {
        let alpha = logo_alpha(LOGO_WAIT_SECS + LOGO_FADE_IN_SECS / 2.0);
        assert!((alpha - 0.5).abs() < TOLERANCE, "alpha was {alpha}");
    }

    #[test]
    fn logo_is_fully_visible_while_holding() {
        assert_eq!(logo_alpha(LOGO_FADE_IN_END_SECS), 1.0);
        assert_eq!(logo_alpha(LOGO_HOLD_END_SECS - 0.01), 1.0);
    }

    #[test]
    fn logo_is_fully_visible_when_fade_out_starts() {
        assert_eq!(logo_alpha(LOGO_HOLD_END_SECS), 1.0);
    }

    #[test]
    fn logo_is_half_visible_mid_fade_out() {
        let alpha = logo_alpha(LOGO_HOLD_END_SECS + LOGO_FADE_OUT_SECS / 2.0);
        assert!((alpha - 0.5).abs() < TOLERANCE, "alpha was {alpha}");
    }

    #[test]
    fn logo_is_invisible_at_the_end() {
        assert_eq!(logo_alpha(LOGO_TOTAL_SECS), 0.0);
    }
}
