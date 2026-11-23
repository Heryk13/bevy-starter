use avian3d::prelude::{SpatialQuery, SpatialQueryFilter};
use bevy::prelude::*;

use super::Player;
use super::look::{EYE_OFFSET_M, look_around};
use crate::input::{Action, ActionName, Binding, Page, action_just_pressed, register_action};
use crate::screens::gameplay::playroom::PlayroomCamera;
use crate::screens::{Pause, Screen};

// ==== Config =================================================================

// swaps the shoulder the camera looks over: the action, and the key it starts on
const SWAP_SHOULDER: ActionName = ActionName("swap_shoulder");
const SWAP_SHOULDER_KEY: KeyCode = KeyCode::KeyV;
const SWAP_SHOULDER_LABEL: &str = "action-swap-shoulder";

// ---- the camera: behind the eyes, above them, over one shoulder -------------
const THIRD_PERSON_DISTANCE_M: f32 = 3.0;
const THIRD_PERSON_RISE_M: f32 = 0.3;
const THIRD_PERSON_SHOULDER_M: f32 = 0.6;
// how far in front of a wall the camera stops, so it never shows the wall's inside
const THIRD_PERSON_WALL_GAP_M: f32 = 0.2;

// ==== Plugin =================================================================

/// Shows the player from behind and above one shoulder, shooter style: the mouse still turns
/// the body (`look.rs`), so the character always faces where the camera looks. V swaps the
/// shoulder, and a wall between the eyes and the camera pulls the camera in. A game that keeps
/// another camera deletes this file and every line marked `// camera: third-person`.
pub(super) fn plugin(app: &mut App) {
    register_action(
        app,
        Action {
            name: SWAP_SHOULDER,
            default: Binding::Key(SWAP_SHOULDER_KEY),
            page: Page::Game,
            label: Some(SWAP_SHOULDER_LABEL),
        },
    );
    app.add_observer(view_in_third_person.run_if(in_state(Screen::Gameplay)));
    app.add_systems(
        Update,
        (
            swap_shoulder.run_if(action_just_pressed(SWAP_SHOULDER)),
            put_camera_behind.after(look_around),
        )
            .chain()
            // photo mode's free camera moves it on the body, out of the running game
            .run_if(in_state(Pause::Off)),
    );
}

// ==== Components =============================================================

/// Marks the camera while it looks over the player's shoulder, with which shoulder.
#[derive(Component, Default)]
pub(crate) struct ThirdPersonView {
    shoulder: Shoulder,
}

/// The shoulder the camera looks over.
#[derive(Clone, Copy, Default, PartialEq, Eq, Debug)]
enum Shoulder {
    #[default]
    Right,
    Left,
}

// ==== Systems ================================================================

/// Puts the camera in third person as it spawns. With other cameras in the game, the switch
/// (`camera_switch.rs`) then keeps only the saved one's view.
fn view_in_third_person(add: On<Add<PlayroomCamera>>, mut commands: Commands) {
    commands
        .entity(add.entity)
        .insert(ThirdPersonView::default());
}

/// Moves the camera to the other shoulder on V.
fn swap_shoulder(mut views: Query<&mut ThirdPersonView>) {
    for mut view in &mut views {
        view.shoulder = match view.shoulder {
            Shoulder::Right => Shoulder::Left,
            Shoulder::Left => Shoulder::Right,
        };
    }
}

/// Puts the camera behind and above the eyes, over the shoulder, turned with the pitch around
/// them, and pulls it in to short of a wall a ray from the eyes meets on the way, the body
/// itself left out. Only while it hangs from the body: the prop editor lifts it off.
fn put_camera_behind(
    spatial: SpatialQuery,
    bodies: Query<(Entity, &GlobalTransform), With<Player>>,
    mut cameras: Query<(&ChildOf, &mut Transform, &ThirdPersonView), With<PlayroomCamera>>,
) {
    for (parent, mut transform, view) in &mut cameras {
        let Ok((body, body_global)) = bodies.get(parent.parent()) else {
            continue;
        };
        let eyes = Vec3::new(0.0, EYE_OFFSET_M, 0.0);
        let wanted = behind(transform.rotation, view.shoulder);
        let eyes_world = body_global.transform_point(eyes);
        let towards = body_global.transform_point(eyes + wanted) - eyes_world;
        let hit = Dir3::new(towards).ok().and_then(|direction| {
            let filter = SpatialQueryFilter::default().with_excluded_entities([body]);
            spatial
                .cast_ray(eyes_world, direction, towards.length(), true, &filter)
                .map(|hit| hit.distance)
        });
        transform.translation = eyes + wanted * reach(towards.length(), hit);
    }
}

// ==== Helpers ================================================================

/// Returns where the camera sits from the eyes, in the body's space, with the view tilted by
/// `pitch`: behind, above and to the shoulder's side, swung round the eyes as the view tilts,
/// so looking down raises it.
fn behind(pitch: Quat, shoulder: Shoulder) -> Vec3 {
    let side = match shoulder {
        Shoulder::Right => THIRD_PERSON_SHOULDER_M,
        Shoulder::Left => -THIRD_PERSON_SHOULDER_M,
    };
    pitch * Vec3::new(side, THIRD_PERSON_RISE_M, THIRD_PERSON_DISTANCE_M)
}

/// Returns how much of the way from the eyes the camera may go, `distance` away, when a ray
/// along it hits something `hit` away: all of it with no hit, else to `THIRD_PERSON_WALL_GAP_M`
/// short of the hit, never past the eyes.
fn reach(distance: f32, hit: Option<f32>) -> f32 {
    match hit {
        Some(hit) if distance > 0.0 => ((hit - THIRD_PERSON_WALL_GAP_M) / distance).clamp(0.0, 1.0),
        _ => 1.0,
    }
}

// ==== Tests ==================================================================

#[cfg(test)]
mod tests {
    use super::*;

    const TOLERANCE: f32 = 1e-4;

    #[test]
    fn the_camera_sits_behind_above_and_over_the_right_shoulder() {
        let offset = behind(Quat::IDENTITY, Shoulder::Right);
        // the camera looks down -Z, so behind is +Z
        assert!(offset.z > 0.0);
        assert!(offset.y > 0.0);
        assert!(offset.x > 0.0);
    }

    #[test]
    fn the_other_shoulder_mirrors_it() {
        let right = behind(Quat::IDENTITY, Shoulder::Right);
        let left = behind(Quat::IDENTITY, Shoulder::Left);
        assert!((left.x + right.x).abs() < TOLERANCE);
        assert!((left.y - right.y).abs() < TOLERANCE);
        assert!((left.z - right.z).abs() < TOLERANCE);
    }

    #[test]
    fn looking_down_raises_the_camera() {
        let level = behind(Quat::IDENTITY, Shoulder::Right);
        let down = behind(Quat::from_rotation_x(-0.5), Shoulder::Right);
        assert!(down.y > level.y);
    }

    #[test]
    fn a_wall_pulls_the_camera_to_short_of_it() {
        let distance = THIRD_PERSON_DISTANCE_M;
        let hit = distance / 2.0;
        let expected = (hit - THIRD_PERSON_WALL_GAP_M) / distance;
        assert!((reach(distance, Some(hit)) - expected).abs() < TOLERANCE);
        assert!((reach(distance, None) - 1.0).abs() < TOLERANCE);
        assert!(reach(distance, Some(0.0)).abs() < TOLERANCE);
    }
}
