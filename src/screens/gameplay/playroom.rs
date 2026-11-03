use bevy::light::atmosphere::ScatteringMedium;
use bevy::light::{Atmosphere, SunDisk};
use bevy::prelude::*;

use crate::screens::Screen;
use crate::screens::loading::GameplayAssets;

mod still_camera;
mod sun_glare;

// ==== Config =================================================================

// ---- sun --------------------------------------------------------------------
// the direction it shines from, given as a point looking at the origin: a directional
// light ignores its position and keeps only the direction, so the length is free and this
// sets the angle alone. Low, about 20° up, and 15° right of where the still camera looks,
// so the sun is in view and every shadow falls towards the camera, long enough to read
const SUN_DIRECTION: Vec3 = Vec3::new(5.0, 3.6, -8.7);
// how wide the disk drawn in the sky is, times the real sun's: the real one is an 8 px dot
// at this view, lost inside the glare's core, so this one is drawn larger, as games often
// do. It changes the disk alone, never the light or the shadows
const SUN_DISK_SCALE: f32 = 2.0;

// ---- sky --------------------------------------------------------------------
// samples in the table of how the air thins with height, and in the one of which way it
// scatters light; Earth's curves are smooth, so these, Bevy's own example values, are plenty,
// and fewer would band the sky's gradient
const SKY_FALLOFF_RESOLUTION: u32 = 256;
const SKY_PHASE_RESOLUTION: u32 = 256;

// ==== Plugin =================================================================

/// Shows the playroom: the level, its light and a still camera.
pub(super) fn plugin(app: &mut App) {
    app.add_systems(OnEnter(Screen::Gameplay), spawn_playroom);
    app.add_systems(OnExit(Screen::Gameplay), unload_gameplay_assets);
    app.add_plugins(still_camera::plugin);
    app.add_plugins(sun_glare::plugin);
}

// ==== Systems ================================================================

/// Spawns the playroom's level, already loaded by the loading screen, the sun that lights
/// it and the sky over it. It needs `GameplayAssets`, so
/// gameplay is entered through the loading screen, never straight from the menu.
fn spawn_playroom(
    mut commands: Commands,
    gameplay_assets: Res<GameplayAssets>,
    mut scattering_mediums: ResMut<Assets<ScatteringMedium>>,
) {
    commands.spawn((
        WorldAssetRoot(gameplay_assets.playroom.clone()),
        DespawnOnExit(Screen::Gameplay),
    ));
    commands.spawn((
        DirectionalLight {
            illuminance: light_consts::lux::RAW_SUNLIGHT,
            shadow_maps_enabled: true,
            ..default()
        },
        SunDisk {
            angular_size: SunDisk::EARTH.angular_size * SUN_DISK_SCALE,
            ..SunDisk::EARTH
        },
        Transform::from_translation(SUN_DIRECTION).looking_at(Vec3::ZERO, Vec3::Y),
        DespawnOnExit(Screen::Gameplay),
    ));

    let medium = scattering_mediums.add(ScatteringMedium::earth(
        SKY_FALLOFF_RESOLUTION,
        SKY_PHASE_RESOLUTION,
    ));
    commands.spawn((Atmosphere::earth(medium), DespawnOnExit(Screen::Gameplay)));
}

/// Drops the playroom's handles once gameplay ends, so Bevy can free the level's memory
/// while the player is in the menu. A load from the pause menu loads it again.
fn unload_gameplay_assets(mut commands: Commands) {
    commands.remove_resource::<GameplayAssets>();
}
