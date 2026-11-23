use bevy::input::common_conditions::input_pressed;
use bevy::input::mouse::AccumulatedMouseMotion;
use bevy::prelude::*;

use super::fly::{RISE, SINK, fly_velocity};
use super::photo_mode::{PhotoHint, register_photo_hint};
use super::prop_editor::PropEditor;
use crate::input::Controls;
use crate::screens::gameplay::player::{
    MOVE_BACK, MOVE_FORWARD, MOVE_LEFT, MOVE_RIGHT, Player, walk_input,
};
use crate::screens::gameplay::playroom::PlayroomCamera;
use crate::screens::{Pause, Screen};
use crate::ui::Icon;

// ==== Config =================================================================

// ---- moving: slow, since photos are taken close to the player ---------------
const FREE_CAMERA_SPEED_M_PER_S: f32 = 3.0;
// faster in the prop editor: a map is built across the whole level
const EDITOR_CAMERA_SPEED_M_PER_S: f32 = 8.0;

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

/// Photo mode's 3D free camera, and the prop editor's: the view leaves the player to frame the
/// photo or to build the map, moved by the walk keys, Rise and Sink, through walls if need
/// be, and comes back where it was as photo mode or the editor ends. Photo mode turns it with
/// the mouse; the editor only while the right button is held, as Unreal's does, since the rest
/// of the time the mouse clicks props. The editor lifts it off the body, which the running
/// game still moves. It counts real time, since photo mode freezes the game.
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
        OnEnter(PropEditor::On),
        (keep_camera_home, lift_camera_off_body).chain(),
    );
    app.add_systems(
        Update,
        (
            turn_free_camera.run_if(
                in_state(Pause::Photo)
                    .or_else(in_state(PropEditor::On).and_then(input_pressed(MouseButton::Right))),
            ),
            move_free_camera,
        )
            .chain()
            .run_if(in_state(Pause::Photo).or_else(in_state(PropEditor::On))),
    );
    app.add_systems(OnExit(Pause::Photo), bring_camera_home);
    app.add_systems(
        OnExit(PropEditor::On),
        (put_camera_on_body, bring_camera_home).chain(),
    );
}

// ==== Components =============================================================

/// Where the camera was as photo mode or the editor began, in its parent's space, to put it
/// back after.
#[derive(Component)]
struct FreeCameraHome(Transform);

// ==== Systems ================================================================

/// Keeps where the camera is as photo mode or the editor begins.
fn keep_camera_home(
    mut commands: Commands,
    camera: Single<(Entity, &Transform), With<PlayroomCamera>>,
) {
    let (entity, transform) = *camera;
    commands.entity(entity).insert(FreeCameraHome(*transform));
}

/// Lifts the camera off the body as the editor opens, where it is in the world, so the body
/// the running game still moves never drags it along. It goes with gameplay, as it would on
/// the body.
fn lift_camera_off_body(
    mut commands: Commands,
    camera: Single<(Entity, &GlobalTransform, &mut Transform), With<PlayroomCamera>>,
) {
    let (entity, global, mut transform) = camera.into_inner();
    *transform = global.compute_transform();
    commands
        .entity(entity)
        .remove::<ChildOf>()
        .insert(DespawnOnExit(Screen::Gameplay));
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
/// Sink, faster in the editor. In its parent's space: the body's in photo mode, which only
/// ever turns about the vertical, and the world's in the editor, so up there is up in the
/// world either way.
fn move_free_camera(
    controls: Controls,
    time: Res<Time<Real>>,
    editor: Option<Res<State<PropEditor>>>,
    mut camera: Single<&mut Transform, With<PlayroomCamera>>,
) {
    let speed = match editor.map(|editor| *editor.get()) {
        Some(PropEditor::On) => EDITOR_CAMERA_SPEED_M_PER_S,
        _ => FREE_CAMERA_SPEED_M_PER_S,
    };
    let velocity = fly_velocity(
        walk_input(&controls),
        camera.rotation,
        controls.pressed(RISE),
        controls.pressed(SINK),
        speed,
    );
    if velocity != Vec3::ZERO {
        camera.translation += velocity * time.delta_secs();
    }
}

/// Puts the camera back on the body as the editor closes, for `bring_camera_home` to put it
/// where it was there. With no body, gameplay is ending and the camera goes with it.
fn put_camera_on_body(
    mut commands: Commands,
    camera: Single<Entity, With<PlayroomCamera>>,
    body: Option<Single<Entity, With<Player>>>,
) {
    if let Some(body) = body {
        commands.entity(*camera).insert(ChildOf(*body));
    }
}

/// Puts the camera back where it was as photo mode or the editor began.
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
