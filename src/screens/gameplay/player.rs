use avian3d::prelude::{
    Collider, ColliderConstructorHierarchyReady, LinearVelocity, LockedAxes, RigidBody,
};
use bevy::prelude::*;

use crate::screens::{Pause, Screen};

mod first_person; // camera: first-person

// ==== Config =================================================================

// ---- body: a capsule about a person's size ----------------------------------
// half a shoulder width
const PLAYER_RADIUS_M: f32 = 0.3;
// from the soles to the top of the head, hemispheres included
const PLAYER_HEIGHT_M: f32 = 1.8;

// ---- walk -------------------------------------------------------------------
// a real person's walking pace, per second rather than per frame, since avian moves the
// body by its velocity and keeps it the same at any frame rate
const PLAYER_WALK_SPEED_M_PER_S: f32 = 1.4;

// ---- spawn: where a new game starts -----------------------------------------
// how far above the floor the soles start, so the body is seen to drop and land, and
// a floor a little above the origin never swallows it
const PLAYER_SPAWN_DROP_M: f32 = 0.5;

// ---- derived from the body --------------------------------------------------
// the capsule's middle without its two hemispheres, the length Collider::capsule and
// Capsule3d::new both take: the height minus a radius at each end
const PLAYER_SEGMENT_LENGTH_M: f32 = PLAYER_HEIGHT_M - PLAYER_RADIUS_M * 2.0;
// where the capsule's centre starts: half its height puts the soles on the floor, and the
// drop lifts them off it
const PLAYER_SPAWN_HEIGHT_M: f32 = PLAYER_HEIGHT_M / 2.0 + PLAYER_SPAWN_DROP_M;
// the centre of the map, among the ramps, jumps and stairs: a constant rather than the
// glb's spawn node, so renaming a node in Blender never moves it
const PLAYER_SPAWN_POINT: Vec3 = Vec3::new(0.0, PLAYER_SPAWN_HEIGHT_M, 0.0);

// ==== Plugin =================================================================

pub(super) fn plugin(app: &mut App) {
    app.add_observer(spawn_player.run_if(in_state(Screen::Gameplay)));
    app.add_plugins(first_person::plugin); // camera: first-person
    // walk_player reads the yaw that the camera's look writes, with no order between them:
    // naming the camera's system here would tie the body to a file a game may delete, and a
    // turn read one frame late is a few milliseconds no one can feel
    app.add_systems(Update, walk_player.run_if(in_state(Pause::Off)));
}

// ==== Components =============================================================

/// Marks the player's body, the one entity the camera and, later, walking act on.
#[derive(Component)]
struct Player;

// ==== Systems ================================================================

/// Spawns the player at the spawn point once the playroom's colliders are ready, since a
/// dynamic body spawned before them falls through the floor on the first physics step.
fn spawn_player(
    _: On<ColliderConstructorHierarchyReady>,
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    commands.spawn((
        RigidBody::Dynamic,
        Collider::capsule(PLAYER_RADIUS_M, PLAYER_SEGMENT_LENGTH_M),
        LockedAxes::ROTATION_LOCKED,
        Mesh3d(meshes.add(Capsule3d::new(PLAYER_RADIUS_M, PLAYER_SEGMENT_LENGTH_M))),
        MeshMaterial3d(materials.add(StandardMaterial::default())),
        Transform::from_translation(PLAYER_SPAWN_POINT),
        Player,
        DespawnOnExit(Screen::Gameplay),
    ));
}

/// Walks the player with WASD where the body faces. Only the velocity across the ground is
/// set, so letting go stops the body at once, and its height is left to gravity.
fn walk_player(
    keys: Res<ButtonInput<KeyCode>>,
    body: Single<(&Transform, &mut LinearVelocity), With<Player>>,
) {
    let (transform, mut velocity) = body.into_inner();
    let mut input = Vec2::ZERO;
    if keys.pressed(KeyCode::KeyW) {
        input.y += 1.0;
    }
    if keys.pressed(KeyCode::KeyS) {
        input.y -= 1.0;
    }
    if keys.pressed(KeyCode::KeyA) {
        input.x -= 1.0;
    }
    if keys.pressed(KeyCode::KeyD) {
        input.x += 1.0;
    }
    let walk = walk_velocity(input, transform.rotation);
    velocity.x = walk.x;
    velocity.z = walk.z;
}

// ==== Helpers ================================================================

/// Returns the velocity the keys ask for, on the ground and turned to where the body faces:
/// `input` is right and forward, one per key, made one long first so a diagonal is no
/// faster than straight ahead, and forward is -Z, as everywhere in Bevy.
fn walk_velocity(input: Vec2, rotation: Quat) -> Vec3 {
    let direction = input.normalize_or_zero();
    let local = Vec3::new(direction.x, 0.0, -direction.y);
    rotation * local * PLAYER_WALK_SPEED_M_PER_S
}

// ==== Tests ==================================================================

#[cfg(test)]
mod tests {
    use super::*;

    const TOLERANCE: f32 = 1e-5;

    #[test]
    fn nothing_pressed_stands_still() {
        assert!(walk_velocity(Vec2::ZERO, Quat::IDENTITY).abs_diff_eq(Vec3::ZERO, TOLERANCE));
    }

    #[test]
    fn forward_walks_straight_ahead() {
        let expected = Vec3::new(0.0, 0.0, -PLAYER_WALK_SPEED_M_PER_S);
        assert!(
            walk_velocity(Vec2::new(0.0, 1.0), Quat::IDENTITY).abs_diff_eq(expected, TOLERANCE)
        );
    }

    #[test]
    fn a_diagonal_walks_as_fast_as_straight_ahead() {
        let straight = walk_velocity(Vec2::new(0.0, 1.0), Quat::IDENTITY);
        let diagonal = walk_velocity(Vec2::new(1.0, 1.0), Quat::IDENTITY);
        assert!((diagonal.length() - straight.length()).abs() < TOLERANCE);
    }

    #[test]
    fn forward_follows_where_the_body_faces() {
        // a quarter turn to the left: a positive turn about Y is anticlockwise seen from above
        let rotation = Quat::from_rotation_y(std::f32::consts::FRAC_PI_2);
        let expected = Vec3::new(-PLAYER_WALK_SPEED_M_PER_S, 0.0, 0.0);
        assert!(walk_velocity(Vec2::new(0.0, 1.0), rotation).abs_diff_eq(expected, TOLERANCE));
    }
}
