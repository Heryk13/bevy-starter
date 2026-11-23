use bevy::input::mouse::AccumulatedMouseMotion;
use bevy::prelude::*;
use bevy::window::{CursorGrabMode, CursorOptions, PrimaryWindow};

use super::{PLAYER_HEIGHT_M, Player};
#[cfg(feature = "dev")]
use crate::dev_tools::prop_editor::PropEditor;
use crate::input::Controls;
use crate::save::Session;
use crate::screens::gameplay::playroom::PlayroomCamera;
use crate::screens::{Pause, Screen};
use crate::ui::ClosesOnEsc;

// ==== Config =================================================================

// ---- eyes -------------------------------------------------------------------
// from the soles, a little under the top of the head
const EYE_HEIGHT_M: f32 = 1.7;

// ---- look: how the mouse turns the view -------------------------------------
// the mouse reports pixels, the view turns in radians: about what Bevy's first-person
// example turns by
const LOOK_RAD_PER_PX: f32 = 0.002;
// how far up and down the view tilts: short of 90°, where forward and up become one
// direction and the view no longer knows which way is left, so it flips
const LOOK_PITCH_LIMIT_DEG: f32 = 85.0;
// how fast a right stick pushed all the way turns the view: half a turn in about a second
const LOOK_STICK_RAD_PER_S: f32 = 3.0;

// ---- derived from the eyes and the look -------------------------------------
// the camera hangs from the body, placed from the capsule's centre rather than its soles:
// the eyes less the half of the body below the centre
pub(super) const EYE_OFFSET_M: f32 = EYE_HEIGHT_M - PLAYER_HEIGHT_M / 2.0;
// the limit as Bevy's rotations take it
const LOOK_PITCH_LIMIT_RAD: f32 = LOOK_PITCH_LIMIT_DEG.to_radians();
// the stick's speed as the pixels of mouse it stands for, so both go one way through the look
const LOOK_STICK_PX_PER_S: f32 = LOOK_STICK_RAD_PER_S / LOOK_RAD_PER_PX;

// ==== Plugin =================================================================

/// The playroom's camera and the mouse look, whichever camera the game keeps: the camera hangs
/// from the body at eye height, the mouse turns the body across and tilts the camera up and
/// down, and the cursor is held while it does. Where the camera sits is each camera's own
/// file's, which this one never names, so a camera a game leaves out takes nothing from it.
/// Every camera needs this file, so it carries no marker and stays whichever cameras a game
/// keeps.
pub(super) fn plugin(app: &mut App) {
    app.add_observer(spawn_playroom_camera.run_if(in_state(Screen::Gameplay)));
    // every frame and on every screen: the cursor must come back the moment a modal opens
    // over the running game, or the game is left, whatever left it
    app.add_systems(Update, hold_cursor);
    app.add_systems(
        Update,
        look_around
            .run_if(in_state(Pause::Off))
            .run_if(not(any_with_component::<ClosesOnEsc>)),
    );
}

// ==== Systems ================================================================

/// Spawns the playroom's camera as a child of the player at eye height, so it drops, walks
/// and despawns with the body and needs no code to follow it. It starts tilted to the
/// session's pitch, so a loaded game looks where it was saved looking.
fn spawn_playroom_camera(add: On<Add<Player>>, mut commands: Commands, session: Res<Session>) {
    commands.spawn((
        PlayroomCamera,
        Transform::from_xyz(0.0, EYE_OFFSET_M, 0.0)
            .with_rotation(Quat::from_rotation_x(session.game.player.pitch)),
        ChildOf(add.entity),
    ));
}

/// Hides the cursor and holds it in place while the game runs with no modal open over it, so
/// a turn never carries it out of the window and every move of the mouse becomes looking
/// around; gives it back the rest of the time, for the menus and for a modal over the running
/// game, the file explorer say. A modal is whatever Esc closes first (`ClosesOnEsc`), so a dev
/// window the pointer goes through leaves it held. Writes only when that changes.
fn hold_cursor(
    pause: Option<Res<State<Pause>>>,
    modals: Query<(), With<ClosesOnEsc>>,
    mut cursor: Single<&mut CursorOptions, With<PrimaryWindow>>,
    mut held: Local<Option<bool>>,
    #[cfg(feature = "dev")] editor: Option<Res<State<PropEditor>>>,
    #[cfg(feature = "dev")] mouse: Res<ButtonInput<MouseButton>>,
) {
    let pause = pause.map(|pause| *pause.get());
    // the prop editor turns its camera while the right button is held, and only then: the rest
    // of the time the cursor clicks props. The editor is a dev tool: a release build never turns
    #[cfg(feature = "dev")]
    let turning = editor.is_some_and(|editor| *editor.get() == PropEditor::On)
        && mouse.pressed(MouseButton::Right);
    #[cfg(not(feature = "dev"))]
    let turning = false;
    let locked = is_cursor_locked(pause, !modals.is_empty(), turning);
    // against what was last asked, not what the window says: where the OS refuses to hold the
    // cursor, Bevy puts the old mode back, and asking again every frame would only fail again
    if *held == Some(locked) {
        return;
    }
    *held = Some(locked);
    cursor.grab_mode = if locked {
        CursorGrabMode::Locked
    } else {
        CursorGrabMode::None
    };
    cursor.visible = !locked;
}

/// Turns the view with the mouse: across turns the whole body, so walking follows where the
/// player faces, and up and down tilts only the camera, so looking down never tips the body.
/// The pitch is read back from the camera each frame rather than kept anywhere else. The
/// mouse's motion is already this frame's, so it is not scaled by the frame's time; a right
/// stick's push is a speed, so it is.
pub(super) fn look_around(
    mouse_motion: Res<AccumulatedMouseMotion>,
    controls: Controls,
    time: Res<Time>,
    mut body: Single<&mut Transform, With<Player>>,
    mut camera: Single<&mut Transform, (With<PlayroomCamera>, Without<Player>)>,
) {
    let stick = controls.stick(Gamepad::right_stick);
    let delta = look_delta_px(mouse_motion.delta, stick, time.delta_secs());
    body.rotate_y(-delta.x * LOOK_RAD_PER_PX);
    camera.rotation = Quat::from_rotation_x(next_pitch(
        camera.rotation.to_euler(EulerRot::YXZ).1,
        delta.y,
    ));
}

// ==== Helpers ================================================================

/// Tells whether the cursor is held for looking around: only while the game runs, `pause`
/// being `Off`, or in photo mode, whose free camera turns with the mouse, and no modal is
/// open over it; or while the prop editor turns its camera, `turning`, though its window is a
/// modal. Outside gameplay `pause` is `None`.
fn is_cursor_locked(pause: Option<Pause>, modal_open: bool, turning: bool) -> bool {
    let looking = match pause {
        Some(Pause::Off) => true,
        #[cfg(feature = "dev")]
        Some(Pause::Photo) => true,
        _ => false,
    };
    (looking && !modal_open) || turning
}

/// Returns how far the view turns this frame, as pixels of mouse: the mouse's own `mouse_px`,
/// and a right stick pushed to `stick` (up being up) for `delta_secs`. Screen y grows down, so
/// the stick's up is a negative y.
fn look_delta_px(mouse_px: Vec2, stick: Vec2, delta_secs: f32) -> Vec2 {
    mouse_px + Vec2::new(stick.x, -stick.y) * LOOK_STICK_PX_PER_S * delta_secs
}

/// Returns the pitch after the mouse moves `delta_y_px` down the screen, held inside the
/// limit. Screen y grows downwards and pitch upwards, so the motion is subtracted.
fn next_pitch(current: f32, delta_y_px: f32) -> f32 {
    (current - delta_y_px * LOOK_RAD_PER_PX).clamp(-LOOK_PITCH_LIMIT_RAD, LOOK_PITCH_LIMIT_RAD)
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
        assert!((pitch + (5.0 * LOOK_RAD_PER_PX)).abs() < TOLERANCE);
    }

    #[test]
    fn a_right_stick_pushed_up_for_a_second_looks_up_at_its_speed() {
        let delta = look_delta_px(Vec2::ZERO, Vec2::Y, 1.0);
        let pitch = next_pitch(0.0, delta.y);
        assert!((pitch - LOOK_STICK_RAD_PER_S.min(LOOK_PITCH_LIMIT_RAD)).abs() < TOLERANCE);
        assert!(delta.x.abs() < TOLERANCE);
    }

    #[test]
    fn pitch_stops_at_the_top_limit() {
        let pitch = next_pitch(LOOK_PITCH_LIMIT_RAD, -5.0);
        assert!((pitch - LOOK_PITCH_LIMIT_RAD).abs() < TOLERANCE);
    }

    #[test]
    fn pitch_stops_at_the_bottom_limit() {
        let pitch = next_pitch(-LOOK_PITCH_LIMIT_RAD, 5.0);
        assert!((pitch + LOOK_PITCH_LIMIT_RAD).abs() < TOLERANCE);
    }

    #[test]
    fn the_cursor_is_held_while_the_game_runs_with_nothing_over_it() {
        assert!(is_cursor_locked(Some(Pause::Off), false, false));
    }

    #[test]
    fn a_modal_over_the_running_game_frees_the_cursor() {
        assert!(!is_cursor_locked(Some(Pause::Off), true, false));
    }

    #[test]
    fn the_paused_game_frees_the_cursor() {
        assert!(!is_cursor_locked(Some(Pause::On), true, false));
    }

    #[test]
    fn the_cursor_is_free_outside_gameplay() {
        assert!(!is_cursor_locked(None, false, false));
    }

    #[cfg(feature = "dev")]
    #[test]
    fn the_cursor_is_held_in_photo_mode_for_its_free_camera() {
        assert!(is_cursor_locked(Some(Pause::Photo), false, false));
    }

    #[test]
    fn the_cursor_is_held_while_the_prop_editor_turns_its_camera() {
        assert!(is_cursor_locked(Some(Pause::Off), true, true));
    }
}
