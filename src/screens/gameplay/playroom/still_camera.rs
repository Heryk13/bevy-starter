use bevy::anti_alias::smaa::Smaa;
use bevy::camera::{Exposure, Hdr};
use bevy::light::AtmosphereEnvironmentMapLight;
use bevy::pbr::{AtmosphereSettings, ScreenSpaceAmbientOcclusion};
use bevy::post_process::bloom::Bloom;
use bevy::prelude::*;

use crate::screens::Screen;

// ==== Config =================================================================

// ---- still camera: framed to judge the sky, the sun and the shadows ---------
// where it stands, in metres, about 41 m from the level's centre
const STILL_CAMERA_POSITION_M: Vec3 = Vec3::new(-10.0, 3.0, 40.0);
// what it looks at: high over the origin, so it tilts about 14° up and the horizon sits in
// the bottom fifth of the default 45° view, the level along the bottom and the sky above
const STILL_CAMERA_TARGET_M: Vec3 = Vec3::new(0.0, 13.0, 0.0);
// below the screen camera's default order of 0, so the level is drawn first and the UI
// lands on top of it
const STILL_CAMERA_ORDER: isize = -1;
// outdoor daylight: the sun is RAW_SUNLIGHT, far too bright for the default exposure, made
// for indoor light. Each step up halves the brightness. Bevy's atmosphere example uses 13,
// this one was settled by eye
const STILL_CAMERA_EXPOSURE_EV100: f32 = 13.5;
// how strongly the sky lights what the sun does not reach, as a share of its real strength:
// inside a shadow the sky is the only light, so lowering this darkens only the shadows and
// leaves the sunlit side almost as it is. 1.0 is physical, and SSAO darkens the creases
const STILL_CAMERA_SKY_LIGHT_INTENSITY: f32 = 1.0;
// how far from a crease SSAO looks for what blocks the sky: Bevy's 0.73 m shades a strip
// a few pixels thin from 40 m away, so this is wider, at the cost of some haloing and
// noise around objects, which Bevy warns of
const STILL_CAMERA_SSAO_RADIUS_M: f32 = 5.0;

// ==== Plugin =================================================================

/// Shows the playroom through a still camera, with the UI cameras matched to it.
pub(super) fn plugin(app: &mut App) {
    app.add_systems(
        OnEnter(Screen::Gameplay),
        (spawn_still_camera, match_ui_cameras_to_still_camera),
    );
    app.add_systems(OnExit(Screen::Gameplay), reset_ui_cameras);
}

// ==== Systems ================================================================

/// Spawns the still camera that shows the playroom, with the exposure, sky light, bloom and
/// ambient occlusion the level is lit for.
fn spawn_still_camera(mut commands: Commands) {
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
        // darkens creases and contacts; it brings the depth and normal prepasses it reads,
        // and works only without MSAA
        ScreenSpaceAmbientOcclusion {
            radius: STILL_CAMERA_SSAO_RADIUS_M,
            ..default()
        },
        Msaa::Off,
        // smooths edges in MSAA's place, from the finished image, so SSAO keeps working
        Smaa::default(),
        Transform::from_translation(STILL_CAMERA_POSITION_M)
            .looking_at(STILL_CAMERA_TARGET_M, Vec3::Y),
        DespawnOnExit(Screen::Gameplay),
    ));
}

/// Matches the UI cameras, the screen camera and the floating one above it, to the still
/// camera: its `Hdr`, which its atmosphere and `Bloom` bring, and its `Msaa::Off`, which
/// its SSAO needs. Cameras on one window share an image only while their format and MSAA
/// match: apart, the screen camera's opaque clear hides the level, its blur never sees it,
/// and the floating camera, which never clears, keeps showing the last frame they shared.
/// Sharing an image is not sharing its clear, though: each camera clears it the first time
/// it draws. With MSAA on, Bevy copied the cameras below into each one's MSAA image first,
/// which hid that; without it, the screen camera must not clear at all.
fn match_ui_cameras_to_still_camera(
    mut commands: Commands,
    cameras: Query<Entity, With<Camera2d>>,
    mut screen_camera: Single<&mut Camera, With<IsDefaultUiCamera>>,
) {
    for camera in &cameras {
        commands.entity(camera).insert((Hdr, Msaa::Off));
    }
    screen_camera.clear_color = ClearColorConfig::None;
}

/// Returns the UI cameras to their defaults: outside gameplay there is no level under them.
/// `Msaa` is put back rather than removed, since every camera needs one.
fn reset_ui_cameras(
    mut commands: Commands,
    cameras: Query<Entity, With<Camera2d>>,
    mut screen_camera: Single<&mut Camera, With<IsDefaultUiCamera>>,
) {
    for camera in &cameras {
        commands
            .entity(camera)
            .remove::<Hdr>()
            .insert(Msaa::default());
    }
    screen_camera.clear_color = ClearColorConfig::Default;
}
