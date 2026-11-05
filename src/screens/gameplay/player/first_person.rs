#[cfg(feature = "dev")]
use avian3d::debug_render::DebugRender;
use bevy::input::mouse::AccumulatedMouseMotion;
use bevy::prelude::*;
use bevy::window::{CursorGrabMode, CursorOptions, PrimaryWindow};

use super::{PLAYER_HEIGHT_M, Player};
use crate::save::Session;
use crate::screens::gameplay::playroom::PlayroomCamera;
use crate::screens::{Pause, Screen};

// ==== Config =================================================================

// ---- eyes -------------------------------------------------------------------
// from the soles, a little under the top of the head
const FIRST_PERSON_EYE_HEIGHT_M: f32 = 1.7;

// ---- look: how the mouse turns the view -------------------------------------
// the mouse reports pixels, the view turns in radians: about what Bevy's first-person
// example turns by
const FIRST_PERSON_LOOK_RAD_PER_PX: f32 = 0.002;
// how far up and down the view tilts: short of 90°, where forward and up become one
// direction and the view no longer knows which way is left, so it flips
const FIRST_PERSON_PITCH_LIMIT_DEG: f32 = 85.0;

// ---- derived from the eyes and the look -------------------------------------
// the camera is the body's child, placed from the capsule's centre rather than its soles:
// the eyes less the half of the body below the centre
const FIRST_PERSON_EYE_OFFSET_M: f32 = FIRST_PERSON_EYE_HEIGHT_M - PLAYER_HEIGHT_M / 2.0;
// the limit as Bevy's rotations take it
const FIRST_PERSON_PITCH_LIMIT_RAD: f32 = FIRST_PERSON_PITCH_LIMIT_DEG.to_radians();

// ==== Plugin =================================================================

/// Shows the playroom through the player's eyes. A game that keeps another camera deletes
/// this file and every line marked `// camera: first-person`.
pub(super) fn plugin(app: &mut App) {
    app.add_observer(spawn_first_person_camera.run_if(in_state(Screen::Gameplay)));
    #[cfg(feature = "dev")]
    app.add_observer(hide_body_debug_collider.run_if(in_state(Screen::Gameplay)));
    app.add_systems(OnEnter(Pause::Off), lock_cursor);
    app.add_systems(Update, look_around.run_if(in_state(Pause::Off)));
    app.add_systems(OnExit(Pause::Off), unlock_cursor);
}

// ==== Systems ================================================================

/// Spawns the playroom's camera as a child of the player at eye height, so it drops, walks
/// and despawns with the body and needs no code to follow it. It starts tilted to the
/// session's pitch, so a loaded game looks where it was saved looking.
fn spawn_first_person_camera(add: On<Add<Player>>, mut commands: Commands, session: Res<Session>) {
    commands.spawn((
        PlayroomCamera,
        Transform::from_xyz(0.0, FIRST_PERSON_EYE_OFFSET_M, 0.0)
            .with_rotation(Quat::from_rotation_x(session.game.player.pitch)),
        ChildOf(add.entity),
    ));
}

/// Hides the body's own collider from the physics debug view, since the camera sits inside
/// it and would see its outline as a line down the middle of the screen.
#[cfg(feature = "dev")]
fn hide_body_debug_collider(add: On<Add<Player>>, mut commands: Commands) {
    commands.entity(add.entity).insert(DebugRender::none());
}

/// Hides the cursor and holds it in place while the game runs, so a turn never carries it
/// out of the window, and every move of the mouse becomes looking around.
fn lock_cursor(mut cursor: Single<&mut CursorOptions, With<PrimaryWindow>>) {
    cursor.grab_mode = CursorGrabMode::Locked;
    cursor.visible = false;
}

/// Turns the view with the mouse: across turns the whole body, so walking follows where the
/// player faces, and up and down tilts only the camera, so looking down never tips the body.
/// The pitch is read back from the camera each frame rather than kept anywhere else. The
/// mouse's motion is already this frame's, so it is not scaled by the frame's time.
fn look_around(
    mouse_motion: Res<AccumulatedMouseMotion>,
    mut body: Single<&mut Transform, With<Player>>,
    mut camera: Single<&mut Transform, (With<PlayroomCamera>, Without<Player>)>,
) {
    body.rotate_y(-mouse_motion.delta.x * FIRST_PERSON_LOOK_RAD_PER_PX);
    camera.rotation = Quat::from_rotation_x(next_pitch(
        camera.rotation.to_euler(EulerRot::YXZ).1,
        mouse_motion.delta.y,
    ));
}

/// Gives the cursor back whenever the game stops running, paused or left, for the menus.
fn unlock_cursor(mut cursor: Single<&mut CursorOptions, With<PrimaryWindow>>) {
    cursor.grab_mode = CursorGrabMode::None;
    cursor.visible = true;
}

// ==== Helpers ================================================================

/// Returns the pitch after the mouse moves `delta_y_px` down the screen, held inside the
/// limit. Screen y grows downwards and pitch upwards, so the motion is subtracted.
fn next_pitch(current: f32, delta_y_px: f32) -> f32 {
    (current - delta_y_px * FIRST_PERSON_LOOK_RAD_PER_PX)
        .clamp(-FIRST_PERSON_PITCH_LIMIT_RAD, FIRST_PERSON_PITCH_LIMIT_RAD)
}

// ==== Tests ==================================================================

#[cfg(test)]
mod tests {
    use super::*;

    const TOLERANCE: f32 = 1e-5;

    #[test]
    fn pitch_holds_still_while_the_mouse_does() {
        let pitch = next_pitch(0.3, 0.0);
        assert!((pitch - 0.3).abs() < TOLERANCE);
    }

    #[test]
    fn moving_the_mouse_down_looks_down() {
        let pitch = next_pitch(0.0, 5.0);
        assert!((pitch + (5.0 * FIRST_PERSON_LOOK_RAD_PER_PX)).abs() < TOLERANCE);
    }

    #[test]
    fn pitch_stops_at_the_top_limit() {
        let pitch = next_pitch(FIRST_PERSON_PITCH_LIMIT_RAD, -5.0);
        assert!((pitch - FIRST_PERSON_PITCH_LIMIT_RAD).abs() < TOLERANCE);
    }

    #[test]
    fn pitch_stops_at_the_bottom_limit() {
        let pitch = next_pitch(-FIRST_PERSON_PITCH_LIMIT_RAD, 5.0);
        assert!((pitch + FIRST_PERSON_PITCH_LIMIT_RAD).abs() < TOLERANCE);
    }
}
