use bevy::input::mouse::AccumulatedMouseMotion;
use bevy::prelude::*;

use super::fly::{RISE, SINK, fly_velocity};
use super::photo_mode::{PhotoHint, register_photo_hint};
use crate::input::Controls;
use crate::screens::Pause;
use crate::screens::gameplay::player::{
    MOVE_BACK, MOVE_FORWARD, MOVE_LEFT, MOVE_RIGHT, walk_input,
};
use crate::screens::gameplay::playroom::PlayroomCamera;
use crate::ui::Icon;

// ==== Config =================================================================

// ---- moving: slow, since photos are taken close to the player ---------------
const FREE_CAMERA_SPEED_M_PER_S: f32 = 3.0;

// ---- turning: as first person turns -----------------------------------------
const FREE_CAMERA_LOOK_RAD_PER_PX: f32 = 0.002;
// short of straight up and down, where the view would no longer know which way is left
const FREE_CAMERA_PITCH_LIMIT_DEG: f32 = 85.0;

// ---- words: English, as every dev tool's, never read in a release build -----
const MOVE_WORDS: &str = "Move";
const UP_WORDS: &str = "Up";
const DOWN_WORDS: &str = "Down";

// ---- derived ----------------------------------------------------------------
const FREE_CAMERA_PITCH_LIMIT_RAD: f32 = FREE_CAMERA_PITCH_LIMIT_DEG.to_radians();

// ==== Plugin =================================================================

/// Photo mode's 3D free camera: the view leaves the player to frame the photo, turned by the
/// mouse and moved by the walk keys, Rise and Sink, through walls if need be, and comes back
/// where it was as photo mode ends. It counts real time, since the game is frozen.
pub(super) fn plugin(app: &mut App) {
    for (icon, actions, words) in [
        (
            Icon::Move,
            &[MOVE_FORWARD, MOVE_LEFT, MOVE_BACK, MOVE_RIGHT][..],
            MOVE_WORDS,
        ),
        (Icon::ArrowUp, &[RISE][..], UP_WORDS),
        (Icon::ArrowDown, &[SINK][..], DOWN_WORDS),
    ] {
        register_photo_hint(
            app,
            PhotoHint {
                icon,
                actions,
                words,
            },
        );
    }
    app.add_systems(OnEnter(Pause::Photo), keep_camera_home);
    app.add_systems(
        Update,
        (turn_free_camera, move_free_camera)
            .chain()
            .run_if(in_state(Pause::Photo)),
    );
    app.add_systems(OnExit(Pause::Photo), bring_camera_home);
}

// ==== Components =============================================================

/// Where the camera was as photo mode began, in its parent's space, to put it back after.
#[derive(Component)]
struct FreeCameraHome(Transform);

// ==== Systems ================================================================

/// Keeps where the camera is as photo mode begins.
fn keep_camera_home(
    mut commands: Commands,
    camera: Single<(Entity, &Transform), With<PlayroomCamera>>,
) {
    let (entity, transform) = *camera;
    commands.entity(entity).insert(FreeCameraHome(*transform));
}

/// Turns the camera with the mouse, across and up and down, in its parent's space.
fn turn_free_camera(
    mouse_motion: Res<AccumulatedMouseMotion>,
    mut camera: Single<&mut Transform, With<PlayroomCamera>>,
) {
    if mouse_motion.delta != Vec2::ZERO {
        camera.rotation = turned(camera.rotation, mouse_motion.delta);
    }
}

/// Moves the camera where it looks by the walk keys, and straight up and down by Rise and
/// Sink. In its parent's space, the body's, which only ever turns about the vertical, so up
/// there is up in the world.
fn move_free_camera(
    controls: Controls,
    time: Res<Time<Real>>,
    mut camera: Single<&mut Transform, With<PlayroomCamera>>,
) {
    let velocity = fly_velocity(
        walk_input(&controls),
        camera.rotation,
        controls.pressed(RISE),
        controls.pressed(SINK),
        FREE_CAMERA_SPEED_M_PER_S,
    );
    if velocity != Vec3::ZERO {
        camera.translation += velocity * time.delta_secs();
    }
}

/// Puts the camera back where it was as photo mode began.
fn bring_camera_home(
    mut commands: Commands,
    camera: Single<(Entity, &mut Transform, &FreeCameraHome), With<PlayroomCamera>>,
) {
    let (entity, mut transform, home) = camera.into_inner();
    *transform = home.0;
    commands.entity(entity).remove::<FreeCameraHome>();
}

// ==== Helpers ================================================================

/// Returns `rotation` after the mouse moves `delta_px`: right turns right, down looks down,
/// and up and down stop short of straight up and down.
fn turned(rotation: Quat, delta_px: Vec2) -> Quat {
    let (yaw, pitch, _) = rotation.to_euler(EulerRot::YXZ);
    let yaw = yaw - delta_px.x * FREE_CAMERA_LOOK_RAD_PER_PX;
    let pitch = (pitch - delta_px.y * FREE_CAMERA_LOOK_RAD_PER_PX)
        .clamp(-FREE_CAMERA_PITCH_LIMIT_RAD, FREE_CAMERA_PITCH_LIMIT_RAD);
    Quat::from_euler(EulerRot::YXZ, yaw, pitch, 0.0)
}

// ==== Tests ==================================================================

#[cfg(test)]
mod tests {
    use super::*;

    const TOLERANCE: f32 = 1e-4;

    #[test]
    fn moving_the_mouse_right_turns_right() {
        let forward = turned(Quat::IDENTITY, Vec2::new(100.0, 0.0)) * Vec3::NEG_Z;
        assert!(forward.x > 0.0);
    }

    #[test]
    fn the_view_stops_short_of_straight_up() {
        let rotation = turned(Quat::IDENTITY, Vec2::new(0.0, -100_000.0));
        let pitch = rotation.to_euler(EulerRot::YXZ).1;
        assert!((pitch - FREE_CAMERA_PITCH_LIMIT_RAD).abs() < TOLERANCE);
    }
}
