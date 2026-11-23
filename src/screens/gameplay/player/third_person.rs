use avian3d::prelude::{SpatialQuery, SpatialQueryFilter};
use bevy::prelude::*;

use super::Player;
use super::look::{EYE_OFFSET_M, look_around};
#[cfg(feature = "dev")]
use crate::dev_tools::physics_debug::PhysicsDebugView;
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
// half the width under which a thing between the eyes and the camera is seen past rather than
// pulling the camera in: a trunk, a post, a railing, but no wall
const NARROW_HALF_WIDTH_M: f32 = 0.5;

// ---- F6's second view: the rays, a dev tool's -------------------------------
#[cfg(feature = "dev")]
const DEBUG_RAY_FREE: Color = Color::srgb(0.2, 0.9, 0.3);
#[cfg(feature = "dev")]
const DEBUG_RAY_STOPPED: Color = Color::srgb(1.0, 0.25, 0.2);
#[cfg(feature = "dev")]
const DEBUG_RAY_SEEN_PAST: Color = Color::srgb(1.0, 0.85, 0.1);
#[cfg(feature = "dev")]
const DEBUG_CAMERA_RADIUS_M: f32 = 0.1;

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
/// itself left out. Something narrow the ray meets, a trunk or a post, leaves it where it is:
/// two more rays, `NARROW_HALF_WIDTH_M` to either side, tell a wall, which one of them meets
/// too, from a thing the player is seen past. Only while it hangs from the body: the prop
/// editor lifts it off.
fn put_camera_behind(
    spatial: SpatialQuery,
    // the body's `Transform`, not its `GlobalTransform`: it has no parent, so the two are
    // the same place, and the global one is only brought up to date after `Update`, so it
    // would cast the rays from where the body stood a frame ago
    bodies: Query<(Entity, &Transform), With<Player>>,
    mut cameras: Query<
        (&ChildOf, &mut Transform, &ThirdPersonView),
        (With<PlayroomCamera>, Without<Player>),
    >,
    #[cfg(feature = "dev")] debug_view: Res<PhysicsDebugView>,
    #[cfg(feature = "dev")] mut gizmos: Gizmos,
) {
    for (parent, mut transform, view) in &mut cameras {
        let Ok((body, body_place)) = bodies.get(parent.parent()) else {
            continue;
        };
        let eyes = Vec3::new(0.0, EYE_OFFSET_M, 0.0);
        let wanted = behind(transform.rotation, view.shoulder);
        let eyes_world = body_place.transform_point(eyes);
        let towards = body_place.transform_point(eyes + wanted) - eyes_world;
        let Ok(direction) = Dir3::new(towards) else {
            continue;
        };
        let filter = SpatialQueryFilter::default().with_excluded_entities([body]);
        let hit_from = |origin: Vec3| {
            spatial
                .cast_ray(origin, direction, towards.length(), true, &filter)
                .map(|hit| hit.distance)
        };
        // across the view, in the world: the camera's own right, turned with the body
        let side = body_place.rotation * transform.rotation * Vec3::X * NARROW_HALF_WIDTH_M;
        let rays = [eyes_world, eyes_world - side, eyes_world + side]
            .map(|origin| (origin, hit_from(origin)));
        let [(_, center), (_, left), (_, right)] = rays;
        let hit = blocking_hit(center, left, right);
        transform.translation = eyes + wanted * reach(towards.length(), hit);
        // F6's second view, a dev tool's, so not in a release build
        #[cfg(feature = "dev")]
        if *debug_view == PhysicsDebugView::CameraRays {
            let camera_world = body_place.transform_point(transform.translation);
            draw_camera_rays(
                &mut gizmos,
                rays,
                direction,
                towards.length(),
                hit,
                camera_world,
            );
        }
    }
}

/// Draws the camera's three rays, each as far as it went: green where a ray met nothing, red
/// where it met something that counts, the side rays' hits and a wall in the middle, and
/// yellow in the middle where what it met was narrow and left the camera where it is; and a
/// ball where the camera ends up. F6's second view.
#[cfg(feature = "dev")]
fn draw_camera_rays(
    gizmos: &mut Gizmos,
    rays: [(Vec3, Option<f32>); 3],
    direction: Dir3,
    length: f32,
    stopped_at: Option<f32>,
    camera: Vec3,
) {
    for (index, (origin, hit)) in rays.into_iter().enumerate() {
        let is_center = index == 0;
        let color = match hit {
            None => DEBUG_RAY_FREE,
            Some(_) if is_center && stopped_at.is_none() => DEBUG_RAY_SEEN_PAST,
            Some(_) => DEBUG_RAY_STOPPED,
        };
        gizmos.line(origin, origin + direction * hit.unwrap_or(length), color);
    }
    gizmos.sphere(
        Isometry3d::from_translation(camera),
        DEBUG_CAMERA_RADIUS_M,
        DEBUG_RAY_FREE,
    );
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

/// Returns what stops the camera, of what the ray from the eyes hit, `center`, given what the
/// rays either side of it hit, `left` and `right`: nothing when neither of those hit anything,
/// since what the middle one met is narrow enough to see the player past.
fn blocking_hit(center: Option<f32>, left: Option<f32>, right: Option<f32>) -> Option<f32> {
    center.filter(|_| left.is_some() || right.is_some())
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

    #[test]
    fn a_narrow_thing_leaves_the_camera_where_it_is() {
        assert_eq!(blocking_hit(Some(1.0), None, None), None);
    }

    #[test]
    fn a_wall_or_its_corner_stops_the_camera() {
        assert_eq!(blocking_hit(Some(1.0), Some(1.0), Some(1.0)), Some(1.0));
        assert_eq!(blocking_hit(Some(1.0), Some(1.2), None), Some(1.0));
        assert_eq!(blocking_hit(Some(1.0), None, Some(1.2)), Some(1.0));
    }

    #[test]
    fn nothing_in_the_middle_stops_nothing() {
        assert_eq!(blocking_hit(None, Some(1.0), Some(1.0)), None);
    }
}
