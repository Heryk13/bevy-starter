use bevy::prelude::*;
use bevy::ui::Checked;
use bevy::ui_widgets::Checkbox;
use bevy::ui_widgets::ValueChange;
use bevy::window::WindowMode;

use super::CloseOverlay;
use super::OVERLAY_Z;
use super::Overlay;
use crate::ui::button;

// ==== Config =================================================================

const CHECKBOX_CHECKED_COLOR: Color = Color::srgb(1.0, 1.0, 1.0);
const CHECKBOX_UNCHECKED_COLOR: Color = Color::srgb(1.0, 0.0, 0.0);

const CHECKBOX_SIZE_PX: f32 = 24.0;
const ROW_GAP_PX: f32 = 12.0;

// ==== Plugin =================================================================

pub(super) fn plugin(app: &mut App) {
    app.add_systems(OnEnter(Overlay::Settings), spawn_settings_overlay);
    app.add_observer(toggle_fullscreen);
    app.add_systems(
        Update,
        update_fullscreen_checkbox.run_if(in_state(Overlay::Settings)),
    );
}

// ==== Components =============================================================

/// Marks the checkbox that switches the window in and out of fullscreen.
#[derive(Component)]
struct FullscreenToggle;

// ==== Systems ================================================================

/// Spawns the settings panel over the current screen, blocking clicks to it.
fn spawn_settings_overlay(mut commands: Commands) {
    commands.spawn((
        Node {
            width: percent(100),
            height: percent(100),
            flex_direction: FlexDirection::Column,
            justify_content: JustifyContent::Center,
            align_items: AlignItems::Center,
            ..default()
        },
        children![
            Text::new("Settings"),
            (
                Node {
                    flex_direction: FlexDirection::Row,
                    column_gap: px(ROW_GAP_PX),
                    ..default()
                },
                children![
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
                    Text::new("Fullscreen"),
                ]
            ),
            (button("Close"), CloseOverlay),
        ],
        OVERLAY_Z,
        DespawnOnExit(Overlay::Settings),
    ));
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

// ==== Helpers ================================================================

/// Returns the mode the fullscreen toggle switches to: windowed goes fullscreen,
/// anything else goes back to a window.
fn toggled_window_mode(mode: WindowMode) -> WindowMode {
    match mode {
        WindowMode::Windowed => WindowMode::BorderlessFullscreen(MonitorSelection::Current),
        _ => WindowMode::Windowed,
    }
}

// ==== Tests ==================================================================

#[cfg(test)]
mod tests {
    use super::*;

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
}
