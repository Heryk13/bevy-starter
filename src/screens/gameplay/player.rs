use avian3d::prelude::{Collider, ColliderConstructorHierarchyReady, LockedAxes, RigidBody};
use bevy::prelude::*;

use crate::screens::Screen;

mod first_person; // camera: first-person

// ==== Config =================================================================

// ---- body: a capsule about a person's size ----------------------------------
// half a shoulder width
const PLAYER_RADIUS_M: f32 = 0.3;
// from the soles to the top of the head, hemispheres included
const PLAYER_HEIGHT_M: f32 = 1.8;

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
