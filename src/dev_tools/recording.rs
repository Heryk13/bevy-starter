use bevy::dev_tools::fps_overlay::FPS_OVERLAY_ZINDEX;
use bevy::prelude::*;

use super::{
    CAPTURE, DEV_TOOLS_Z_INDEX, DevUi, SECS_PER_HOUR, SECS_PER_MINUTE, dev_window, spawn_dev_window,
};
use crate::input::{
    Action, ActionName, Binding, Bindings, Page, action_just_pressed, register_action,
};
use crate::overlays::{binding_legend, keycap};
use crate::screens::{Pause, Screen};
use crate::ui::blur::Floating;
use crate::ui::{
    BORDER_PX, DANGER, Icon, SMALL_FONT_SIZE_PX, TEXT, card_glass, custom_icon, floating_root, font,
};

mod bar;
mod capture;
mod encoder;
#[cfg(windows)]
mod sound;

use bar::RecordingChoices;

// ==== Config =================================================================

// opens and closes the recording's bar, cancels its countdown and stops it: the action, and
// the key it starts on. E starts and stops it too: `CAPTURE`, shared with photo mode
const RECORD: ActionName = ActionName("record");
const RECORD_KEY: KeyCode = KeyCode::F9;
// English, as every dev tool's words
const RECORD_LABEL: &str = "Screen Recording";

// ---- countdown: a big number in a round glass in the middle -----------------
const COUNTDOWN_SIZE_PX: f32 = 160.0;
// larger than any title: it is read from across the room while the player gets in place
const COUNTDOWN_FONT_SIZE_PX: f32 = 88.0;

// ---- REC pill: the dot, the time, how to stop it ----------------------------
// at the top centre: the top right is where slow motion's and speed's capsules open, and
// both may be open while recording
const PILL_TOP_PX: f32 = 16.0;
const PILL_GAP_PX: f32 = 10.0;
const DOT_SIZE_PX: f32 = 12.0;
const STOP_ICON_SIZE_PX: f32 = 14.0;
// how long one blink takes, lit then dim
const BLINK_SECS: f32 = 1.0;
const DOT_DIM_ALPHA: f32 = 0.25;

// ---- derived ----------------------------------------------------------------
const DOT_RADIUS_PX: f32 = DOT_SIZE_PX / 2.0;

// ==== Plugin =================================================================

/// Screen recording on F9, in gameplay only: a bar to set it up, E to start, a countdown,
/// then a video of the game alone, its own UI and its sound included and the dev UI never,
/// into the gallery, until E or F9 stops it. Its parts: the bar (`bar.rs`), the frames
/// (`capture.rs`), the file (`encoder.rs`) and, on Windows, the sound (`sound.rs`).
pub(super) fn plugin(app: &mut App) {
    register_action(
        app,
        Action {
            name: RECORD,
            default: Binding::Key(RECORD_KEY),
            page: Page::Dev,
            label: Some(RECORD_LABEL),
        },
    );
    app.init_state::<RecordingState>();
    app.add_plugins((bar::plugin, capture::plugin));
    app.add_systems(
        Update,
        (
            press_record.run_if(action_just_pressed(RECORD)),
            start_from_bar
                .run_if(action_just_pressed(CAPTURE))
                .run_if(in_state(RecordingState::Bar)),
            stop_recording
                .run_if(action_just_pressed(CAPTURE))
                .run_if(in_state(RecordingState::On)),
            count_down.run_if(in_state(RecordingState::Countdown)),
            blink_rec_pill.run_if(in_state(RecordingState::On)),
        ),
    );
    app.add_systems(OnEnter(RecordingState::Countdown), spawn_countdown);
    app.add_systems(
        OnEnter(RecordingState::On),
        (spawn_rec_pill, lend_dev_layer),
    );
    app.add_systems(OnExit(RecordingState::On), return_dev_layer);
    app.add_systems(OnExit(Screen::Gameplay), end_recording_with_gameplay);
}

// ==== State ==================================================================

/// Where the recording is: off, its bar open, counting down, or recording. Whatever each
/// value spawns carries `DespawnOnExit` of it. Photo mode's key does nothing but in `Off`,
/// so the two tools never run together.
#[derive(States, Default, Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub(super) enum RecordingState {
    #[default]
    Off,
    Bar,
    Countdown,
    On,
}

// ==== Components =============================================================

/// Marks the countdown's number, with how long it has counted, in real seconds.
#[derive(Component)]
struct CountdownNumber {
    total_secs: u32,
    elapsed_secs: f32,
}

/// Marks the REC pill's text, with how long the recording has gone on, in real seconds.
#[derive(Component, Default)]
struct RecTime {
    elapsed_secs: f32,
}

/// Marks the REC pill's blinking dot.
#[derive(Component)]
struct RecDot;

/// What a dev root drawn by the screen camera was before the recording lent it to the dev
/// tools' layer: its `GlobalZIndex`, to give back once the recording ends.
#[derive(Component)]
struct LentToDevLayer(GlobalZIndex);

// ==== Systems ================================================================

/// Moves the recording on F9: opens the bar in the running game, closes it, cancels the
/// countdown, stops a recording.
fn press_record(
    state: Res<State<RecordingState>>,
    screen: Res<State<Screen>>,
    pause: Option<Res<State<Pause>>>,
    mut next_state: ResMut<NextState<RecordingState>>,
) {
    let running =
        *screen.get() == Screen::Gameplay && pause.is_some_and(|pause| *pause.get() == Pause::Off);
    next_state.set_if_different(record_key_target(*state.get(), running));
}

/// Starts from the bar on E, as its Record button does: the countdown, or the recording at
/// once with no timer.
fn start_from_bar(
    choices: Res<RecordingChoices>,
    mut next_state: ResMut<NextState<RecordingState>>,
) {
    next_state.set_if_different(start_target(choices.timer_secs));
}

/// Stops the recording on E; leaving `On` writes the file.
fn stop_recording(mut next_state: ResMut<NextState<RecordingState>>) {
    next_state.set_if_different(RecordingState::Off);
}

/// Spawns the countdown in the middle of the screen, from the timer the bar chose.
fn spawn_countdown(mut commands: Commands, choices: Res<RecordingChoices>) {
    let circle = (
        Node {
            width: px(COUNTDOWN_SIZE_PX),
            height: px(COUNTDOWN_SIZE_PX),
            justify_content: JustifyContent::Center,
            align_items: AlignItems::Center,
            border: UiRect::all(px(BORDER_PX)),
            border_radius: BorderRadius::MAX,
            ..default()
        },
        card_glass(),
        children![(
            Text::new(choices.timer_secs.to_string()),
            font(COUNTDOWN_FONT_SIZE_PX),
            TextColor(TEXT),
            CountdownNumber {
                total_secs: choices.timer_secs,
                elapsed_secs: 0.0,
            },
        )],
    );
    spawn_dev_window(
        &mut commands,
        DespawnOnExit(RecordingState::Countdown),
        circle,
    );
}

/// Counts the countdown down in real time, so slow motion never slows it, and starts the
/// recording at zero.
fn count_down(
    time: Res<Time<Real>>,
    number: Single<(&mut Text, &mut CountdownNumber)>,
    mut next_state: ResMut<NextState<RecordingState>>,
) {
    let (mut text, mut number) = number.into_inner();
    number.elapsed_secs += time.delta_secs();
    match countdown_number(number.total_secs, number.elapsed_secs) {
        Some(shown) => {
            let shown = shown.to_string();
            if text.0 != shown {
                text.0 = shown;
            }
        }
        None => next_state.set_if_different(RecordingState::On),
    }
}

/// Spawns the REC pill at the top centre: the blinking dot, the time so far, a stop sign and
/// the key that stops it. A hint only, never in the video: the cursor is locked while
/// playing, so nothing on it is clicked.
fn spawn_rec_pill(mut commands: Commands, bindings: Res<Bindings>) {
    let legend = bindings
        .binding(CAPTURE)
        .map(binding_legend)
        .unwrap_or_default();
    let pill = (
        dev_window(Node {
            top: px(PILL_TOP_PX),
            ..default()
        }),
        children![(
            Node {
                align_items: AlignItems::Center,
                column_gap: px(PILL_GAP_PX),
                ..default()
            },
            children![
                (
                    Node {
                        width: px(DOT_SIZE_PX),
                        height: px(DOT_SIZE_PX),
                        border_radius: BorderRadius::all(px(DOT_RADIUS_PX)),
                        ..default()
                    },
                    BackgroundColor(DANGER),
                    RecDot,
                ),
                (
                    Text::new(elapsed_text(0)),
                    font(SMALL_FONT_SIZE_PX),
                    TextColor(TEXT),
                    RecTime::default(),
                ),
                custom_icon(Icon::Square, STOP_ICON_SIZE_PX, TEXT),
                keycap(CAPTURE, legend),
            ],
        )],
    );
    // a root of its own along the top, which centres the pill across it
    commands
        .spawn((
            floating_root(DEV_TOOLS_Z_INDEX),
            Pickable::IGNORE,
            DevUi,
            DespawnOnExit(RecordingState::On),
            children![pill],
        ))
        .insert_recursive::<Children>(Pickable::IGNORE);
}

/// Blinks the dot once a second and writes the time, in real time, so slow motion slows the
/// game and never the clock the video keeps.
fn blink_rec_pill(
    time: Res<Time<Real>>,
    rec_time: Single<(&mut Text, &mut RecTime)>,
    mut dot: Single<&mut BackgroundColor, With<RecDot>>,
) {
    let (mut text, mut rec_time) = rec_time.into_inner();
    let before = rec_time.elapsed_secs as u64;
    rec_time.elapsed_secs += time.delta_secs();
    let lit = (rec_time.elapsed_secs % BLINK_SECS) < BLINK_SECS / 2.0;
    dot.0 = DANGER.with_alpha(if lit { 1.0 } else { DOT_DIM_ALPHA });
    let secs = rec_time.elapsed_secs as u64;
    if secs != before {
        text.0 = elapsed_text(secs);
    }
}

/// Lends the dev tools' layer the dev roots the screen camera draws, Bevy's FPS overlay and
/// the keys panel's hint, for as long as the recording lasts: the frame is taken under that
/// layer, so they stay on screen and out of the video. Only while recording, since an open
/// layer blurs every frame.
fn lend_dev_layer(
    mut commands: Commands,
    roots: Query<(Entity, &GlobalZIndex), (Without<ChildOf>, Without<Floating>, With<Node>)>,
) {
    for (root, z_index) in &roots {
        // the hint is the one dev root already at the layer's index that is not on the layer
        if z_index.0 == FPS_OVERLAY_ZINDEX || *z_index == DEV_TOOLS_Z_INDEX {
            commands
                .entity(root)
                .insert((LentToDevLayer(*z_index), DEV_TOOLS_Z_INDEX, Floating));
        }
    }
}

/// Gives the screen camera back what `lend_dev_layer` lent the dev tools' layer.
fn return_dev_layer(mut commands: Commands, lent: Query<(Entity, &LentToDevLayer)>) {
    for (root, LentToDevLayer(z_index)) in &lent {
        commands
            .entity(root)
            .insert(*z_index)
            .remove::<(LentToDevLayer, Floating, UiTargetCamera)>();
    }
}

/// Ends the recording, or closes its bar, as gameplay ends: a recording leaving gameplay
/// stops, and its file is written.
fn end_recording_with_gameplay(mut next_state: ResMut<NextState<RecordingState>>) {
    next_state.set_if_different(RecordingState::Off);
}

// ==== Helpers ================================================================

/// Returns where F9 takes the recording from `state`: the bar opens only in the running
/// game, and every other state goes back to off.
fn record_key_target(state: RecordingState, running: bool) -> RecordingState {
    match state {
        RecordingState::Off if running => RecordingState::Bar,
        _ => RecordingState::Off,
    }
}

/// Returns where starting from the bar goes: the countdown, or the recording at once with
/// no timer.
fn start_target(timer_secs: u32) -> RecordingState {
    if timer_secs == 0 {
        RecordingState::On
    } else {
        RecordingState::Countdown
    }
}

/// Returns the number a countdown of `total_secs` shows `elapsed_secs` in, rounded up, so
/// 3 shows for the whole first second; `None` once it is over.
fn countdown_number(total_secs: u32, elapsed_secs: f32) -> Option<u32> {
    let left = total_secs as f32 - elapsed_secs;
    (left > 0.0).then(|| left.ceil() as u32)
}

/// Returns how long a recording has gone on, as its pill writes it: `00:42`, then `1:02:05`
/// past an hour.
fn elapsed_text(secs: u64) -> String {
    let hours = secs / SECS_PER_HOUR;
    let minutes = secs % SECS_PER_HOUR / SECS_PER_MINUTE;
    let seconds = secs % SECS_PER_MINUTE;
    if hours > 0 {
        format!("{hours}:{minutes:02}:{seconds:02}")
    } else {
        format!("{minutes:02}:{seconds:02}")
    }
}

// ==== Tests ==================================================================

#[cfg(test)]
mod tests {
    use super::*;

    const TIMER_SECS: u32 = 3;

    // ---- the keys -----------------------------------------------------------

    #[test]
    fn f9_opens_the_bar_only_in_the_running_game() {
        assert_eq!(
            record_key_target(RecordingState::Off, true),
            RecordingState::Bar
        );
        assert_eq!(
            record_key_target(RecordingState::Off, false),
            RecordingState::Off
        );
    }

    #[test]
    fn f9_closes_cancels_and_stops_everything_else() {
        for state in [
            RecordingState::Bar,
            RecordingState::Countdown,
            RecordingState::On,
        ] {
            assert_eq!(record_key_target(state, true), RecordingState::Off);
        }
    }

    #[test]
    fn no_timer_starts_recording_at_once() {
        assert_eq!(start_target(0), RecordingState::On);
        assert_eq!(start_target(TIMER_SECS), RecordingState::Countdown);
    }

    // ---- the countdown ------------------------------------------------------

    #[test]
    fn the_countdown_shows_its_whole_seconds_left() {
        assert_eq!(countdown_number(TIMER_SECS, 0.5), Some(TIMER_SECS));
        assert_eq!(
            countdown_number(TIMER_SECS, TIMER_SECS as f32 - 0.5),
            Some(1)
        );
    }

    #[test]
    fn the_countdown_is_over_at_zero() {
        assert_eq!(countdown_number(TIMER_SECS, TIMER_SECS as f32), None);
    }

    // ---- the pill -----------------------------------------------------------

    #[test]
    fn the_time_reads_minutes_and_seconds_under_an_hour() {
        assert_eq!(elapsed_text(42), "00:42");
    }

    #[test]
    fn the_time_reads_hours_past_an_hour() {
        assert_eq!(
            elapsed_text(SECS_PER_HOUR + 2 * SECS_PER_MINUTE + 5),
            "1:02:05"
        );
    }
}
