use bevy::prelude::*;

use super::{PLAYER_HEIGHT_M, Player};
use crate::screens::Screen;
use crate::screens::gameplay::playroom::PlayroomCamera;

// ==== Config =================================================================

// ---- eyes -------------------------------------------------------------------
// from the soles, a little under the top of the head
const FIRST_PERSON_EYE_HEIGHT_M: f32 = 1.7;

// ---- derived from the eyes --------------------------------------------------
// the camera is the body's child, placed from the capsule's centre rather than its soles:
// the eyes less the half of the body below the centre
const FIRST_PERSON_EYE_OFFSET_M: f32 = FIRST_PERSON_EYE_HEIGHT_M - PLAYER_HEIGHT_M / 2.0;

// ==== Plugin =================================================================

/// Shows the playroom through the player's eyes. A game that keeps another camera deletes
/// this file and every line marked `// camera: first-person`.
pub(super) fn plugin(app: &mut App) {
    app.add_observer(spawn_first_person_camera.run_if(in_state(Screen::Gameplay)));
}

// ==== Systems ================================================================

/// Spawns the playroom's camera as a child of the player at eye height, so it drops, walks
/// and despawns with the body and needs no code to follow it.
fn spawn_first_person_camera(add: On<Add<Player>>, mut commands: Commands) {
    commands.spawn((
        PlayroomCamera,
        Transform::from_xyz(0.0, FIRST_PERSON_EYE_OFFSET_M, 0.0),
        ChildOf(add.entity),
    ));
}
