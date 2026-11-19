use bevy::prelude::*;

use super::speed::SpeedCapsule;
use super::{dev_window, register_inner_key, spawn_dev_window};
use crate::input::{
    Action, ActionName, Binding, Bindings, Controls, Page, action_just_pressed, register_action,
};
use crate::overlays::{binding_legend, keycap};
use crate::screens::Pause;
use crate::ui::{ACCENT, BODY_FONT_SIZE_PX, Icon, TEXT, custom_icon, font};

// ==== Config =================================================================

// ---- actions: the capsule's key, and the two that step it while it is open --
const SLOW_MOTION: ActionName = ActionName("slow_motion");
const SLOW_MOTION_KEY: KeyCode = KeyCode::F4;
// shared with speed's capsule, since a key belongs to one action only
pub(super) const SLOWER: ActionName = ActionName("slower");
const SLOWER_KEY: KeyCode = KeyCode::NumpadSubtract;
pub(super) const FASTER: ActionName = ActionName("faster");
const FASTER_KEY: KeyCode = KeyCode::NumpadAdd;

// ---- words: English, as every dev tool's, never read in a release build -----
const SLOW_MOTION_LABEL: &str = "Slow Motion";
const SLOWER_LABEL: &str = "Slower";
const FASTER_LABEL: &str = "Faster";

// ---- speeds: from 1× outward, as the bar lights them ------------------------
const SLOW_MOTION_STEPS: [f32; 4] = [1.0, 0.5, 0.25, 0.1];

// ---- capsule: top right, an icon, a reading, a bar of steps -----------------
const CAPSULE_TOP_PX: f32 = 16.0;
const CAPSULE_RIGHT_PX: f32 = 16.0;
const CAPSULE_GAP_PX: f32 = 14.0;
// a HUD's icon stands alone, so it is larger than one beside text
const CAPSULE_ICON_SIZE_PX: f32 = 32.0;
// wide enough for the longest speed, 0.25×, so the bar does not move as the speed changes
const SPEED_MIN_WIDTH_PX: f32 = 64.0;
const SEGMENT_WIDTH_PX: f32 = 30.0;
const SEGMENT_HEIGHT_PX: f32 = 8.0;
const SEGMENT_GAP_PX: f32 = 4.0;
const SEGMENT_LIT: Color = ACCENT;
const SEGMENT_DARK: Color = Color::srgba(1.0, 1.0, 1.0, 0.16);
// between the two keys that step it
const KEYS_GAP_PX: f32 = 4.0;

// ---- derived ----------------------------------------------------------------
// fully round ends, as every capsule's
const SEGMENT_RADIUS_PX: f32 = SEGMENT_HEIGHT_PX / 2.0;
const FULL_SPEED: f32 = SLOW_MOTION_STEPS[0];

// ==== Plugin =================================================================

/// Slow motion on F4: a capsule showing the game's speed, stepped down and back up with
/// Numpad − and +, which only work while it is open. Gameplay and physics slow down, the UI
/// and the sound do not. Closing it, by its key or by opening speed's capsule (F8) in its
/// place, puts the speed back at 1×.
pub(super) fn plugin(app: &mut App) {
    for (name, key, label) in [
        (SLOW_MOTION, SLOW_MOTION_KEY, SLOW_MOTION_LABEL),
        (SLOWER, SLOWER_KEY, SLOWER_LABEL),
        (FASTER, FASTER_KEY, FASTER_LABEL),
    ] {
        register_action(
            app,
            Action {
                name,
                default: Binding::Key(key),
                page: Page::Dev,
                label: Some(label),
            },
        );
    }
    // the capsule shows these two once it is open, and they do nothing before
    register_inner_key(app, SLOWER);
    register_inner_key(app, FASTER);
    app.add_systems(
        Update,
        (
            toggle_slow_motion.run_if(action_just_pressed(SLOW_MOTION)),
            step_slow_motion.run_if(any_with_component::<SlowMotionCapsule>),
        )
            .chain()
            // hidden in photo mode, so it neither opens nor steps behind a photo
            .run_if(not(in_state(Pause::Photo))),
    );
    app.add_observer(reset_slow_motion);
}

// ==== Step Capsule ===========================================================

// what `speed.rs` builds its capsule from, so the two look and step alike; they stay here,
// in the first file that had them, until a third tool needs them (CONTRIBUTING)

/// Marks a segment of a capsule's bar, with its place from the left: the first is lit at
/// 1×. Both capsules use it, since only one is ever open.
#[derive(Component)]
pub(super) struct StepSegment(usize);

/// Returns a capsule at the top right, for `spawn_dev_window`: the icon, the `reading`, the
/// bar of `steps` lit up to `value`, and the Slower and Faster keys, each on the keycap of
/// its binding.
pub(super) fn step_capsule(
    symbol: Icon,
    reading: impl Bundle,
    steps: &[f32],
    value: f32,
    bindings: &Bindings,
) -> impl Bundle {
    let lit = lit_segments(steps, value);
    let segments = (0..steps.len()).map(move |place| {
        (
            Node {
                width: px(SEGMENT_WIDTH_PX),
                height: px(SEGMENT_HEIGHT_PX),
                border_radius: BorderRadius::all(px(SEGMENT_RADIUS_PX)),
                ..default()
            },
            BackgroundColor(segment_color(place, lit)),
            StepSegment(place),
        )
    });
    let legend = |action| {
        bindings
            .binding(action)
            .map(binding_legend)
            .unwrap_or_default()
    };
    (
        dev_window(Node {
            top: px(CAPSULE_TOP_PX),
            right: px(CAPSULE_RIGHT_PX),
            ..default()
        }),
        children![(
            Node {
                align_items: AlignItems::Center,
                column_gap: px(CAPSULE_GAP_PX),
                ..default()
            },
            children![
                custom_icon(symbol, CAPSULE_ICON_SIZE_PX, TEXT),
                reading,
                (
                    Node {
                        column_gap: px(SEGMENT_GAP_PX),
                        ..default()
                    },
                    Children::spawn(SpawnIter(segments)),
                ),
                (
                    Node {
                        column_gap: px(KEYS_GAP_PX),
                        ..default()
                    },
                    children![
                        keycap(SLOWER, legend(SLOWER)),
                        keycap(FASTER, legend(FASTER)),
                    ],
                ),
            ],
        )],
    )
}

/// Lights the bar of `steps` up to `value`.
pub(super) fn paint_step_bar(
    steps: &[f32],
    value: f32,
    segments: &mut Query<(&StepSegment, &mut BackgroundColor)>,
) {
    let lit = lit_segments(steps, value);
    for (StepSegment(place), mut fill) in segments {
        fill.0 = segment_color(*place, lit);
    }
}

/// Returns how a step reads on a capsule: `0.25×`, `5×`.
pub(super) fn step_label(value: f32) -> String {
    format!("{value}×")
}

// ---- the steps: a list written from 1× outward ------------------------------

/// Returns the step one further from 1× than `value`, or the last step at the last.
pub(super) fn step_out(steps: &[f32], value: f32) -> f32 {
    let place = nearest_step(steps, value);
    steps
        .get(place + 1)
        .or_else(|| steps.last())
        .copied()
        .unwrap_or(value)
}

/// Returns the step one nearer 1× than `value`, or 1× itself at 1×.
pub(super) fn step_in(steps: &[f32], value: f32) -> f32 {
    let place = nearest_step(steps, value);
    steps.get(place.saturating_sub(1)).copied().unwrap_or(value)
}

/// Returns how many segments of the bar are lit at `value`: one at 1×, one more per step
/// outward.
fn lit_segments(steps: &[f32], value: f32) -> usize {
    nearest_step(steps, value) + 1
}

/// Returns the place in `steps` of the step nearest `value`, so a value that went through
/// `Time<Virtual>` and came back a little off, as an `f32` may, still counts as its step.
fn nearest_step(steps: &[f32], value: f32) -> usize {
    steps
        .iter()
        .enumerate()
        .min_by(|(_, a), (_, b)| (*a - value).abs().total_cmp(&(*b - value).abs()))
        .map_or(0, |(place, _)| place)
}

/// Returns the fill of the segment at `place` while `lit` segments are lit from the left.
fn segment_color(place: usize, lit: usize) -> Color {
    if place < lit {
        SEGMENT_LIT
    } else {
        SEGMENT_DARK
    }
}

// ==== Components =============================================================

/// Marks the root of the open slow motion capsule.
#[derive(Component)]
pub(super) struct SlowMotionCapsule;

/// Marks the text that shows the speed.
#[derive(Component)]
struct SpeedText;

// ==== Systems ================================================================

/// Opens the capsule in place of speed's, or closes it; `reset_slow_motion` puts the speed
/// back at 1× as it goes.
fn toggle_slow_motion(
    mut commands: Commands,
    open: Option<Single<Entity, With<SlowMotionCapsule>>>,
    speed_capsules: Query<Entity, With<SpeedCapsule>>,
    time: Res<Time<Virtual>>,
    bindings: Res<Bindings>,
) {
    if let Some(capsule) = open {
        commands.entity(*capsule).despawn();
        return;
    }
    for capsule in &speed_capsules {
        commands.entity(capsule).despawn();
    }
    let speed = time.relative_speed();
    let reading = (
        Node {
            min_width: px(SPEED_MIN_WIDTH_PX),
            ..default()
        },
        Text::new(step_label(speed)),
        font(BODY_FONT_SIZE_PX),
        TextColor(TEXT),
        SpeedText,
    );
    let capsule = step_capsule(
        Icon::Hourglass,
        reading,
        &SLOW_MOTION_STEPS,
        speed,
        &bindings,
    );
    spawn_dev_window(&mut commands, SlowMotionCapsule, capsule);
}

/// Steps the speed one step slower or faster, and shows it on the capsule. Only while the
/// capsule is open, so its keys never steal a key from the game.
fn step_slow_motion(
    controls: Controls,
    mut time: ResMut<Time<Virtual>>,
    mut speed_text: Single<&mut Text, With<SpeedText>>,
    mut segments: Query<(&StepSegment, &mut BackgroundColor)>,
) {
    let speed = time.relative_speed();
    let new_speed = if controls.just_pressed(SLOWER) {
        step_out(&SLOW_MOTION_STEPS, speed)
    } else if controls.just_pressed(FASTER) {
        step_in(&SLOW_MOTION_STEPS, speed)
    } else {
        return;
    };
    time.set_relative_speed(new_speed);
    speed_text.0 = step_label(new_speed);
    paint_step_bar(&SLOW_MOTION_STEPS, new_speed, &mut segments);
}

/// Puts the speed back at 1× once the capsule goes, so nobody forgets the game slowed down,
/// whether its key closed it or speed's capsule took its place.
fn reset_slow_motion(_: On<Remove<SlowMotionCapsule>>, mut time: ResMut<Time<Virtual>>) {
    time.set_relative_speed(FULL_SPEED);
}

// ==== Tests ==================================================================

#[cfg(test)]
mod tests {
    use super::*;

    const TOLERANCE: f32 = 1e-6;
    // as far off as an `f32` read back from `Time<Virtual>` could be, and far less than
    // half the gap between two steps
    const READ_BACK_ERROR: f32 = 1e-4;

    const SLOWEST: f32 = SLOW_MOTION_STEPS[SLOW_MOTION_STEPS.len() - 1];
    const BEFORE_SLOWEST: f32 = SLOW_MOTION_STEPS[SLOW_MOTION_STEPS.len() - 2];

    // ---- step_out and step_in -----------------------------------------------

    #[test]
    fn one_step_out_from_full_speed_is_the_second_step() {
        let out = step_out(&SLOW_MOTION_STEPS, FULL_SPEED);
        assert!((out - SLOW_MOTION_STEPS[1]).abs() < TOLERANCE);
    }

    #[test]
    fn stepping_out_stops_at_the_slowest_step() {
        assert!((step_out(&SLOW_MOTION_STEPS, SLOWEST) - SLOWEST).abs() < TOLERANCE);
    }

    #[test]
    fn stepping_in_stops_at_full_speed() {
        assert!((step_in(&SLOW_MOTION_STEPS, FULL_SPEED) - FULL_SPEED).abs() < TOLERANCE);
    }

    #[test]
    fn one_step_in_from_the_slowest_is_the_step_before_it() {
        assert!((step_in(&SLOW_MOTION_STEPS, SLOWEST) - BEFORE_SLOWEST).abs() < TOLERANCE);
    }

    #[test]
    fn a_speed_read_back_a_little_off_still_steps_from_its_step() {
        let read_back = SLOW_MOTION_STEPS[1] + READ_BACK_ERROR;
        let out = step_out(&SLOW_MOTION_STEPS, read_back);
        assert!((out - SLOW_MOTION_STEPS[2]).abs() < TOLERANCE);
    }

    // ---- lit_segments -------------------------------------------------------

    #[test]
    fn full_speed_lights_one_segment_and_the_slowest_every_one() {
        assert_eq!(lit_segments(&SLOW_MOTION_STEPS, FULL_SPEED), 1);
        assert_eq!(
            lit_segments(&SLOW_MOTION_STEPS, SLOWEST),
            SLOW_MOTION_STEPS.len()
        );
    }
}
