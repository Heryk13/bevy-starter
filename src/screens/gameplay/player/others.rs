use bevy::prelude::*;

use super::{PLAYER_SIZE_PX, PLAYER_Z};
use crate::multiplayer::world::FreshInWorld;
use crate::multiplayer::{MAX_PLAYERS, Network, NetworkedBody, RemotePlayer};
use crate::screens::Screen;
use crate::screens::gameplay::playroom::TILE_DISPLAY_PX;
use crate::ui::ACCENT;

// ==== Config =================================================================

// ---- where a joiner starts: a ring around the spawn point, the host's -------
// two tiles out: wide enough for ten squares side by side, close enough to see each other
const JOINER_RING_RADIUS_TILES: f32 = 2.0;

// ---- derived ----------------------------------------------------------------
const JOINER_RING_RADIUS_PX: f32 = JOINER_RING_RADIUS_TILES * TILE_DISPLAY_PX as f32;

// ==== Plugin =================================================================

/// The other players, in a game played together: the same square as the player's own, with no
/// name, moved by `multiplayer.rs` to where each of them stands. A joiner starts on a ring
/// around the spawn point, a place for each player already in, so no two squares start one on
/// the other. A game without multiplayer deletes this file and every line marked
/// `// multiplayer`.
pub(super) fn plugin(app: &mut App) {
    app.add_observer(draw_other_player.run_if(in_state(Screen::Gameplay)));
    app.add_observer(start_beside_the_others.run_if(in_state(Screen::Gameplay)));
}

// ==== Systems ================================================================

/// Gives another player's body the player's own square, at the player's depth.
fn draw_other_player(
    add: On<Add<RemotePlayer>>,
    mut commands: Commands,
    mut transforms: Query<&mut Transform>,
) {
    commands
        .entity(add.entity)
        .insert(Sprite::from_color(ACCENT, Vec2::splat(PLAYER_SIZE_PX)));
    if let Ok(mut transform) = transforms.get_mut(add.entity) {
        transform.translation.z = PLAYER_Z;
    }
}

/// Moves a joiner's own body, as it spawns, off the spawn point to its place on the ring: the
/// first free after the players already in, the host keeping the middle; only in a world it
/// is new to, since back in one it knows it starts where it stood.
fn start_beside_the_others(
    add: On<Add<NetworkedBody>>,
    network: Res<Network>,
    fresh: Res<FreshInWorld>,
    others: Query<(), With<RemotePlayer>>,
    mut bodies: Query<&mut Transform>,
) {
    if *network != Network::Joined || !fresh.0 {
        return;
    }
    if let Ok(mut transform) = bodies.get_mut(add.entity) {
        transform.translation += ring_place(others.iter().count()).extend(0.0);
    }
}

// ==== Helpers ================================================================

/// Returns the offset from the spawn point of a joiner who finds `others` players already in:
/// a place on the ring, one for each player a game can hold, so none is taken twice.
fn ring_place(others: usize) -> Vec2 {
    let places = MAX_PLAYERS.max(1) as f32;
    let angle = std::f32::consts::TAU * others as f32 / places;
    Vec2::from_angle(angle) * JOINER_RING_RADIUS_PX
}

// ==== Tests ==================================================================

#[cfg(test)]
mod tests {
    use super::*;

    const TOLERANCE: f32 = 1e-3;

    #[test]
    fn every_joiner_starts_on_the_ring_in_a_place_of_its_own() {
        let places: Vec<Vec2> = (0..MAX_PLAYERS as usize).map(ring_place).collect();
        for (index, place) in places.iter().enumerate() {
            assert!((place.length() - JOINER_RING_RADIUS_PX).abs() < TOLERANCE);
            for other in places.iter().skip(index + 1) {
                assert!(place.distance(*other) > PLAYER_SIZE_PX);
            }
        }
    }
}
