use bevy::prelude::*;
use bevy::ui_widgets::Activate;

use super::{RecordingState, start_target};
use crate::dev_tools::{CAPTURE, DEV_TOOLS_Z_INDEX, DevUi, dev_window};
use crate::input::Bindings;
use crate::overlays::{binding_legend, keycap};
use crate::ui::{
    BODY_FONT_SIZE_PX, BORDER, BORDER_PX, ButtonKind, ClosesOnEsc, Dismiss, Icon, TEXT, card_glass,
    floating_root, font, icon, laid_out_button,
};

// ==== Config =================================================================

// ---- choices: the timer's steps, and where the bar starts -------------------
const TIMER_STEPS_SECS: [u32; 4] = [0, 3, 5, 10];
const FIRST_TIMER_SECS: u32 = 3;
const FRAME_RATES: [u32; 2] = [30, 60];
const FIRST_FRAME_RATE: u32 = 60;
// the sound is heard through Windows' process loopback, so elsewhere there is none to record
const SOUND_RECORDABLE: bool = cfg!(windows);

// ---- words: English, as every dev tool's ------------------------------------
const SOUND_ON_WORDS: &str = "Sound on";
const SOUND_OFF_WORDS: &str = "Sound off";
const NO_TIMER_WORDS: &str = "None";
const RECORD_WORDS: &str = "Record";

// ---- bar: the bottom centre, a row of tools ---------------------------------
const BAR_BOTTOM_PX: f32 = 24.0;
const BAR_GAP_PX: f32 = 6.0;
const TOOL_HEIGHT_PX: f32 = 36.0;
const TOOL_PADDING_X_PX: f32 = 12.0;
// between a tool's icon and its words
const TOOL_PART_GAP_PX: f32 = 8.0;
const DIVIDER_HEIGHT_PX: f32 = 24.0;

// ---- the timer's list: above the timer, a row a step ------------------------
const LIST_GAP_PX: f32 = 8.0;
const LIST_PADDING_PX: f32 = 6.0;
const LIST_RADIUS_PX: f32 = 16.0;
const LIST_MIN_WIDTH_PX: f32 = 150.0;
const ROW_HEIGHT_PX: f32 = 32.0;
const CHECK_SIZE_PX: f32 = 16.0;

// ---- derived ----------------------------------------------------------------
// the list's bottom edge sits this far over the timer's own bottom
const LIST_BOTTOM_PX: f32 = TOOL_HEIGHT_PX + LIST_GAP_PX;

// ==== Plugin =================================================================

/// The recording's bar: Sound, the frame rate and the timer, each set with a click in place,
/// and Record. The choices last until the game closes.
pub(super) fn plugin(app: &mut App) {
    app.init_resource::<RecordingChoices>();
    app.add_systems(OnEnter(RecordingState::Bar), spawn_bar);
    app.add_systems(Update, fill_bar.run_if(in_state(RecordingState::Bar)));
    app.add_observer(dismiss_bar.run_if(in_state(RecordingState::Bar)));
}

// ==== Components =============================================================

/// Marks the root of the open bar, the one Esc dismisses.
#[derive(Component)]
struct RecordingBar;

/// Marks the row the tools are spawned into, with whether the timer's list is open.
#[derive(Component, Default)]
struct BarTools {
    list_open: bool,
}

// ==== Resources ==============================================================

/// What the bar is set to: whether the sound is recorded, the frames a second, and the
/// countdown's seconds, 0 for none. Kept until the game closes, not in the settings file:
/// a recording's setup is a moment's.
#[derive(Resource)]
pub(super) struct RecordingChoices {
    pub(super) sound: bool,
    pub(super) frame_rate: u32,
    pub(super) timer_secs: u32,
}

impl Default for RecordingChoices {
    fn default() -> Self {
        Self {
            sound: SOUND_RECORDABLE,
            frame_rate: FIRST_FRAME_RATE,
            timer_secs: FIRST_TIMER_SECS,
        }
    }
}

// ==== Systems ================================================================

/// Spawns the bar at the bottom centre. Its root lets the pointer through, as the file
/// explorer's does, so the game around it stays sharp, and the bar itself takes it.
fn spawn_bar(mut commands: Commands) {
    commands.spawn((
        floating_root(DEV_TOOLS_Z_INDEX),
        Pickable::IGNORE,
        DevUi,
        ClosesOnEsc,
        RecordingBar,
        DespawnOnExit(RecordingState::Bar),
        children![(
            // left to the root's centring along the bottom edge
            dev_window(Node {
                bottom: px(BAR_BOTTOM_PX),
                ..default()
            }),
            children![(
                Node {
                    align_items: AlignItems::Center,
                    column_gap: px(BAR_GAP_PX),
                    ..default()
                },
                BarTools::default(),
            )],
        )],
    ));
}

/// Spawns the tools as the bar opens and again whenever a choice changes or the timer's list
/// opens or closes, each tool showing what it is set to.
fn fill_bar(
    mut commands: Commands,
    choices: Res<RecordingChoices>,
    bindings: Res<Bindings>,
    tools: Single<(Entity, Ref<BarTools>)>,
) {
    let (row, tools) = tools.into_inner();
    if !tools.is_changed() && !choices.is_changed() {
        return;
    }
    commands.entity(row).despawn_children();

    let sound_words = if choices.sound {
        SOUND_ON_WORDS
    } else {
        SOUND_OFF_WORDS
    };
    let sound_icon = if choices.sound {
        Icon::Volume2
    } else {
        Icon::VolumeX
    };
    commands
        .spawn((
            tool(sound_icon, sound_words.to_string(), ButtonKind::Quiet),
            ChildOf(row),
        ))
        .observe(|_: On<Activate>, mut choices: ResMut<RecordingChoices>| {
            choices.sound = flipped_sound(choices.sound, SOUND_RECORDABLE);
        });

    commands
        .spawn((
            tool(
                Icon::Film,
                frame_rate_words(choices.frame_rate),
                ButtonKind::Quiet,
            ),
            ChildOf(row),
        ))
        .observe(|_: On<Activate>, mut choices: ResMut<RecordingChoices>| {
            choices.frame_rate = other_frame_rate(choices.frame_rate);
        });

    // the timer and, while it is open, its list above it
    let timer_kind = if tools.list_open {
        ButtonKind::Choice
    } else {
        ButtonKind::Quiet
    };
    let timer = commands
        .spawn((
            Node {
                position_type: PositionType::Relative,
                ..default()
            },
            ChildOf(row),
        ))
        .id();
    commands
        .spawn((
            tool_with_chevron(Icon::Timer, timer_words(choices.timer_secs), timer_kind),
            ChildOf(timer),
        ))
        .observe(|_: On<Activate>, mut tools: Single<&mut BarTools>| {
            tools.list_open = !tools.list_open;
        });
    if tools.list_open {
        let list = commands
            .spawn((
                Node {
                    position_type: PositionType::Absolute,
                    bottom: px(LIST_BOTTOM_PX),
                    left: px(0),
                    min_width: px(LIST_MIN_WIDTH_PX),
                    flex_direction: FlexDirection::Column,
                    padding: UiRect::all(px(LIST_PADDING_PX)),
                    border: UiRect::all(px(BORDER_PX)),
                    border_radius: BorderRadius::all(px(LIST_RADIUS_PX)),
                    ..default()
                },
                card_glass(),
                ChildOf(timer),
            ))
            .id();
        for secs in TIMER_STEPS_SECS {
            commands
                .spawn((timer_row(secs, secs == choices.timer_secs), ChildOf(list)))
                .observe(
                    move |_: On<Activate>,
                          mut choices: ResMut<RecordingChoices>,
                          mut tools: Single<&mut BarTools>| {
                        choices.timer_secs = secs;
                        tools.list_open = false;
                    },
                );
        }
    }

    commands.spawn((
        Node {
            width: px(BORDER_PX),
            height: px(DIVIDER_HEIGHT_PX),
            ..default()
        },
        BackgroundColor(BORDER),
        ChildOf(row),
    ));

    let legend = bindings
        .binding(CAPTURE)
        .map(binding_legend)
        .unwrap_or_default();
    commands
        .spawn((
            laid_out_button(
                tool_node(),
                ButtonKind::Primary,
                (
                    tool_parts(),
                    children![
                        icon(Icon::CircleDot),
                        (
                            Text::new(RECORD_WORDS),
                            font(BODY_FONT_SIZE_PX),
                            TextColor(TEXT),
                        ),
                        keycap(CAPTURE, legend),
                    ],
                ),
            ),
            ChildOf(row),
        ))
        .observe(
            |_: On<Activate>,
             choices: Res<RecordingChoices>,
             mut next_state: ResMut<NextState<RecordingState>>| {
                next_state.set_if_different(start_target(choices.timer_secs));
            },
        );
}

/// Closes the bar when Esc dismisses it, without recording.
fn dismiss_bar(
    dismiss: On<Dismiss>,
    bars: Query<(), With<RecordingBar>>,
    mut next_state: ResMut<NextState<RecordingState>>,
) {
    if bars.contains(dismiss.entity) {
        next_state.set_if_different(RecordingState::Off);
    }
}

// ==== Helpers ================================================================

// ---- the tools --------------------------------------------------------------

/// Returns a tool of the bar: its icon beside its words, on a capsule of `kind`.
fn tool(symbol: Icon, words: String, kind: ButtonKind) -> impl Bundle {
    laid_out_button(
        tool_node(),
        kind,
        (
            tool_parts(),
            children![
                icon(symbol),
                (Text::new(words), font(BODY_FONT_SIZE_PX), TextColor(TEXT)),
            ],
        ),
    )
}

/// Returns a tool that opens a list: as `tool`, with a chevron after its words.
fn tool_with_chevron(symbol: Icon, words: String, kind: ButtonKind) -> impl Bundle {
    laid_out_button(
        tool_node(),
        kind,
        (
            tool_parts(),
            children![
                icon(symbol),
                (Text::new(words), font(BODY_FONT_SIZE_PX), TextColor(TEXT)),
                icon(Icon::ChevronDown),
            ],
        ),
    )
}

/// Returns the shape of a tool: a capsule as tall as the bar's tools are.
fn tool_node() -> Node {
    Node {
        height: px(TOOL_HEIGHT_PX),
        padding: UiRect::horizontal(px(TOOL_PADDING_X_PX)),
        align_items: AlignItems::Center,
        border_radius: BorderRadius::MAX,
        ..default()
    }
}

/// Returns the row a tool's icon and words sit in.
fn tool_parts() -> Node {
    Node {
        align_items: AlignItems::Center,
        column_gap: px(TOOL_PART_GAP_PX),
        ..default()
    }
}

/// Returns a row of the timer's list: a check beside the step the timer is on, then its
/// words.
fn timer_row(secs: u32, chosen: bool) -> impl Bundle {
    let check = if chosen {
        Visibility::Inherited
    } else {
        Visibility::Hidden
    };
    laid_out_button(
        Node {
            height: px(ROW_HEIGHT_PX),
            padding: UiRect::horizontal(px(TOOL_PADDING_X_PX)),
            align_items: AlignItems::Center,
            column_gap: px(TOOL_PART_GAP_PX),
            border_radius: BorderRadius::MAX,
            ..default()
        },
        ButtonKind::Quiet,
        (
            tool_parts(),
            children![
                (
                    Node {
                        width: px(CHECK_SIZE_PX),
                        ..default()
                    },
                    children![(icon(Icon::Check), check)],
                ),
                (
                    Text::new(timer_row_words(secs)),
                    font(BODY_FONT_SIZE_PX),
                    TextColor(TEXT),
                ),
            ],
        ),
    )
}

// ---- the choices ------------------------------------------------------------

/// Returns the sound after a click: flipped, or left off where there is no sound to record.
fn flipped_sound(sound: bool, recordable: bool) -> bool {
    recordable && !sound
}

/// Returns the frame rate after a click: the other of the two.
fn other_frame_rate(frame_rate: u32) -> u32 {
    FRAME_RATES
        .into_iter()
        .find(|rate| *rate != frame_rate)
        .unwrap_or(frame_rate)
}

/// Returns how the bar writes a frame rate: `60 fps`.
fn frame_rate_words(frame_rate: u32) -> String {
    format!("{frame_rate} fps")
}

/// Returns how the timer's tool writes its step: `3 s`, or None.
fn timer_words(secs: u32) -> String {
    if secs == 0 {
        NO_TIMER_WORDS.to_string()
    } else {
        format!("{secs} s")
    }
}

/// Returns how the timer's list writes a step: `3 seconds`, or None.
fn timer_row_words(secs: u32) -> String {
    if secs == 0 {
        NO_TIMER_WORDS.to_string()
    } else {
        format!("{secs} seconds")
    }
}

// ==== Tests ==================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_click_swaps_the_two_frame_rates() {
        assert_eq!(other_frame_rate(FRAME_RATES[0]), FRAME_RATES[1]);
        assert_eq!(other_frame_rate(FRAME_RATES[1]), FRAME_RATES[0]);
    }

    #[test]
    fn the_sound_flips_where_it_can_be_recorded() {
        assert!(flipped_sound(false, true));
        assert!(!flipped_sound(true, true));
    }

    #[test]
    fn the_sound_stays_off_where_there_is_none_to_record() {
        assert!(!flipped_sound(false, false));
    }

    #[test]
    fn no_timer_reads_none_on_the_tool_and_in_the_list() {
        assert_eq!(timer_words(0), NO_TIMER_WORDS);
        assert_eq!(timer_row_words(0), NO_TIMER_WORDS);
        assert_eq!(timer_words(FIRST_TIMER_SECS), "3 s");
        assert_eq!(timer_row_words(FIRST_TIMER_SECS), "3 seconds");
    }
}
