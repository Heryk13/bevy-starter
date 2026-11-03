use bevy::prelude::*;

use super::PlayroomCamera;
use crate::screens::Screen;

// ==== Config =================================================================

// ---- still camera: framed to judge the sky, the sun and the shadows ---------
// where it stands, in metres, about 41 m from the level's centre
const STILL_CAMERA_POSITION_M: Vec3 = Vec3::new(-10.0, 3.0, 40.0);
// what it looks at: high over the origin, so it tilts about 14° up and the horizon sits in
// the bottom fifth of the default 45° view, the level along the bottom and the sky above
const STILL_CAMERA_TARGET_M: Vec3 = Vec3::new(0.0, 13.0, 0.0);

// ==== Plugin =================================================================

/// Shows the playroom through a camera that never moves. A branch with a player drops this
/// file whole: its own camera carries `PlayroomCamera` instead.
pub(super) fn plugin(app: &mut App) {
    app.add_systems(OnEnter(Screen::Gameplay), spawn_still_camera);
}

// ==== Systems ================================================================

/// Spawns the still camera that shows the playroom, with the playroom's camera kit.
fn spawn_still_camera(mut commands: Commands) {
    commands.spawn((
        PlayroomCamera,
        Transform::from_translation(STILL_CAMERA_POSITION_M)
            .looking_at(STILL_CAMERA_TARGET_M, Vec3::Y),
        DespawnOnExit(Screen::Gameplay),
    ));
}
