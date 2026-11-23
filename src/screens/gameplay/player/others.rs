use avian3d::prelude::{Collider, RigidBody};
use bevy::prelude::*;

use super::{PLAYER_RADIUS_M, PLAYER_SEGMENT_LENGTH_M};
use crate::multiplayer::{MAX_PLAYERS, Network, NetworkedBody, RemotePlayer};
use crate::screens::Screen;

// ==== Config =================================================================

// ---- where a joiner starts: a ring around the spawn point, the host's -------
// wide enough for ten capsules side by side, close enough to see each other start
const JOINER_RING_RADIUS_M: f32 = 2.0;

// ==== Plugin =================================================================

/// The other players, in a game played together: the same capsule as the player's own, with
/// no name, moved by `multiplayer.rs` to where each of them stands. Kinematic: the player
/// bumps into them, and they are never pushed, since each one's own game moves them. A joiner
/// starts on a ring around the spawn point, a place for each player already in, so no two
/// bodies start one inside the other. A game without multiplayer deletes this file and every
/// line marked `// multiplayer`.
pub(super) fn plugin(app: &mut App) {
    app.add_observer(draw_other_player.run_if(in_state(Screen::Gameplay)));
    app.add_observer(start_beside_the_others.run_if(in_state(Screen::Gameplay)));
}

// ==== Systems ================================================================

/// Gives another player's body the player's own capsule, to see and to bump into.
fn draw_other_player(
    add: On<Add<RemotePlayer>>,
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    commands.entity(add.entity).insert((
        RigidBody::Kinematic,
        Collider::capsule(PLAYER_RADIUS_M, PLAYER_SEGMENT_LENGTH_M),
        Mesh3d(meshes.add(Capsule3d::new(PLAYER_RADIUS_M, PLAYER_SEGMENT_LENGTH_M))),
        MeshMaterial3d(materials.add(StandardMaterial::default())),
    ));
}

/// Moves a joiner's own body, as it spawns, off the spawn point to its place on the ring: the
/// first free after the players already in, the host keeping the middle.
fn start_beside_the_others(
    add: On<Add<NetworkedBody>>,
    network: Res<Network>,
    others: Query<(), With<RemotePlayer>>,
    mut bodies: Query<&mut Transform>,
) {
    if *network != Network::Joined {
        return;
    }
    if let Ok(mut transform) = bodies.get_mut(add.entity) {
        transform.translation += ring_place(others.iter().count());
    }
}

// ==== Helpers ================================================================

/// Returns the offset from the spawn point of a joiner who finds `others` players already in:
/// a place on the ring, one for each player a game can hold, so none is taken twice.
fn ring_place(others: usize) -> Vec3 {
    let places = MAX_PLAYERS.max(1) as f32;
    let angle = std::f32::consts::TAU * others as f32 / places;
    Vec3::new(angle.cos(), 0.0, angle.sin()) * JOINER_RING_RADIUS_M
}

// ==== Tests ==================================================================

#[cfg(test)]
mod tests {
    use super::*;

    const TOLERANCE: f32 = 1e-4;

    #[test]
    fn every_joiner_starts_on_the_ring_in_a_place_of_its_own() {
        let places: Vec<Vec3> = (0..MAX_PLAYERS as usize).map(ring_place).collect();
        for (index, place) in places.iter().enumerate() {
            assert!((place.length() - JOINER_RING_RADIUS_M).abs() < TOLERANCE);
            assert!(place.y.abs() < TOLERANCE);
            for other in places.iter().skip(index + 1) {
                assert!(place.distance(*other) > PLAYER_RADIUS_M * 2.0);
            }
        }
    }
}
