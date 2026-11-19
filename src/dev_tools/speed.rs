use bevy::prelude::*;

use super::slow_motion::{
    FASTER, SLOWER, SlowMotionCapsule, StepSegment, paint_step_bar, step_capsule, step_in,
    step_label, step_out,
};
use super::spawn_dev_window;
use crate::input::{
    Action, ActionName, Binding, Bindings, Controls, Page, action_just_pressed, register_action,
};
use crate::screens::Pause;
use crate::ui::{BODY_FONT_SIZE_PX, Icon, SMALL_FONT_SIZE_PX, TEXT, font, muted_text};

// ==== Config =================================================================

// ---- action: the capsule's key; Slower and Faster are slow motion's ---------
const SPEED: ActionName = ActionName("speed");
const SPEED_KEY: KeyCode = KeyCode::F8;
// English, as every dev tool's, never read in a release build
const SPEED_LABEL: &str = "Speed";

// ---- multiplier: from 1× outward, as the bar lights them --------------------
const SPEED_STEPS: [f32; 4] = [1.0, 2.0, 5.0, 10.0];

// ---- speedometer ------------------------------------------------------------
// long enough to smooth out avian moving a body only on its fixed steps, as the FPS
// counter smooths out its frames
const SPEEDOMETER_REFRESH_SECS: f32 = 0.25;
// in place of a speed while nothing carries a `Speedometer`
const NO_SPEED: &str = "—";

// ---- capsule: slow motion's, with the speed over the multiplier -------------
// wide enough for 2560.0 px/s, the 2D walk at 10×, so the bar does not move as it changes
const READING_MIN_WIDTH_PX: f32 = 104.0;

// ---- derived ----------------------------------------------------------------
const NORMAL_MULTIPLIER: f32 = SPEED_STEPS[0];

// ==== Plugin =================================================================

/// Speed on F8: a capsule showing how fast the body with a `Speedometer` goes, over the
/// `SpeedMultiplier` a player multiplies its walk by, stepped with Numpad − and +. It takes
/// slow motion's place on screen and its keys, so opening one closes the other, and closing
/// it puts the multiplier back at 1×. With no player it shows `—`.
pub(super) fn plugin(app: &mut App) {
    register_action(
        app,
        Action {
            name: SPEED,
            default: Binding::Key(SPEED_KEY),
            page: Page::Dev,
            label: Some(SPEED_LABEL),
        },
    );
    app.init_resource::<SpeedMultiplier>();
    app.add_systems(
        Update,
        (
            toggle_speed.run_if(action_just_pressed(SPEED)),
            (step_speed, measure_speed).run_if(any_with_component::<SpeedCapsule>),
        )
            .chain()
            // hidden in photo mode, so it neither opens nor steps behind a photo
            .run_if(not(in_state(Pause::Photo))),
    );
    app.add_observer(reset_speed);
}

// ==== Components =============================================================

/// Puts a body on speed's capsule, the one whose speed it shows, in `unit`: `"m/s"` in 3D,
/// `"px/s"` in 2D. A player adds it behind `#[cfg(feature = "dev")]`.
#[derive(Component)]
pub(crate) struct Speedometer {
    pub(crate) unit: &'static str,
}

/// Marks the root of the open speed capsule.
#[derive(Component)]
pub(super) struct SpeedCapsule;

/// The text showing the speed, with what it measured since it was last written. Born with
/// the capsule, so a measurement never starts from where the body was the last time.
#[derive(Component, Default)]
struct SpeedReading {
    // the body measured and where it was a frame ago: another body, as after loading a
    // save, starts over instead of counting the jump between the two
    last: Option<(Entity, Vec3)>,
    distance: f32,
    secs: f32,
}

/// Marks the text showing the multiplier.
#[derive(Component)]
struct MultiplierText;

// ==== Resources ==============================================================

/// How many times faster a player walks and flies. A setting of the tool, owned by no
/// entity, so a resource; back at 1× as the capsule closes.
#[derive(Resource)]
pub(crate) struct SpeedMultiplier(pub(crate) f32);

impl Default for SpeedMultiplier {
    fn default() -> Self {
        Self(NORMAL_MULTIPLIER)
    }
}

// ==== Systems ================================================================

/// Opens the capsule in place of slow motion's, or closes it; `reset_speed` puts the
/// multiplier back at 1× as it goes.
fn toggle_speed(
    mut commands: Commands,
    open: Option<Single<Entity, With<SpeedCapsule>>>,
    slow_motion_capsules: Query<Entity, With<SlowMotionCapsule>>,
    multiplier: Res<SpeedMultiplier>,
    bindings: Res<Bindings>,
) {
    if let Some(capsule) = open {
        commands.entity(*capsule).despawn();
        return;
    }
    for capsule in &slow_motion_capsules {
        commands.entity(capsule).despawn();
    }
    let reading = (
        Node {
            flex_direction: FlexDirection::Column,
            min_width: px(READING_MIN_WIDTH_PX),
            ..default()
        },
        children![
            (
                Text::new(NO_SPEED),
                font(BODY_FONT_SIZE_PX),
                TextColor(TEXT),
                SpeedReading::default(),
            ),
            (
                Text::new(step_label(multiplier.0)),
                muted_text(SMALL_FONT_SIZE_PX),
                MultiplierText,
            ),
        ],
    );
    let capsule = step_capsule(
        Icon::SportShoe,
        reading,
        &SPEED_STEPS,
        multiplier.0,
        &bindings,
    );
    spawn_dev_window(&mut commands, SpeedCapsule, capsule);
}

/// Steps the multiplier one step up on Faster or down on Slower, and shows it on the
/// capsule. Only while the capsule is open, so its keys never steal a key from the game.
fn step_speed(
    controls: Controls,
    mut multiplier: ResMut<SpeedMultiplier>,
    mut multiplier_text: Single<&mut Text, With<MultiplierText>>,
    mut segments: Query<(&StepSegment, &mut BackgroundColor)>,
) {
    let new_multiplier = if controls.just_pressed(FASTER) {
        step_out(&SPEED_STEPS, multiplier.0)
    } else if controls.just_pressed(SLOWER) {
        step_in(&SPEED_STEPS, multiplier.0)
    } else {
        return;
    };
    multiplier.0 = new_multiplier;
    multiplier_text.0 = step_label(new_multiplier);
    paint_step_bar(&SPEED_STEPS, new_multiplier, &mut segments);
}

/// Adds up how far the body with a `Speedometer` went each frame of game time, and writes
/// its speed every `SPEEDOMETER_REFRESH_SECS`. While the game is paused no time passes, so
/// the last speed stays.
fn measure_speed(
    time: Res<Time>,
    body: Option<Single<(Entity, &GlobalTransform, &Speedometer)>>,
    reading: Single<(&mut Text, &mut SpeedReading)>,
) {
    let (mut text, mut reading) = reading.into_inner();
    let Some(body) = body else {
        reading.last = None;
        // compared first, since a write would lay the text out again every frame
        if text.0 != NO_SPEED {
            text.0 = NO_SPEED.to_owned();
        }
        return;
    };
    let (entity, transform, speedometer) = *body;
    let position = transform.translation();
    match reading.last {
        Some((last_entity, last_position)) if last_entity == entity => {
            reading.distance += position.distance(last_position);
            reading.secs += time.delta_secs();
        }
        _ => {
            reading.distance = 0.0;
            reading.secs = 0.0;
        }
    }
    reading.last = Some((entity, position));
    if reading.secs < SPEEDOMETER_REFRESH_SECS {
        return;
    }
    if let Some(speed) = measured_speed(reading.distance, reading.secs) {
        text.0 = speed_text(speed, speedometer.unit);
    }
    reading.distance = 0.0;
    reading.secs = 0.0;
}

/// Puts the multiplier back at 1× once the capsule goes, whether its key closed it or slow
/// motion's capsule took its place.
fn reset_speed(_: On<Remove<SpeedCapsule>>, mut multiplier: ResMut<SpeedMultiplier>) {
    multiplier.0 = NORMAL_MULTIPLIER;
}

// ==== Helpers ================================================================

/// Returns the speed of a body that went `distance` in `secs`, or `None` when no time
/// passed.
fn measured_speed(distance: f32, secs: f32) -> Option<f32> {
    (secs > 0.0).then(|| distance / secs)
}

/// Returns how a speed reads on the capsule: `13.2 m/s`.
fn speed_text(speed: f32, unit: &str) -> String {
    format!("{speed:.1} {unit}")
}

// ==== Tests ==================================================================

#[cfg(test)]
mod tests {
    use super::*;

    const TOLERANCE: f32 = 1e-6;

    const FASTEST: f32 = SPEED_STEPS[SPEED_STEPS.len() - 1];

    // ---- the steps, which go up here ----------------------------------------

    #[test]
    fn one_step_out_from_1x_is_the_second_step() {
        let out = step_out(&SPEED_STEPS, NORMAL_MULTIPLIER);
        assert!((out - SPEED_STEPS[1]).abs() < TOLERANCE);
    }

    #[test]
    fn stepping_out_stops_at_the_fastest_step() {
        assert!((step_out(&SPEED_STEPS, FASTEST) - FASTEST).abs() < TOLERANCE);
    }

    #[test]
    fn stepping_in_stops_at_1x() {
        let back = step_in(&SPEED_STEPS, NORMAL_MULTIPLIER);
        assert!((back - NORMAL_MULTIPLIER).abs() < TOLERANCE);
    }

    // ---- measured_speed and speed_text --------------------------------------

    #[test]
    fn three_metres_over_the_refresh_time_read_as_their_speed() {
        let speed = measured_speed(3.0, SPEEDOMETER_REFRESH_SECS);
        let expected = 3.0 / SPEEDOMETER_REFRESH_SECS;
        assert!(speed.is_some_and(|speed| (speed - expected).abs() < TOLERANCE));
    }

    #[test]
    fn no_time_passed_reads_no_speed() {
        assert_eq!(measured_speed(3.0, 0.0), None);
    }

    #[test]
    fn a_speed_reads_with_one_decimal_and_its_unit() {
        assert_eq!(speed_text(13.24, "m/s"), "13.2 m/s");
    }
}
