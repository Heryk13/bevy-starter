use bevy::camera::{Exposure, Hdr};
use bevy::light::atmosphere::ScatteringMedium;
use bevy::light::{Atmosphere, AtmosphereEnvironmentMapLight, SunDisk};
use bevy::pbr::AtmosphereSettings;
use bevy::post_process::bloom::Bloom;
use bevy::prelude::*;

use crate::screens::Screen;
use crate::screens::loading::GameplayAssets;

mod sun_glare;

// ==== Config =================================================================

// ---- still camera: framed to judge the sky, the sun and the shadows ---------
// where it stands, in metres, about 41 m from the level's centre
const STILL_CAMERA_POSITION: Vec3 = Vec3::new(-10.0, 3.0, 40.0);
// what it looks at: high over the origin, so it tilts about 14° up and the horizon sits in
// the bottom fifth of the default 45° view, the level along the bottom and the sky above
const STILL_CAMERA_TARGET: Vec3 = Vec3::new(0.0, 13.0, 0.0);
// below the screen camera's default order of 0, so the level is drawn first and the UI
// lands on top of it
const STILL_CAMERA_ORDER: isize = -1;
// outdoor daylight: the sun is RAW_SUNLIGHT, far too bright for the default exposure, made
// for indoor light. Each step up halves the brightness. Bevy's atmosphere example uses 13,
// this one was settled by eye
const STILL_CAMERA_EXPOSURE_EV100: f32 = 13.5;
// how strongly the sky lights what the sun does not reach, as a share of its real strength:
// inside a shadow the sky is the only light, so this darkens shadows and leaves the sunlit
// side almost as it is. Below 1.0 is a look, not physics, settled by eye
const STILL_CAMERA_SKY_LIGHT_INTENSITY: f32 = 0.1;

// ---- sun --------------------------------------------------------------------
// where it shines from, looking at the origin: a directional light ignores its position
// and keeps only the direction, so this sets the angle alone. Low, about 20° up, and 15°
// right of where the still camera looks, so the sun is in view and every shadow falls
// towards the camera, long enough to read
const SUN_POSITION: Vec3 = Vec3::new(5.0, 3.6, -8.7);
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
    app.add_systems(
        OnEnter(Screen::Gameplay),
        (spawn_playroom, give_ui_cameras_hdr),
    );
    app.add_systems(OnExit(Screen::Gameplay), take_ui_cameras_hdr);
    app.add_plugins(sun_glare::plugin);
}

// ==== Systems ================================================================

/// Spawns the playroom's level, already loaded by the loading screen, the still camera
/// that shows it, the sun that lights it and the sky over it.
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
        Camera3d::default(),
        Camera {
            order: STILL_CAMERA_ORDER,
            ..default()
        },
        Exposure {
            ev100: STILL_CAMERA_EXPOSURE_EV100,
        },
        AtmosphereEnvironmentMapLight {
            intensity: STILL_CAMERA_SKY_LIGHT_INTENSITY,
            ..default()
        },
        AtmosphereSettings::default(),
        // a soft halo around the sun and whatever else is brighter than white
        Bloom::NATURAL,
        Transform::from_translation(STILL_CAMERA_POSITION).looking_at(STILL_CAMERA_TARGET, Vec3::Y),
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
        Transform::from_translation(SUN_POSITION).looking_at(Vec3::ZERO, Vec3::Y),
        DespawnOnExit(Screen::Gameplay),
    ));

    let medium = scattering_mediums.add(ScatteringMedium::earth(
        SKY_FALLOFF_RESOLUTION,
        SKY_PHASE_RESOLUTION,
    ));
    commands.spawn((Atmosphere::earth(medium), DespawnOnExit(Screen::Gameplay)));
}

/// Gives the UI cameras, the screen camera and the floating one above it, the still camera's
/// `Hdr`, which its atmosphere and `Bloom` bring. Cameras on one window share an image only
/// while their formats match: apart, the screen camera's opaque clear hides the level, its
/// blur never sees it, and the floating camera, which never clears, keeps showing the last
/// frame they shared.
fn give_ui_cameras_hdr(mut commands: Commands, cameras: Query<Entity, With<Camera2d>>) {
    for camera in &cameras {
        commands.entity(camera).insert(Hdr);
    }
}

/// Takes the UI cameras' `Hdr` away again: outside gameplay there is no level under them.
fn take_ui_cameras_hdr(mut commands: Commands, cameras: Query<Entity, With<Camera2d>>) {
    for camera in &cameras {
        commands.entity(camera).remove::<Hdr>();
    }
}
