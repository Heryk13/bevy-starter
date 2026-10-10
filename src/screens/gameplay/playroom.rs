use bevy::prelude::*;

use crate::screens::Screen;
use crate::screens::loading::GameplayAssets;

// ==== Config =================================================================

// where the still camera stands, in metres, looking at the origin
const STILL_CAMERA_POSITION: Vec3 = Vec3::new(-20.0, 12.0, 20.0);
// below the screen camera's default order of 0, so the level is drawn first and the UI
// lands on top of it
const STILL_CAMERA_ORDER: isize = -1;

// ==== Plugin =================================================================

/// Shows the playroom: the level, its light and a still camera.
pub(super) fn plugin(app: &mut App) {
    app.add_systems(OnEnter(Screen::Gameplay), spawn_playroom);
}

// ==== Systems ================================================================

/// Spawns the playroom's level, already loaded by the loading screen, and the
/// still camera that shows it.
fn spawn_playroom(mut commands: Commands, gameplay_assets: Res<GameplayAssets>) {
    commands.spawn((
        WorldAssetRoot(gameplay_assets.playroom.clone()),
        DespawnOnExit(Screen::Gameplay),
    ));
    commands.spawn((
        Camera3d::default(),
        Camera {
            order: STILL_CAMERA_ORDER,
            ..default()
        },
        Transform::from_translation(STILL_CAMERA_POSITION).looking_at(Vec3::ZERO, Vec3::Y),
        DespawnOnExit(Screen::Gameplay),
    ));
}
