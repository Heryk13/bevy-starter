use bevy::audio::Volume;
use bevy::prelude::*;
use bevy::ui::Checked;
use bevy::ui_widgets::{
    Checkbox, Slider, SliderRange, SliderThumb, SliderValue, TrackClick, ValueChange,
};
use bevy::window::WindowMode;

use super::CloseOverlay;
use super::Overlay;
use super::overlay_root;
use crate::ui::button;

// ==== Config =================================================================

// ---- layout -----------------------------------------------------------------
const LABEL_GAP_PX: f32 = 12.0;

// ---- fullscreen checkbox ----------------------------------------------------
const CHECKBOX_SIZE_PX: f32 = 24.0;
const CHECKBOX_CHECKED_COLOR: Color = Color::WHITE;
const CHECKBOX_UNCHECKED_COLOR: Color = Color::srgb(1.0, 0.0, 0.0);

// ---- volume slider: the thumb travels the track minus its own width ---------
const SLIDER_WIDTH_PX: f32 = 200.0;
const SLIDER_HEIGHT_PX: f32 = 8.0;
const SLIDER_THUMB_SIZE_PX: f32 = 16.0;
const SLIDER_TRACK_COLOR: Color = Color::srgb(0.5, 0.5, 0.5);
const SLIDER_THUMB_COLOR: Color = Color::WHITE;
// 100 and not 1.0: `SliderStep` defaults to 1.0, so each arrow key moves the slider
// by a hundredth of its range instead of across all of it
const SLIDER_MAX: f32 = 100.0;

// ==== Plugin =================================================================

pub(super) fn plugin(app: &mut App) {
    app.add_systems(OnEnter(Overlay::Settings), spawn_settings_overlay);
    app.add_observer(toggle_fullscreen);
    app.add_observer(update_volume);
    app.add_systems(
        Update,
        (update_fullscreen_checkbox, update_volume_thumb).run_if(in_state(Overlay::Settings)),
    );
}

// ==== Components =============================================================

/// Marks the checkbox that switches the window in and out of fullscreen.
#[derive(Component)]
struct FullscreenToggle;

/// Marks the slider that sets the game's volume.
#[derive(Component)]
struct VolumeSlider;

// ==== Systems ================================================================

/// Spawns the settings panel over the current screen, blocking clicks to it.
fn spawn_settings_overlay(mut commands: Commands, global_volume: Res<GlobalVolume>) {
    let slider_value = slider_from_volume(global_volume.volume.to_linear());
    commands.spawn((
        overlay_root(),
        children![
            Text::new("Settings"),
            (
                Node {
                    flex_direction: FlexDirection::Row,
                    column_gap: px(LABEL_GAP_PX),
                    ..default()
                },
                children![
                    Text::new("Fullscreen"),
                    (
                        Node {
                            width: px(CHECKBOX_SIZE_PX),
                            height: px(CHECKBOX_SIZE_PX),
                            ..default()
                        },
                        Checkbox,
                        BackgroundColor(CHECKBOX_UNCHECKED_COLOR),
                        FullscreenToggle,
                        AccessibleLabel::new("Fullscreen"),
                    ),
                ]
            ),
            (
                Node {
                    flex_direction: FlexDirection::Row,
                    column_gap: px(LABEL_GAP_PX),
                    align_items: AlignItems::Center,
                    ..default()
                },
                children![
                    Text::new("Volume"),
                    (
                        Node {
                            width: px(SLIDER_WIDTH_PX),
                            height: px(SLIDER_HEIGHT_PX),
                            ..default()
                        },
                        BackgroundColor(SLIDER_TRACK_COLOR),
                        Slider {
                            track_click: TrackClick::Snap,
                            ..default()
                        },
                        SliderRange::new(0.0, SLIDER_MAX),
                        SliderValue(slider_value),
                        AccessibleLabel::new("Volume"),
                        children![(
                            Node {
                                width: px(SLIDER_THUMB_SIZE_PX),
                                height: px(SLIDER_THUMB_SIZE_PX),
                                top: px((SLIDER_HEIGHT_PX - SLIDER_THUMB_SIZE_PX) / 2.0),
                                position_type: PositionType::Absolute,
                                ..default()
                            },
                            BackgroundColor(SLIDER_THUMB_COLOR),
                            SliderThumb
                        )],
                        VolumeSlider
                    ),
                ],
            ),
            (button("Close"), CloseOverlay),
        ],
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

/// Paints the fullscreen checkbox and sets its checked state from the window mode.
fn update_fullscreen_checkbox(
    mut commands: Commands,
    checkbox: Single<(Entity, &mut BackgroundColor), With<FullscreenToggle>>,
    window: Single<&Window>,
) {
    let (entity, mut background_color) = checkbox.into_inner();
    if window.mode == WindowMode::Windowed {
        commands.entity(entity).remove::<Checked>();
        background_color.0 = CHECKBOX_UNCHECKED_COLOR;
    } else {
        commands.entity(entity).insert(Checked);
        background_color.0 = CHECKBOX_CHECKED_COLOR;
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

/// Moves the volume slider's thumb to match its value, since the widget draws nothing.
fn update_volume_thumb(
    slider: Single<(&SliderValue, &SliderRange), With<VolumeSlider>>,
    mut thumb: Single<&mut Node, With<SliderThumb>>,
) {
    let (slider_value, slider_range) = slider.into_inner();
    let thumb_position = slider_range.thumb_position(slider_value.0);
    let distance = SLIDER_WIDTH_PX - SLIDER_THUMB_SIZE_PX;
    thumb.left = px(thumb_position * distance);
}

// ==== Helpers ================================================================

/// Returns the mode the fullscreen toggle switches to: windowed goes fullscreen,
/// anything else goes back to a window.
fn toggled_window_mode(mode: WindowMode) -> WindowMode {
    match mode {
        WindowMode::Windowed => WindowMode::BorderlessFullscreen(MonitorSelection::Current),
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

// ==== Tests ==================================================================

#[cfg(test)]
mod tests {
    use super::*;

    const TOLERANCE: f32 = 1e-5;

    #[test]
    fn windowed_becomes_borderless_fullscreen() {
        assert_eq!(
            toggled_window_mode(WindowMode::Windowed),
            WindowMode::BorderlessFullscreen(MonitorSelection::Current)
        );
    }

    #[test]
    fn fullscreen_becomes_windowed() {
        assert_eq!(
            toggled_window_mode(WindowMode::BorderlessFullscreen(MonitorSelection::Current)),
            WindowMode::Windowed
        );
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
}
