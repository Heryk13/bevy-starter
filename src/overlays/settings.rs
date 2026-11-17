use bevy::audio::Volume;
use bevy::prelude::*;
use bevy::ui::Checked;
use bevy::ui_widgets::{
    Checkbox, RadioButton, RadioGroup, Slider, SliderRange, SliderThumb, SliderValue, TrackClick,
    ValueChange,
};
use bevy::window::WindowMode;
use serde::{Deserialize, Serialize};

use super::{Overlay, close_footer, overlay_card, overlay_root};
use crate::disk::{game_data_dir, write_atomic};
use crate::i18n::{Language, language_from_locale};
use crate::ui::blur::ReduceTransparency;
use crate::ui::{ACCENT, FIELD_FILL, INK, SMALL_FONT_SIZE_PX, TEXT_MUTED, font, heading, label};

// ==== Config =================================================================

// ---- layout: a grid, the labels in a column of their own --------------------
// wide enough for the longest label, Reduce Transparency, to stay on one line
const LABEL_WIDTH_PX: f32 = 190.0;

// ---- colors: the palette lives in ui.rs, this only paints the controls ------
// the empty part of a control: a switch that is off, a slider's track past its thumb
const CONTROL_TRACK: Color = Color::srgba(1.0, 1.0, 1.0, 0.16);

// ---- fullscreen -------------------------------------------------------------
// borderless, on the monitor the window is on: it switches instantly and never
// changes the desktop resolution
const FULLSCREEN_MODE: WindowMode = WindowMode::BorderlessFullscreen(MonitorSelection::Current);

// ---- switches: fullscreen and reduce transparency ---------------------------
const SWITCH_WIDTH_PX: f32 = 46.0;
const SWITCH_HEIGHT_PX: f32 = 26.0;
const SWITCH_KNOB_SIZE_PX: f32 = 20.0;

// ---- volume slider: the thumb travels the track minus its own width ---------
// the slider is as tall as its thumb, so the whole height takes a click; the track is a
// thin line through its middle
const SLIDER_WIDTH_PX: f32 = 220.0;
const SLIDER_THUMB_SIZE_PX: f32 = 18.0;
const SLIDER_TRACK_HEIGHT_PX: f32 = 6.0;
// 100 and not 1.0: `SliderStep` defaults to 1.0, so each arrow key moves the slider
// by a hundredth of its range instead of across all of it
const SLIDER_MAX: f32 = 100.0;

// ---- language segments: one capsule track, the chosen language filled -------
const SEGMENT_HEIGHT_PX: f32 = 32.0;
const SEGMENT_PADDING_PX: f32 = 14.0;
// between the track's edge and its segments, so the chosen one floats inside it
const SEGMENT_INSET_PX: f32 = 3.0;

// ---- file: a different version is ignored and the defaults are kept ---------
const SETTINGS_VERSION: u32 = 1;
const SETTINGS_FILE_NAME: &str = "settings.ron";

// ---- derived: where a switch's knob rests -----------------------------------
// the same gap on every side of the knob, whichever end it is at: the inset is how far
// from the track's left edge the knob rests when off, the offset when on
const SWITCH_KNOB_INSET_PX: f32 = (SWITCH_HEIGHT_PX - SWITCH_KNOB_SIZE_PX) / 2.0;
const SWITCH_KNOB_ON_OFFSET_PX: f32 = SWITCH_WIDTH_PX - SWITCH_KNOB_SIZE_PX - SWITCH_KNOB_INSET_PX;

// ==== Plugin =================================================================

pub(super) fn plugin(app: &mut App) {
    app.add_systems(Startup, load_settings);
    app.add_systems(OnEnter(Overlay::Settings), spawn_settings_overlay);
    app.add_observer(toggle_fullscreen.run_if(in_state(Overlay::Settings)));
    app.add_observer(toggle_reduce_transparency.run_if(in_state(Overlay::Settings)));
    app.add_observer(update_volume.run_if(in_state(Overlay::Settings)));
    app.add_observer(choose_language.run_if(in_state(Overlay::Settings)));
    app.add_systems(
        Update,
        (
            update_fullscreen_switch,
            update_reduce_transparency_switch,
            update_volume_slider,
            update_language_segments,
        )
            .run_if(in_state(Overlay::Settings)),
    );
    app.add_systems(OnExit(Overlay::Settings), save_settings);
}

// ==== Components =============================================================

/// Marks the switch that puts the window in and out of fullscreen.
#[derive(Component)]
struct FullscreenToggle;

/// Marks the switch that turns the glass into flat fills, with no blur behind them.
#[derive(Component)]
struct ReduceTransparencyToggle;

/// Marks the knob of a switch, which rests at its left end when off and at its right when on.
#[derive(Component)]
struct SwitchKnob;

/// Marks the slider that sets the game's volume.
#[derive(Component)]
struct VolumeSlider;

/// Marks the part of the volume slider's track before its thumb, filled with the accent.
#[derive(Component)]
struct VolumeFill;

/// Marks a segment of the language control with the language it chooses.
#[derive(Component)]
struct LanguageOption(Language);

// ==== Settings File ==========================================================

/// The preferences remembered between runs, as `settings.ron` holds them.
#[derive(Serialize, Deserialize)]
struct SettingsFile {
    version: u32,
    fullscreen: bool,
    volume: f32,
    // `None` follows the OS. The default lets a file from before languages existed load
    #[serde(default)]
    language: Option<Language>,
    // off in a file from before the glass existed, which the default lets load
    #[serde(default)]
    reduce_transparency: bool,
}

// ==== Systems ================================================================

/// Applies the remembered settings to the window, the volume, the language and the glass.
/// Anything missing, unreadable or from another version keeps the defaults, silently: it is
/// a preference. A language never chosen follows the OS.
fn load_settings(
    mut window: Single<&mut Window>,
    mut global_volume: ResMut<GlobalVolume>,
    mut language: ResMut<Language>,
    mut reduce_transparency: ResMut<ReduceTransparency>,
) {
    let settings = game_data_dir()
        .and_then(|dir| std::fs::read_to_string(dir.join(SETTINGS_FILE_NAME)).ok())
        .and_then(|text| parse_settings(&text));
    *language = settings
        .as_ref()
        .and_then(|settings| settings.language)
        .or_else(|| sys_locale::get_locale().map(|locale| language_from_locale(&locale)))
        .unwrap_or_default();
    let Some(settings) = settings else {
        return;
    };
    // clamped: the file can be edited by hand, and the slider only reaches full volume
    global_volume.volume = Volume::Linear(settings.volume.clamp(0.0, 1.0));
    window.mode = if settings.fullscreen {
        FULLSCREEN_MODE
    } else {
        WindowMode::Windowed
    };
    reduce_transparency.0 = settings.reduce_transparency;
}

/// Spawns the settings panel over the current screen, blocking clicks to it.
fn spawn_settings_overlay(mut commands: Commands, global_volume: Res<GlobalVolume>) {
    let slider_value = slider_from_volume(global_volume.volume.to_linear());
    commands.spawn((
        overlay_root(),
        children![(
            overlay_card(),
            children![
                heading("settings-title"),
                setting_row(
                    "settings-fullscreen",
                    (
                        switch(),
                        FullscreenToggle,
                        AccessibleLabel::new("Fullscreen"),
                    ),
                ),
                setting_row(
                    "settings-reduce-transparency",
                    (
                        switch(),
                        ReduceTransparencyToggle,
                        AccessibleLabel::new("Reduce Transparency"),
                    ),
                ),
                setting_row(
                    "settings-volume",
                    (
                        Node {
                            width: px(SLIDER_WIDTH_PX),
                            height: px(SLIDER_THUMB_SIZE_PX),
                            ..default()
                        },
                        Slider {
                            track_click: TrackClick::Snap,
                            ..default()
                        },
                        SliderRange::new(0.0, SLIDER_MAX),
                        SliderValue(slider_value),
                        AccessibleLabel::new("Volume"),
                        VolumeSlider,
                        children![
                            (slider_track(percent(100)), BackgroundColor(CONTROL_TRACK)),
                            (slider_track(px(0)), BackgroundColor(ACCENT), VolumeFill),
                            (
                                Node {
                                    width: px(SLIDER_THUMB_SIZE_PX),
                                    height: px(SLIDER_THUMB_SIZE_PX),
                                    position_type: PositionType::Absolute,
                                    border_radius: BorderRadius::MAX,
                                    ..default()
                                },
                                BackgroundColor(ACCENT),
                                SliderThumb,
                            ),
                        ],
                    ),
                ),
                setting_row(
                    "settings-language",
                    // ponytail: a segment per language, side by side, fits three or four
                    // names in the card's width. a game with more needs a list in its place
                    (
                        Node {
                            padding: UiRect::all(px(SEGMENT_INSET_PX)),
                            border_radius: BorderRadius::MAX,
                            ..default()
                        },
                        BackgroundColor(FIELD_FILL),
                        RadioGroup,
                        Children::spawn(SpawnIter(Language::ALL.into_iter().map(language_segment))),
                    ),
                ),
                close_footer(),
            ],
        )],
        DespawnOnExit(Overlay::Settings),
    ));
}

/// Puts the window in or out of fullscreen when the fullscreen switch is clicked.
fn toggle_fullscreen(
    value_change: On<ValueChange<bool>>,
    fullscreen_toggles: Query<(), With<FullscreenToggle>>,
    mut window: Single<&mut Window>,
) {
    if fullscreen_toggles.contains(value_change.source) {
        window.mode = toggled_window_mode(window.mode);
    }
}

/// Turns Reduce Transparency on or off when its switch is clicked. The glass follows at
/// once, this panel's own included.
fn toggle_reduce_transparency(
    value_change: On<ValueChange<bool>>,
    reduce_transparency_toggles: Query<(), With<ReduceTransparencyToggle>>,
    mut reduce_transparency: ResMut<ReduceTransparency>,
) {
    if reduce_transparency_toggles.contains(value_change.source) {
        reduce_transparency.0 = !reduce_transparency.0;
    }
}

/// Keeps the fullscreen switch on while the window is fullscreen, and off otherwise.
fn update_fullscreen_switch(
    mut commands: Commands,
    switch: Single<SwitchParts, (With<FullscreenToggle>, Without<SwitchKnob>)>,
    mut knobs: Query<(&mut Node, &mut BackgroundColor), With<SwitchKnob>>,
    window: Single<&Window>,
) {
    let on = window.mode != WindowMode::Windowed;
    set_switch(&mut commands, switch.into_inner(), &mut knobs, on);
}

/// Keeps the Reduce Transparency switch on while the setting is, and off otherwise.
fn update_reduce_transparency_switch(
    mut commands: Commands,
    switch: Single<SwitchParts, (With<ReduceTransparencyToggle>, Without<SwitchKnob>)>,
    mut knobs: Query<(&mut Node, &mut BackgroundColor), With<SwitchKnob>>,
    reduce_transparency: Res<ReduceTransparency>,
) {
    set_switch(
        &mut commands,
        switch.into_inner(),
        &mut knobs,
        reduce_transparency.0,
    );
}

/// Sets the game's volume from the volume slider, and stores the slider's new value,
/// since the widget only reports a drag and never updates itself.
fn update_volume(
    value_change: On<ValueChange<f32>>,
    volume_sliders: Query<(), With<VolumeSlider>>,
    mut global_volume: ResMut<GlobalVolume>,
    mut commands: Commands,
) {
    if volume_sliders.contains(value_change.source) {
        global_volume.volume = Volume::Linear(volume_from_slider(value_change.value));
        commands
            .entity(value_change.source)
            .insert(SliderValue(value_change.value));
    }
}

/// Moves the volume slider's thumb to match its value, and fills the track up to it, since
/// the widget draws nothing.
fn update_volume_slider(
    slider: Single<(&SliderValue, &SliderRange), With<VolumeSlider>>,
    mut thumb: Single<&mut Node, (With<SliderThumb>, Without<VolumeFill>)>,
    mut fill: Single<&mut Node, (With<VolumeFill>, Without<SliderThumb>)>,
) {
    let (slider_value, slider_range) = slider.into_inner();
    let thumb_position = slider_range.thumb_position(slider_value.0);
    let distance = SLIDER_WIDTH_PX - SLIDER_THUMB_SIZE_PX;
    thumb.left = px(thumb_position * distance);
    // to the thumb's middle, so the fill never shows past its edge or short of it
    fill.width = px(thumb_position * distance + SLIDER_THUMB_SIZE_PX / 2.0);
}

/// Switches the game to the language whose segment was clicked. Every `Localized` text
/// follows on its own, the panel's own labels and the menu behind it included.
fn choose_language(
    value_change: On<ValueChange<Entity>>,
    options: Query<&LanguageOption>,
    mut language: ResMut<Language>,
) {
    if let Ok(LanguageOption(chosen)) = options.get(value_change.value) {
        // set_if_neq: a change, even to the same value, would translate every text again
        language.set_if_neq(*chosen);
    }
}

/// Checks the segment of the current language and fills it, unchecking and emptying the
/// others, since the radio group leaves that to the app.
fn update_language_segments(
    mut commands: Commands,
    language: Res<Language>,
    mut segments: Query<(
        Entity,
        &LanguageOption,
        Has<Checked>,
        &mut BackgroundColor,
        &Children,
    )>,
    mut names: Query<&mut TextColor>,
) {
    for (entity, LanguageOption(option), checked, mut fill, children) in &mut segments {
        let chosen = *option == *language;
        // already as it should be, which is most frames: a segment spawns unchosen
        if checked == chosen {
            continue;
        }
        let (fill_color, ink) = if chosen {
            commands.entity(entity).insert(Checked);
            (ACCENT, INK)
        } else {
            commands.entity(entity).remove::<Checked>();
            (Color::NONE, TEXT_MUTED)
        };
        fill.0 = fill_color;
        for child in children {
            if let Ok(mut name) = names.get_mut(*child) {
                name.0 = ink;
            }
        }
    }
}

/// Writes the window mode, the volume, the language and the glass setting to `settings.ron`
/// as the panel closes, once per visit rather than on every slider move.
fn save_settings(
    window: Single<&Window>,
    global_volume: Res<GlobalVolume>,
    language: Res<Language>,
    reduce_transparency: Res<ReduceTransparency>,
) {
    let settings = SettingsFile {
        version: SETTINGS_VERSION,
        fullscreen: window.mode != WindowMode::Windowed,
        volume: global_volume.volume.to_linear(),
        language: Some(*language),
        reduce_transparency: reduce_transparency.0,
    };
    let Ok(text) = ron::to_string(&settings) else {
        return;
    };
    let Some(dir) = game_data_dir() else {
        return;
    };
    // a failed write is logged, never dropped: the player would believe it was saved
    if let Err(error) = write_atomic(&dir.join(SETTINGS_FILE_NAME), &text) {
        error!("could not save the settings: {error}");
    }
}

// ==== Helpers ================================================================

/// Returns one row of the settings grid: the label in its column, then the control.
fn setting_row(key: &'static str, control: impl Bundle) -> impl Bundle {
    (
        Node {
            align_items: AlignItems::Center,
            ..default()
        },
        children![
            (
                Node {
                    width: px(LABEL_WIDTH_PX),
                    ..default()
                },
                children![label(key)],
            ),
            control,
        ],
    )
}

/// Returns a switch that is off, for its marker and its accessible label to be added to. It
/// is a `Checkbox` drawn as a track and a knob, as Bevy's own toggle switch is. Its
/// `update_*_switch` system turns it on and paints it, since the widget does neither.
fn switch() -> impl Bundle {
    (
        Node {
            width: px(SWITCH_WIDTH_PX),
            height: px(SWITCH_HEIGHT_PX),
            border_radius: BorderRadius::MAX,
            ..default()
        },
        Checkbox,
        BackgroundColor(CONTROL_TRACK),
        children![(
            Node {
                width: px(SWITCH_KNOB_SIZE_PX),
                height: px(SWITCH_KNOB_SIZE_PX),
                position_type: PositionType::Absolute,
                left: px(SWITCH_KNOB_INSET_PX),
                top: px(SWITCH_KNOB_INSET_PX),
                border_radius: BorderRadius::MAX,
                ..default()
            },
            BackgroundColor(ACCENT),
            SwitchKnob,
        )],
    )
}

/// What a switch's system reads of it: the switch, whether it is on, its track and its
/// children, the knob among them.
type SwitchParts = (
    Entity,
    Has<Checked>,
    &'static mut BackgroundColor,
    &'static Children,
);

/// Turns a switch on or off and paints it to match: the accent with a dark knob at the right
/// when on, an empty track with a white knob at the left when off. Does nothing to a switch
/// that is already there, which is most frames.
fn set_switch(
    commands: &mut Commands,
    (entity, checked, mut track, children): (Entity, bool, Mut<BackgroundColor>, &Children),
    knobs: &mut Query<(&mut Node, &mut BackgroundColor), With<SwitchKnob>>,
    on: bool,
) {
    if checked == on {
        return;
    }
    let (track_color, knob_color, knob_left_px) = if on {
        commands.entity(entity).insert(Checked);
        (ACCENT, INK, SWITCH_KNOB_ON_OFFSET_PX)
    } else {
        commands.entity(entity).remove::<Checked>();
        (CONTROL_TRACK, ACCENT, SWITCH_KNOB_INSET_PX)
    };
    track.0 = track_color;
    for child in children {
        if let Ok((mut knob, mut fill)) = knobs.get_mut(*child) {
            knob.left = px(knob_left_px);
            fill.0 = knob_color;
        }
    }
}

/// Returns a stretch of the volume slider's track, `width` long from the left: the whole
/// track, or the filled part before the thumb. A thin line through the slider's middle.
fn slider_track(width: Val) -> impl Bundle {
    Node {
        width,
        height: px(SLIDER_TRACK_HEIGHT_PX),
        top: px((SLIDER_THUMB_SIZE_PX - SLIDER_TRACK_HEIGHT_PX) / 2.0),
        position_type: PositionType::Absolute,
        border_radius: BorderRadius::MAX,
        ..default()
    }
}

/// Returns the segment that chooses `language`, unchosen: the language's name in itself. It
/// is a `RadioButton`, so its group reports a click on it and the arrow keys move between
/// the segments.
fn language_segment(language: Language) -> impl Bundle {
    (
        Node {
            height: px(SEGMENT_HEIGHT_PX),
            padding: UiRect::horizontal(px(SEGMENT_PADDING_PX)),
            align_items: AlignItems::Center,
            border_radius: BorderRadius::MAX,
            ..default()
        },
        RadioButton,
        LanguageOption(language),
        BackgroundColor(Color::NONE),
        children![(
            Text::new(language.native_name()),
            font(SMALL_FONT_SIZE_PX),
            TextColor(TEXT_MUTED),
        )],
    )
}

/// Returns the mode the fullscreen toggle switches to: windowed goes fullscreen,
/// anything else goes back to a window.
fn toggled_window_mode(mode: WindowMode) -> WindowMode {
    match mode {
        WindowMode::Windowed => FULLSCREEN_MODE,
        _ => WindowMode::Windowed,
    }
}

/// Returns the linear volume for a slider position in 0..=`SLIDER_MAX`, squared so
/// the slider feels even to the ear instead of bunching all the change near zero.
fn volume_from_slider(slider_value: f32) -> f32 {
    (slider_value / SLIDER_MAX).powi(2)
}

/// Returns the slider position for a linear volume, undoing `volume_from_slider`.
fn slider_from_volume(volume: f32) -> f32 {
    volume.sqrt() * SLIDER_MAX
}

/// Returns the settings a file holds, or `None` when it is unreadable or from another
/// version, so the caller keeps the defaults.
fn parse_settings(text: &str) -> Option<SettingsFile> {
    ron::from_str::<SettingsFile>(text)
        .ok()
        .filter(|settings| settings.version == SETTINGS_VERSION)
}

// ==== Tests ==================================================================

#[cfg(test)]
mod tests {
    use super::*;

    const TOLERANCE: f32 = 1e-5;

    #[test]
    fn windowed_becomes_borderless_fullscreen() {
        assert_eq!(toggled_window_mode(WindowMode::Windowed), FULLSCREEN_MODE);
    }

    #[test]
    fn fullscreen_becomes_windowed() {
        assert_eq!(toggled_window_mode(FULLSCREEN_MODE), WindowMode::Windowed);
    }

    #[test]
    fn slider_at_zero_is_silent() {
        assert!((volume_from_slider(0.0) - 0.0).abs() < TOLERANCE);
    }

    #[test]
    fn slider_at_half_is_a_quarter_volume() {
        assert!((volume_from_slider(SLIDER_MAX / 2.0) - 0.25).abs() < TOLERANCE);
    }

    #[test]
    fn slider_at_full_is_full_volume() {
        assert!((volume_from_slider(SLIDER_MAX) - 1.0).abs() < TOLERANCE);
    }

    #[test]
    fn volume_at_zero_is_slider_at_zero() {
        assert!((slider_from_volume(0.0) - 0.0).abs() < TOLERANCE);
    }

    #[test]
    fn volume_at_quarter_is_slider_at_half() {
        assert!((slider_from_volume(0.25) - (SLIDER_MAX / 2.0)).abs() < TOLERANCE);
    }

    #[test]
    fn volume_at_full_is_slider_at_full() {
        assert!((slider_from_volume(1.0) - SLIDER_MAX).abs() < TOLERANCE);
    }

    #[test]
    fn settings_file_reads_back() {
        let text = format!("(version: {SETTINGS_VERSION}, fullscreen: true, volume: 0.25)");
        let settings = parse_settings(&text).unwrap();
        assert!(settings.fullscreen);
        assert!((settings.volume - 0.25).abs() < TOLERANCE);
    }

    #[test]
    fn settings_from_another_version_are_ignored() {
        let text = format!(
            "(version: {}, fullscreen: true, volume: 0.25)",
            SETTINGS_VERSION + 1
        );
        assert!(parse_settings(&text).is_none());
    }

    #[test]
    fn settings_without_a_language_follow_the_os() {
        let text = format!("(version: {SETTINGS_VERSION}, fullscreen: true, volume: 0.25)");
        assert_eq!(parse_settings(&text).unwrap().language, None);
    }

    #[test]
    fn chosen_language_reads_back() {
        let text = format!(
            "(version: {SETTINGS_VERSION}, fullscreen: true, volume: 0.25, language: Some(Japanese))"
        );
        assert_eq!(
            parse_settings(&text).unwrap().language,
            Some(Language::Japanese)
        );
    }

    #[test]
    fn settings_without_reduce_transparency_load_with_it_off() {
        let text = format!("(version: {SETTINGS_VERSION}, fullscreen: true, volume: 0.25)");
        assert!(!parse_settings(&text).unwrap().reduce_transparency);
    }

    #[test]
    fn reduce_transparency_reads_back() {
        let text = format!(
            "(version: {SETTINGS_VERSION}, fullscreen: true, volume: 0.25, reduce_transparency: true)"
        );
        assert!(parse_settings(&text).unwrap().reduce_transparency);
    }

    #[test]
    fn unreadable_settings_are_ignored() {
        assert!(parse_settings("not ron").is_none());
    }
}
