use avian3d::prelude::{
    Collider, ColliderConstructorHierarchyReady, LinearVelocity, LockedAxes, RigidBody,
};
use bevy::prelude::*;
use serde::{Deserialize, Serialize};

use crate::save::Session;
use crate::screens::gameplay::playroom::PlayroomCamera;
use crate::screens::{Pause, Screen};

mod first_person; // camera: first-person

// ==== Config =================================================================

// ---- body: a capsule about a person's size ----------------------------------
// half a shoulder width
const PLAYER_RADIUS_M: f32 = 0.3;
// from the soles to the top of the head, hemispheres included
const PLAYER_HEIGHT_M: f32 = 1.8;

// ---- walk -------------------------------------------------------------------
// a brisk jog rather than a real walk (1.4), picked in play, where a real walk felt slow
// on a screen; per second rather than per frame, since avian moves the body by its
// velocity and keeps it the same at any frame rate
const PLAYER_WALK_SPEED_M_PER_S: f32 = 4.4;

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
    // turn read one frame late is a few milliseconds no one can feel. record_player_state
    // has no order either: against the look for the same reason, and against the autosave,
    // which reads the session it writes, since a save a frame behind is a few centimetres
    // off. Both only in Pause::Off: a save loaded from the pause menu replaces the session
    // while the screen is still Gameplay, and recording then would write the body that is
    // about to go over the position just loaded
    app.add_systems(
        Update,
        (walk_player, record_player_state).run_if(in_state(Pause::Off)),
    );
}

// ==== Save ===================================================================

/// Where the player stands and looks, the part of `GameData` a save keeps of them. Velocity
/// is left out, so a loaded game starts standing still.
#[derive(Serialize, Deserialize, Clone)]
pub(crate) struct PlayerState {
    pub(crate) position: Vec3,
    pub(crate) yaw: f32,
    pub(crate) pitch: f32,
}

// written by hand, since a derived default would put the body at the origin, half sunk in
// the floor; both a new game and a save from before this field start here
impl Default for PlayerState {
    fn default() -> Self {
        Self {
            position: PLAYER_SPAWN_POINT,
            yaw: 0.0,
            pitch: 0.0,
        }
    }
}

// ==== Components =============================================================

/// Marks the player's body, the one entity the camera, walking and the save act on.
#[derive(Component)]
struct Player;

// ==== Systems ================================================================

/// Spawns the player where the session left them, the spawn point in a new game, once the
/// playroom's colliders are ready, since a dynamic body spawned before them falls through
/// the floor on the first physics step.
fn spawn_player(
    _: On<ColliderConstructorHierarchyReady>,
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    session: Res<Session>,
) {
    commands.spawn((
        RigidBody::Dynamic,
        Collider::capsule(PLAYER_RADIUS_M, PLAYER_SEGMENT_LENGTH_M),
        LockedAxes::ROTATION_LOCKED,
        Mesh3d(meshes.add(Capsule3d::new(PLAYER_RADIUS_M, PLAYER_SEGMENT_LENGTH_M))),
        MeshMaterial3d(materials.add(StandardMaterial::default())),
        Transform::from_translation(session.game.player.position)
            .with_rotation(Quat::from_rotation_y(session.game.player.yaw)),
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

/// Copies where the player stands and looks into the session every frame, so whatever
/// saves next, the pause menu or the autosave, writes where they are now. The yaw is the
/// body's turn and the pitch the camera's tilt, since the camera is the body's child and
/// only tilts.
fn record_player_state(
    body: Single<&Transform, With<Player>>,
    camera: Single<&Transform, (With<PlayroomCamera>, Without<Player>)>,
    mut session: ResMut<Session>,
) {
    let body = body.into_inner();
    session.game.player.pitch = camera.into_inner().rotation.to_euler(EulerRot::YXZ).1;
    session.game.player.yaw = body.rotation.to_euler(EulerRot::YXZ).0;
    session.game.player.position = body.translation;
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
    use crate::save::GameData;

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

    #[test]
    fn game_data_reads_back_from_ron_unchanged() {
        let game_data = GameData {
            player: PlayerState {
                position: Vec3::new(1.0, 2.0, 3.0),
                yaw: 1.0,
                pitch: 0.5,
            },
        };
        let ron_text = ron::to_string(&game_data).unwrap();
        let read_back: GameData = ron::from_str(&ron_text).unwrap();
        assert!(
            game_data
                .player
                .position
                .abs_diff_eq(read_back.player.position, TOLERANCE)
        );
        assert!((game_data.player.yaw - read_back.player.yaw).abs() < TOLERANCE);
        assert!((game_data.player.pitch - read_back.player.pitch).abs() < TOLERANCE);
    }

    #[test]
    fn save_without_a_player_loads_at_the_spawn() {
        let ron_text = r#"
        (
            // no player field
        )
        "#;
        let read_back: GameData = ron::from_str(ron_text).unwrap();
        assert!(
            read_back
                .player
                .position
                .abs_diff_eq(PLAYER_SPAWN_POINT, TOLERANCE)
        );
        assert!(read_back.player.yaw.abs() < TOLERANCE);
        assert!(read_back.player.pitch.abs() < TOLERANCE);
    }
}
