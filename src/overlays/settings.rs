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
use crate::ui::{
    ACCENT, BODY_FONT_SIZE_PX, BORDER, BORDER_PX, FIELD_FILL, TEXT, TEXT_MUTED, font, heading,
    label,
};

// ==== Config =================================================================

// ---- layout: a grid, the labels in a column of their own --------------------
const LABEL_WIDTH_PX: f32 = 140.0;
const CONTROL_GAP_PX: f32 = 12.0;

// ---- fullscreen checkbox ----------------------------------------------------
// borderless, on the monitor the window is on: it switches instantly and never
// changes the desktop resolution
const FULLSCREEN_MODE: WindowMode = WindowMode::BorderlessFullscreen(MonitorSelection::Current);
const CHECKBOX_SIZE_PX: f32 = 22.0;
const CHECKBOX_RADIUS_PX: f32 = 4.0;

// ---- volume slider: the thumb travels the track minus its own width ---------
// the slider is as tall as its thumb, so the whole height takes a click; the track is a
// thin line through its middle
const SLIDER_WIDTH_PX: f32 = 220.0;
const SLIDER_THUMB_SIZE_PX: f32 = 16.0;
const SLIDER_TRACK_HEIGHT_PX: f32 = 4.0;
// 100 and not 1.0: `SliderStep` defaults to 1.0, so each arrow key moves the slider
// by a hundredth of its range instead of across all of it
const SLIDER_MAX: f32 = 100.0;

// ---- language radios: an empty ring until chosen, then filled ---------------
const RADIO_SIZE_PX: f32 = 16.0;
const RADIO_RING_PX: f32 = 2.0;

// ---- file: a different version is ignored and the defaults are kept ---------
const SETTINGS_VERSION: u32 = 1;
const SETTINGS_FILE_NAME: &str = "settings.ron";

// ==== Plugin =================================================================

pub(super) fn plugin(app: &mut App) {
    app.add_systems(Startup, load_settings);
    app.add_systems(OnEnter(Overlay::Settings), spawn_settings_overlay);
    app.add_observer(toggle_fullscreen.run_if(in_state(Overlay::Settings)));
    app.add_observer(update_volume.run_if(in_state(Overlay::Settings)));
    app.add_observer(choose_language.run_if(in_state(Overlay::Settings)));
    app.add_systems(
        Update,
        (
            update_fullscreen_checkbox,
            update_volume_slider,
            update_language_radios,
        )
            .run_if(in_state(Overlay::Settings)),
    );
    app.add_systems(OnExit(Overlay::Settings), save_settings);
}

// ==== Components =============================================================

/// Marks the checkbox that switches the window in and out of fullscreen.
#[derive(Component)]
struct FullscreenToggle;

/// Marks the slider that sets the game's volume.
#[derive(Component)]
struct VolumeSlider;

/// Marks the part of the volume slider's track before its thumb, filled with the accent.
#[derive(Component)]
struct VolumeFill;

/// Marks a language radio button with the language it chooses.
#[derive(Component)]
struct LanguageOption(Language);

/// Marks the ring inside a language radio button, filled to show it is chosen.
#[derive(Component)]
struct RadioMark;

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
}

// ==== Systems ================================================================

/// Applies the remembered settings to the window, the volume and the language. Anything
/// missing, unreadable or from another version keeps the defaults, silently: it is a
/// preference. A language never chosen follows the OS.
fn load_settings(
    mut window: Single<&mut Window>,
    mut global_volume: ResMut<GlobalVolume>,
    mut language: ResMut<Language>,
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
                        Node {
                            width: px(CHECKBOX_SIZE_PX),
                            height: px(CHECKBOX_SIZE_PX),
                            border: UiRect::all(px(BORDER_PX)),
                            border_radius: BorderRadius::all(px(CHECKBOX_RADIUS_PX)),
                            ..default()
                        },
                        Checkbox,
                        BackgroundColor(FIELD_FILL),
                        BorderColor::all(BORDER),
                        FullscreenToggle,
                        AccessibleLabel::new("Fullscreen"),
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
                            (slider_track(percent(100)), BackgroundColor(BORDER)),
                            (slider_track(px(0)), BackgroundColor(ACCENT), VolumeFill),
                            (
                                Node {
                                    width: px(SLIDER_THUMB_SIZE_PX),
                                    height: px(SLIDER_THUMB_SIZE_PX),
                                    position_type: PositionType::Absolute,
                                    border_radius: BorderRadius::MAX,
                                    ..default()
                                },
                                BackgroundColor(TEXT),
                                SliderThumb,
                            ),
                        ],
                    ),
                ),
                setting_row(
                    "settings-language",
                    (
                        Node {
                            column_gap: px(CONTROL_GAP_PX),
                            ..default()
                        },
                        RadioGroup,
                        Children::spawn(SpawnIter(Language::ALL.into_iter().map(language_radio))),
                    ),
                ),
                close_footer(),
            ],
        )],
        DespawnOnExit(Overlay::Settings),
    ));
}

/// Switches the window in or out of fullscreen when the fullscreen checkbox is clicked.
fn toggle_fullscreen(
    value_change: On<ValueChange<bool>>,
    fullscreen_toggles: Query<(), With<FullscreenToggle>>,
    mut window: Single<&mut Window>,
) {
    if fullscreen_toggles.contains(value_change.source) {
        window.mode = toggled_window_mode(window.mode);
    }
}

/// Paints the fullscreen checkbox and sets its checked state from the window mode: filled
/// with the accent when on, an empty box when off.
fn update_fullscreen_checkbox(
    mut commands: Commands,
    checkbox: Single<(Entity, &mut BackgroundColor, &mut BorderColor), With<FullscreenToggle>>,
    window: Single<&Window>,
) {
    let (entity, mut fill, mut border) = checkbox.into_inner();
    if window.mode == WindowMode::Windowed {
        commands.entity(entity).remove::<Checked>();
        fill.0 = FIELD_FILL;
        *border = BorderColor::all(BORDER);
    } else {
        commands.entity(entity).insert(Checked);
        fill.0 = ACCENT;
        *border = BorderColor::all(ACCENT);
    }
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

/// Switches the game to the language whose radio button was clicked. Every `Localized`
/// text follows on its own, the panel's own labels and the menu behind it included.
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

/// Checks the language radio button that matches the current language and fills its ring,
/// unchecking and emptying the others, since the radio group leaves that to the app.
fn update_language_radios(
    mut commands: Commands,
    language: Res<Language>,
    radios: Query<(Entity, &LanguageOption, &Children)>,
    mut marks: Query<(&mut BackgroundColor, &mut BorderColor), With<RadioMark>>,
) {
    for (entity, LanguageOption(option), children) in &radios {
        let chosen = *option == *language;
        if chosen {
            commands.entity(entity).insert(Checked);
        } else {
            commands.entity(entity).remove::<Checked>();
        }
        for child in children {
            if let Ok((mut fill, mut ring)) = marks.get_mut(*child) {
                let (fill_color, ring_color) = if chosen {
                    (ACCENT, ACCENT)
                } else {
                    (Color::NONE, TEXT_MUTED)
                };
                fill.0 = fill_color;
                *ring = BorderColor::all(ring_color);
            }
        }
    }
}

/// Writes the window mode, the volume and the language to `settings.ron` as the panel
/// closes, once per visit rather than on every slider move.
fn save_settings(
    window: Single<&Window>,
    global_volume: Res<GlobalVolume>,
    language: Res<Language>,
) {
    let settings = SettingsFile {
        version: SETTINGS_VERSION,
        fullscreen: window.mode != WindowMode::Windowed,
        volume: global_volume.volume.to_linear(),
        language: Some(*language),
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

/// Returns the radio button that chooses `language`: a dot, then the language's name in
/// itself.
fn language_radio(language: Language) -> impl Bundle {
    (
        Node {
            column_gap: px(CONTROL_GAP_PX / 2.0),
            align_items: AlignItems::Center,
            ..default()
        },
        RadioButton,
        LanguageOption(language),
        children![
            (
                Node {
                    width: px(RADIO_SIZE_PX),
                    height: px(RADIO_SIZE_PX),
                    border: UiRect::all(px(RADIO_RING_PX)),
                    border_radius: BorderRadius::MAX,
                    ..default()
                },
                BorderColor::all(TEXT_MUTED),
                RadioMark,
            ),
            (
                Text::new(language.native_name()),
                font(BODY_FONT_SIZE_PX),
                TextColor(TEXT),
            ),
        ],
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
    fn unreadable_settings_are_ignored() {
        assert!(parse_settings("not ron").is_none());
    }
}
