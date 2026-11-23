use bevy::input::mouse::{AccumulatedMouseScroll, MouseScrollUnit};
use bevy::picking::hover::Hovered;
use bevy::prelude::*;

use super::photo_mode::{PhotoHint, register_photo_hint};
use super::prop_editor::PropEditor;
use super::prop_editor::edits::{Held, PropsSheet};
use crate::input::Controls;
use crate::screens::Pause;
use crate::screens::gameplay::player::{MOVE_DOWN, MOVE_LEFT, MOVE_RIGHT, MOVE_UP, walk_input};
use crate::ui::Icon;

// ==== Config =================================================================

// ---- panning: on the screen, so a zoomed-out view covers more of the room ---
const FREE_CAMERA_PAN_PX_PER_S: f32 = 512.0;

// ---- zoom: the camera's orthographic scale, a notch of the wheel at a time --
// below 1 zooms in, above 1 out
const ZOOM_STEP: f32 = 1.1;
const ZOOM_MIN: f32 = 0.25;
const ZOOM_MAX: f32 = 4.0;
// a wheel or a touchpad that reports pixels: this many make a notch
const SCROLL_PX_PER_NOTCH: f32 = 100.0;

// ---- words: English, as every dev tool's, never read in a release build -----
const MOVE_WORDS: &str = "Move";
const ZOOM_WORDS: &str = "Zoom";

// ==== Plugin =================================================================

/// Photo mode's 2D free camera, and the prop editor's: the walk keys pan the screen camera and
/// the mouse wheel zooms it, to frame the photo or to build the map, and both come back as
/// photo mode or the editor ends. In the editor the wheel scales a prop in the hand instead,
/// and scrolls the props window under the pointer. It counts real time, since photo mode
/// freezes the game.
pub(super) fn plugin(app: &mut App) {
    register_photo_hint(
        app,
        PhotoHint {
            icon: Icon::Move,
            actions: &[MOVE_UP, MOVE_LEFT, MOVE_DOWN, MOVE_RIGHT],
            words: MOVE_WORDS,
        },
    );
    // no keycap: the wheel is no key, and the mouse's icon says where it is
    register_photo_hint(
        app,
        PhotoHint {
            icon: Icon::Mouse,
            actions: &[],
            words: ZOOM_WORDS,
        },
    );
    app.add_systems(OnEnter(Pause::Photo), keep_camera_home);
    app.add_systems(OnEnter(PropEditor::On), keep_camera_home);
    app.add_systems(
        Update,
        (
            pan_free_camera,
            // with a prop in the editor's hand, the wheel scales it instead
            zoom_free_camera
                .run_if(not(any_with_component::<Held>))
                .run_if(not(is_over_editor_window)),
        )
            .run_if(in_state(Pause::Photo).or_else(in_state(PropEditor::On))),
    );
    app.add_systems(OnExit(Pause::Photo), bring_camera_home);
    app.add_systems(OnExit(PropEditor::On), bring_camera_home);
}

// ==== Components =============================================================

/// Where the camera was and how far it was zoomed as photo mode began, to put both back.
#[derive(Component)]
struct FreeCameraHome {
    transform: Transform,
    scale: f32,
}

// ==== Systems ================================================================

/// Keeps where the screen camera is, and its zoom, as photo mode begins.
fn keep_camera_home(
    mut commands: Commands,
    camera: Single<(Entity, &Transform, &Projection), With<IsDefaultUiCamera>>,
) {
    let (entity, transform, projection) = *camera;
    commands.entity(entity).insert(FreeCameraHome {
        transform: *transform,
        scale: orthographic_scale(projection),
    });
}

/// Pans the camera by the walk keys, at a speed on the screen.
fn pan_free_camera(
    controls: Controls,
    time: Res<Time<Real>>,
    camera: Single<(&mut Transform, &Projection), With<IsDefaultUiCamera>>,
) {
    let (mut transform, projection) = camera.into_inner();
    let step = panned(walk_input(&controls), orthographic_scale(projection)) * time.delta_secs();
    if step != Vec2::ZERO {
        transform.translation += step.extend(0.0);
    }
}

/// Zooms the camera by the mouse wheel, read as it is rather than as an action: a binding is
/// a key or a mouse button, and a wheel is neither.
fn zoom_free_camera(
    scroll: Res<AccumulatedMouseScroll>,
    mut projection: Single<&mut Projection, With<IsDefaultUiCamera>>,
) {
    let notches = match scroll.unit {
        MouseScrollUnit::Line => scroll.delta.y,
        MouseScrollUnit::Pixel => scroll.delta.y / SCROLL_PX_PER_NOTCH,
    };
    if notches == 0.0 {
        return;
    }
    if let Projection::Orthographic(orthographic) = projection.as_mut() {
        orthographic.scale = zoomed(orthographic.scale, notches);
    }
}

/// Puts the camera back where it was, and as far zoomed, as photo mode began.
fn bring_camera_home(
    mut commands: Commands,
    camera: Single<(Entity, &mut Transform, &mut Projection, &FreeCameraHome)>,
) {
    let (entity, mut transform, mut projection, home) = camera.into_inner();
    *transform = home.transform;
    if let Projection::Orthographic(orthographic) = projection.as_mut() {
        orthographic.scale = home.scale;
    }
    commands.entity(entity).remove::<FreeCameraHome>();
}

/// Tells whether the pointer is over the prop editor's window, whose grid the wheel scrolls.
fn is_over_editor_window(sheets: Query<&Hovered, With<PropsSheet>>) -> bool {
    sheets.iter().any(Hovered::get)
}

// ==== Helpers ================================================================

/// Returns how far the camera is zoomed, 1 for a 2D camera that is not.
fn orthographic_scale(projection: &Projection) -> f32 {
    match projection {
        Projection::Orthographic(orthographic) => orthographic.scale,
        _ => 1.0,
    }
}

/// Returns the camera's velocity in the room for `input`, right and up, at a camera zoomed to
/// `scale`: the same speed on the screen, so more of the room when zoomed out.
fn panned(input: Vec2, scale: f32) -> Vec2 {
    input.normalize_or_zero() * FREE_CAMERA_PAN_PX_PER_S * scale
}

/// Returns the scale after `notches` of the wheel, up zooming in, held between the limits.
fn zoomed(scale: f32, notches: f32) -> f32 {
    (scale * ZOOM_STEP.powf(-notches)).clamp(ZOOM_MIN, ZOOM_MAX)
}

// ==== Tests ==================================================================

#[cfg(test)]
mod tests {
    use super::*;

    const TOLERANCE: f32 = 1e-4;

    #[test]
    fn a_notch_up_zooms_in_by_one_step() {
        assert!((zoomed(1.0, 1.0) - 1.0 / ZOOM_STEP).abs() < TOLERANCE);
    }

    #[test]
    fn the_zoom_stops_at_both_limits() {
        assert!((zoomed(ZOOM_MIN, 10.0) - ZOOM_MIN).abs() < TOLERANCE);
        assert!((zoomed(ZOOM_MAX, -10.0) - ZOOM_MAX).abs() < TOLERANCE);
    }

    #[test]
    fn a_pan_covers_more_of_the_room_zoomed_out() {
        assert!(panned(Vec2::X, 2.0).x > panned(Vec2::X, 1.0).x);
    }
}
