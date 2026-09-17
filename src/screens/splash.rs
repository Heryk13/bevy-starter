use bevy::prelude::*;

use super::Screen;

#[derive(Resource)]
struct SplashTimer {
    timer: Timer,
}

#[derive(Component)]
struct SplashLogo;

const SKIP_MOUSE_BUTTONS: [MouseButton; 2] = [MouseButton::Left, MouseButton::Right];

const SKIP_KEYS: [KeyCode; 5] = [
    KeyCode::Escape,
    KeyCode::Space,
    KeyCode::Enter,
    KeyCode::KeyE,
    KeyCode::KeyF,
];

const LOGO_WAIT_SECS: f32 = 0.5;
const LOGO_FADE_IN_SECS: f32 = 0.8;
const LOGO_HOLD_SECS: f32 = 1.2;
const LOGO_FADE_OUT_SECS: f32 = 0.5;
const LOGO_EASING: EaseFunction = EaseFunction::SmoothStep;

const LOGO_FADE_IN_END_SECS: f32 = LOGO_WAIT_SECS + LOGO_FADE_IN_SECS;
const LOGO_HOLD_END_SECS: f32 = LOGO_FADE_IN_END_SECS + LOGO_HOLD_SECS;
const LOGO_TOTAL_SECS: f32 = LOGO_HOLD_END_SECS + LOGO_FADE_OUT_SECS;

pub(super) fn plugin(app: &mut App) {
    app.add_systems(OnEnter(Screen::Splash), spawn_splash_screen);
    app.add_systems(
        Update,
        (tick_splash_timer, skip_splash_screen, fade_splash_logo).run_if(in_state(Screen::Splash)),
    );
    app.add_systems(OnExit(Screen::Splash), remove_splash_timer);
}

/// spawn the splash screen and initialize the splash timer
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
            ImageNode::new(asset_server.load("splash/bevy_logo_dark.png"))
                .with_color(Color::WHITE.with_alpha(0.0)),
            SplashLogo,
        )],
        DespawnOnExit(Screen::Splash),
    ));
    commands.insert_resource(SplashTimer {
        timer: Timer::from_seconds(LOGO_TOTAL_SECS, TimerMode::Once),
    });
}

/// tick the splash timer and transition to the next screen when finished
fn tick_splash_timer(
    mut splash_timer: ResMut<SplashTimer>,
    time: Res<Time>,
    mut next_state: ResMut<NextState<Screen>>,
) {
    splash_timer.timer.tick(time.delta());
    if splash_timer.timer.is_finished() {
        next_state.set(Screen::Menu);
    }
}

/// skip the splash screen on interaction
fn skip_splash_screen(
    mouse_input: Res<ButtonInput<MouseButton>>,
    keyboard_input: Res<ButtonInput<KeyCode>>,
    mut next_state: ResMut<NextState<Screen>>,
) {
    if keyboard_input.any_just_pressed(SKIP_KEYS)
        || mouse_input.any_just_pressed(SKIP_MOUSE_BUTTONS)
    {
        next_state.set(Screen::Menu);
    }
}

/// fade in and out the bevy logo
fn fade_splash_logo(
    mut logo_query: Query<&mut ImageNode, With<SplashLogo>>,
    splash_timer: Res<SplashTimer>,
) -> Result {
    let mut logo = logo_query.single_mut()?;
    let elapsed_secs = splash_timer.timer.elapsed_secs();
    let linear_alpha = if elapsed_secs < LOGO_WAIT_SECS {
        0.0
    } else if elapsed_secs < LOGO_FADE_IN_END_SECS {
        (elapsed_secs - LOGO_WAIT_SECS) / LOGO_FADE_IN_SECS
    } else if elapsed_secs < LOGO_HOLD_END_SECS {
        1.0
    } else {
        let remaining_secs = splash_timer.timer.remaining_secs();
        remaining_secs / LOGO_FADE_OUT_SECS
    };
    let alpha = LOGO_EASING.sample_clamped(linear_alpha);
    logo.color.set_alpha(alpha);
    Ok(())
}

/// remove the splash timer resource when exiting the splash screen
fn remove_splash_timer(mut commands: Commands) {
    commands.remove_resource::<SplashTimer>();
}
