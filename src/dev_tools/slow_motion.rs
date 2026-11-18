use bevy::prelude::*;

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
const SLOWER: ActionName = ActionName("slow_motion_slower");
const SLOWER_KEY: KeyCode = KeyCode::NumpadSubtract;
const FASTER: ActionName = ActionName("slow_motion_faster");
const FASTER_KEY: KeyCode = KeyCode::NumpadAdd;

// ---- words: English, as every dev tool's, never read in a release build -----
const SLOW_MOTION_LABEL: &str = "Slow Motion";
const SLOWER_LABEL: &str = "Slower";
const FASTER_LABEL: &str = "Faster";

// ---- speeds: from the fastest to the slowest, as the bar lights them --------
const SLOW_MOTION_STEPS: [f32; 4] = [1.0, 0.5, 0.25, 0.1];
// how near a speed read back must be to a step to count as it, since an `f32` that went
// through `Time<Virtual>` may not equal the constant bit for bit
const SPEED_TOLERANCE: f32 = 1e-3;

// ---- capsule: top right, an hourglass, the speed, a bar of steps ------------
const CAPSULE_TOP_PX: f32 = 16.0;
const CAPSULE_RIGHT_PX: f32 = 16.0;
const CAPSULE_GAP_PX: f32 = 14.0;
// a HUD's icon stands alone, so it is larger than one beside text
const HOURGLASS_SIZE_PX: f32 = 32.0;
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

// ==== Plugin =================================================================

/// Slow motion on F4: a capsule showing the game's speed, stepped down and back up with
/// Numpad − and +, which only work while it is open. Gameplay and physics slow down, the UI
/// and the sound do not. Closing it puts the speed back at 1×.
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
}

// ==== Components =============================================================

/// Marks the root of the open slow motion capsule.
#[derive(Component)]
struct SlowMotionCapsule;

/// Marks the text that shows the speed.
#[derive(Component)]
struct SpeedText;

/// Marks a segment of the bar, with its place from the left: the first is lit at 1×.
#[derive(Component)]
struct SpeedSegment(usize);

// ==== Systems ================================================================

/// Opens the capsule, or closes it and puts the speed back at 1×, so nobody forgets the
/// game slowed down.
fn toggle_slow_motion(
    mut commands: Commands,
    open: Option<Single<Entity, With<SlowMotionCapsule>>>,
    mut time: ResMut<Time<Virtual>>,
    bindings: Res<Bindings>,
) {
    if let Some(capsule) = open {
        commands.entity(*capsule).despawn();
        time.set_relative_speed(1.0);
        return;
    }
    let speed = time.relative_speed();
    let window = (
        dev_window(Node {
            top: px(CAPSULE_TOP_PX),
            right: px(CAPSULE_RIGHT_PX),
            ..default()
        }),
        children![capsule_row(speed, &bindings)],
    );
    spawn_dev_window(&mut commands, SlowMotionCapsule, window);
}

/// Steps the speed one step slower or faster, and shows it on the capsule. Only while the
/// capsule is open, so its keys never steal a key from the game.
fn step_slow_motion(
    controls: Controls,
    mut time: ResMut<Time<Virtual>>,
    mut speed_text: Single<&mut Text, With<SpeedText>>,
    mut segments: Query<(&SpeedSegment, &mut BackgroundColor)>,
) {
    let speed = time.relative_speed();
    let new_speed = if controls.just_pressed(SLOWER) {
        slower(speed)
    } else if controls.just_pressed(FASTER) {
        faster(speed)
    } else {
        return;
    };
    time.set_relative_speed(new_speed);
    speed_text.0 = speed_label(new_speed);
    let lit = lit_segments(new_speed);
    for (SpeedSegment(place), mut fill) in &mut segments {
        fill.0 = segment_color(*place, lit);
    }
}

// ==== Helpers ================================================================

// ---- the capsule ------------------------------------------------------------

/// Returns what the capsule holds, side by side: the hourglass, the speed, the bar lit for
/// `speed`, and the two keys that step it, each on the keycap of its binding.
fn capsule_row(speed: f32, bindings: &Bindings) -> impl Bundle {
    let lit = lit_segments(speed);
    let segments = (0..SLOW_MOTION_STEPS.len()).map(move |place| {
        (
            Node {
                width: px(SEGMENT_WIDTH_PX),
                height: px(SEGMENT_HEIGHT_PX),
                border_radius: BorderRadius::all(px(SEGMENT_RADIUS_PX)),
                ..default()
            },
            BackgroundColor(segment_color(place, lit)),
            SpeedSegment(place),
        )
    });
    let legend = |action| {
        bindings
            .binding(action)
            .map(binding_legend)
            .unwrap_or_default()
    };
    (
        Node {
            align_items: AlignItems::Center,
            column_gap: px(CAPSULE_GAP_PX),
            ..default()
        },
        children![
            custom_icon(Icon::Hourglass, HOURGLASS_SIZE_PX, TEXT),
            (
                Node {
                    min_width: px(SPEED_MIN_WIDTH_PX),
                    ..default()
                },
                Text::new(speed_label(speed)),
                font(BODY_FONT_SIZE_PX),
                TextColor(TEXT),
                SpeedText,
            ),
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
    )
}

/// Returns the fill of the segment at `place` while `lit` segments are lit from the left.
fn segment_color(place: usize, lit: usize) -> Color {
    if place < lit {
        SEGMENT_LIT
    } else {
        SEGMENT_DARK
    }
}

/// Returns how a speed reads on the capsule: `0.25×`.
fn speed_label(speed: f32) -> String {
    format!("{speed}×")
}

// ---- the steps --------------------------------------------------------------

/// Returns the step after `speed` toward the slowest, or `speed` itself at the slowest. A
/// speed between two steps goes to the slower one.
fn slower(speed: f32) -> f32 {
    SLOW_MOTION_STEPS
        .into_iter()
        .find(|step| *step < speed - SPEED_TOLERANCE)
        .unwrap_or(speed)
}

/// Returns the step after `speed` toward 1×, or `speed` itself at 1×. A speed between two
/// steps goes to the faster one.
fn faster(speed: f32) -> f32 {
    SLOW_MOTION_STEPS
        .into_iter()
        .rev()
        .find(|step| *step > speed + SPEED_TOLERANCE)
        .unwrap_or(speed)
}

/// Returns how many segments of the bar are lit at `speed`: one at 1×, one more per step
/// slower.
fn lit_segments(speed: f32) -> usize {
    SLOW_MOTION_STEPS
        .into_iter()
        .filter(|step| *step > speed - SPEED_TOLERANCE)
        .count()
}

// ==== Tests ==================================================================

#[cfg(test)]
mod tests {
    use super::*;

    const FULL_SPEED: f32 = SLOW_MOTION_STEPS[0];
    const SLOWEST: f32 = SLOW_MOTION_STEPS[SLOW_MOTION_STEPS.len() - 1];
    const BEFORE_SLOWEST: f32 = SLOW_MOTION_STEPS[SLOW_MOTION_STEPS.len() - 2];

    // ---- slower and faster --------------------------------------------------

    #[test]
    fn one_step_slower_from_full_speed_is_the_second_step() {
        assert!((slower(FULL_SPEED) - SLOW_MOTION_STEPS[1]).abs() < SPEED_TOLERANCE);
    }

    #[test]
    fn slower_stops_at_the_slowest_step() {
        assert!((slower(SLOWEST) - SLOWEST).abs() < SPEED_TOLERANCE);
    }

    #[test]
    fn faster_stops_at_full_speed() {
        assert!((faster(FULL_SPEED) - FULL_SPEED).abs() < SPEED_TOLERANCE);
    }

    #[test]
    fn one_step_faster_from_the_slowest_is_the_step_before_it() {
        assert!((faster(SLOWEST) - BEFORE_SLOWEST).abs() < SPEED_TOLERANCE);
    }

    #[test]
    fn a_speed_read_back_a_little_off_still_steps_from_its_step() {
        let read_back = SLOW_MOTION_STEPS[1] + SPEED_TOLERANCE / 2.0;
        assert!((slower(read_back) - SLOW_MOTION_STEPS[2]).abs() < SPEED_TOLERANCE);
    }

    // ---- lit_segments -------------------------------------------------------

    #[test]
    fn full_speed_lights_one_segment_and_the_slowest_every_one() {
        assert_eq!(lit_segments(FULL_SPEED), 1);
        assert_eq!(lit_segments(SLOWEST), SLOW_MOTION_STEPS.len());
    }
}
