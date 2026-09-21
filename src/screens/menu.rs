use bevy::audio::Volume;
use bevy::prelude::*;
use bevy::ui_widgets::Activate;

use super::Screen;
use crate::ui::button;

// ==== Config =================================================================

// ---- scrim: dark behind the buttons, clear over the artwork -----------------
const SCRIM_ALPHA: f32 = 0.8;
const SCRIM_STOP_PERCENT: f32 = 60.0;

// ---- button column ----------------------------------------------------------
const BUTTON_GAP_PX: f32 = 12.0;
const COLUMN_PADDING_LEFT_PX: f32 = 32.0;

// ---- veil: black cover that fades the menu in and out -----------------------
const VEIL_FADE_SECS: f32 = 0.8;
const VEIL_EASING: EaseFunction = EaseFunction::SmoothStep;
const VEIL_Z_INDEX: i32 = 1;

// ==== Plugin =================================================================

pub(super) fn plugin(app: &mut App) {
    app.add_systems(OnEnter(Screen::Menu), (spawn_menu_screen, play_menu_music));
    app.add_observer(activate_menu_button);
    app.add_systems(
        Update,
        (
            animate_veils,
            // read the veil after this frame's tick, so the sound never lags the picture
            update_menu_music_volume.after(animate_veils),
        )
            .run_if(in_state(Screen::Menu)),
    );
}

// ==== Components =============================================================

/// Names what a menu button does when it is pressed.
#[derive(Component)]
enum MenuAction {
    Play,
    Settings,
    Credits,
    Quit,
}

/// A black cover over the menu: it fades out as the menu opens and in as it leaves.
#[derive(Component)]
struct Veil {
    timer: Timer,
    fading_out: bool,
}

/// Marks the menu music, so its volume can be found and faded.
#[derive(Component)]
struct MenuMusic;

// ==== Systems ================================================================

/// Spawns the menu screen and its layers, from the background up.
fn spawn_menu_screen(mut commands: Commands, asset_server: Res<AssetServer>) {
    commands.spawn((
        Node {
            width: percent(100),
            height: percent(100),
            ..default()
        },
        children![
            // ponytail: stretch distorts art that is not at the window's aspect ratio;
            // a real "cover" needs a clipping parent plus min-width and min-height on the
            // image, not worth writing before the final art exists
            (
                Node {
                    width: percent(100),
                    height: percent(100),
                    ..default()
                },
                ImageNode::new(asset_server.load("menu/background.jpg"))
                    .with_mode(NodeImageMode::Stretch),
            ),
            (
                Node {
                    width: percent(100),
                    height: percent(100),
                    position_type: PositionType::Absolute,
                    ..default()
                },
                BackgroundGradient::from(LinearGradient::to_right(vec![
                    ColorStop::new(Color::BLACK.with_alpha(SCRIM_ALPHA), percent(0)),
                    ColorStop::new(Color::NONE, percent(SCRIM_STOP_PERCENT)),
                ])),
            ),
            (
                Node {
                    width: percent(100),
                    height: percent(100),
                    position_type: PositionType::Absolute,
                    flex_direction: FlexDirection::Column,
                    justify_content: JustifyContent::Center,
                    row_gap: px(BUTTON_GAP_PX),
                    padding: UiRect {
                        left: px(COLUMN_PADDING_LEFT_PX),
                        ..default()
                    },
                    ..default()
                },
                children![
                    (button("Play"), MenuAction::Play),
                    (button("Settings"), MenuAction::Settings),
                    (button("Credits"), MenuAction::Credits),
                    (button("Quit"), MenuAction::Quit),
                ],
            ),
            veil(false),
        ],
        DespawnOnExit(Screen::Menu),
    ));
}

/// Starts the menu music, looping until the menu closes.
fn play_menu_music(mut commands: Commands, asset_server: Res<AssetServer>) {
    commands.spawn((
        AudioPlayer::new(asset_server.load("menu/music.ogg")),
        PlaybackSettings::LOOP,
        MenuMusic,
        DespawnOnExit(Screen::Menu),
    ));
}

/// Runs the action of the menu button that was pressed.
fn activate_menu_button(
    activate: On<Activate>,
    mut commands: Commands,
    actions: Query<&MenuAction>,
) {
    if let Ok(action) = actions.get(activate.entity) {
        match action {
            MenuAction::Play => {
                commands.spawn((
                    veil(true),
                    DespawnOnExit(Screen::Menu),
                    GlobalZIndex(VEIL_Z_INDEX),
                ));
            }
            // the overlays these open arrive in step 4
            MenuAction::Settings => {}
            MenuAction::Credits => {}
            MenuAction::Quit => {
                commands.write_message(AppExit::Success);
            }
        }
    }
}

/// Fades every veil along its timer, then removes it or leaves the menu when it ends.
fn animate_veils(
    mut veils: Query<(Entity, &mut Veil, &mut BackgroundColor)>,
    time: Res<Time>,
    mut commands: Commands,
    mut next_state: ResMut<NextState<Screen>>,
) {
    for (entity, mut veil, mut background_color) in &mut veils {
        veil.timer.tick(time.delta());
        let alpha = veil_alpha(veil.timer.elapsed_secs(), veil.fading_out);
        background_color.0.set_alpha(alpha);

        if veil.timer.is_finished() {
            if veil.fading_out {
                next_state.set(Screen::Gameplay);
            } else {
                commands.entity(entity).despawn();
            }
        }
    }
}

/// Keeps the menu music at the player's volume, fading it with the veil.
fn update_menu_music_volume(
    mut music: Single<&mut AudioSink, With<MenuMusic>>,
    veil: Option<Single<&Veil>>,
    global_volume: Res<GlobalVolume>,
) {
    let fade = match veil {
        Some(veil) => 1.0 - veil_alpha(veil.timer.elapsed_secs(), veil.fading_out),
        None => 1.0,
    };
    // set every frame: `GlobalVolume` is only read when a sink starts, so a track that
    // plays for minutes would not follow the settings slider otherwise
    music.set_volume(global_volume.volume * Volume::Linear(fade));
}

// ==== Helpers ================================================================

/// Returns a full-screen veil that fades in the given direction, ready to spawn.
fn veil(fading_out: bool) -> impl Bundle {
    (
        Node {
            width: percent(100),
            height: percent(100),
            position_type: PositionType::Absolute,
            ..default()
        },
        BackgroundColor(Color::BLACK.with_alpha(veil_alpha(0.0, fading_out))),
        Veil {
            timer: Timer::from_seconds(VEIL_FADE_SECS, TimerMode::Once),
            fading_out,
        },
    )
}

/// Returns the veil alpha at the given time of its fade, with easing applied.
fn veil_alpha(elapsed_secs: f32, fading_out: bool) -> f32 {
    let progress = elapsed_secs / VEIL_FADE_SECS;
    let linear_alpha = if fading_out { progress } else { 1.0 - progress };
    VEIL_EASING.sample_clamped(linear_alpha)
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
        assert_eq!(veil_alpha(0.0, false), 1.0);
    }

    #[test]
    fn veil_is_half_dark_while_menu_opens() {
        let alpha = veil_alpha(VEIL_FADE_SECS / 2.0, false);
        assert!(
            (alpha - 0.5).abs() < TOLERANCE,
            "{alpha} should be approximately 0.5"
        );
    }

    #[test]
    fn veil_is_transparent_when_menu_finishes_opening() {
        assert_eq!(veil_alpha(VEIL_FADE_SECS, false), 0.0);
    }

    #[test]
    fn veil_is_transparent_when_menu_starts_closing() {
        assert_eq!(veil_alpha(0.0, true), 0.0);
    }

    #[test]
    fn veil_is_half_dark_while_menu_closes() {
        let alpha = veil_alpha(VEIL_FADE_SECS / 2.0, true);
        assert!(
            (alpha - 0.5).abs() < TOLERANCE,
            "{alpha} should be approximately 0.5"
        );
    }

    #[test]
    fn veil_is_black_when_menu_finishes_closing() {
        assert_eq!(veil_alpha(VEIL_FADE_SECS, true), 1.0);
    }
}
