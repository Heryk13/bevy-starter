use avian3d::prelude::{ColliderConstructor, ColliderConstructorHierarchy, RigidBody};
use bevy::anti_alias::smaa::Smaa;
use bevy::asset::uuid_handle;
use bevy::camera::{CameraUpdateSystems, Exposure, Hdr, visibility::RenderLayers};
use bevy::curve::cubic_splines::LinearSpline;
use bevy::light::atmosphere::ScatteringMedium;
use bevy::light::{Atmosphere, AtmosphereEnvironmentMapLight, SunDisk};
use bevy::pbr::{AtmosphereSettings, ScreenSpaceAmbientOcclusion};
use bevy::post_process::auto_exposure::{
    AutoExposure, AutoExposureCompensationCurve, AutoExposurePlugin,
};
use bevy::post_process::bloom::Bloom;
use bevy::prelude::*;
use std::ops::RangeInclusive;

use crate::screens::Screen;
use crate::screens::loading::GameplayAssets;
use crate::ui::blur::{ScreensCovered, ScreensCoveredSystems};

mod lighting;
mod sun_glare;

// ==== Config =================================================================

// ---- the camera kit: what every camera on the level carries -----------------
// below the screen camera's default order of 0, so the level is drawn first and the UI
// lands on top of it
const PLAYROOM_CAMERA_ORDER: isize = -1;
// outdoor daylight: the sun is RAW_SUNLIGHT, far too bright for the default exposure, made
// for indoor light. Each step up halves the brightness. Bevy's atmosphere example uses 13,
// this one was settled by eye. The eye below corrects it frame by frame, measuring the view
// as this exposure leaves it
const PLAYROOM_CAMERA_EXPOSURE_EV100: f32 = 13.5;
// the eye (`AutoExposure`): it brightens or darkens the view until its average brightness
// is Bevy's 1.0, moved by this many stops. Below zero, so daylight stays about where the
// exposure above put it, instead of washing out. Tuned by eye
const PLAYROOM_CAMERA_EYE_TARGET_EV: f32 = -2.0;
// how many stops at most the eye brightens a dark view past daylight: what keeps a dark
// room dark, with a little still seen. 3.0 and 1.5 left the insides too light. Tuned by
// eye
const PLAYROOM_CAMERA_EYE_DARK_LIMIT_EV: f32 = 0.5;
// how many stops at most it darkens a view brighter than daylight, Bevy's default
const PLAYROOM_CAMERA_EYE_BRIGHT_LIMIT_EV: f32 = 8.0;
// the eye's target as an asset, which `AutoExposure` asks for: a handle known before the
// asset is made, so the camera kit can name it, and `make_eye_curve` makes it at startup
const PLAYROOM_CAMERA_EYE_CURVE: Handle<AutoExposureCompensationCurve> =
    uuid_handle!("7b5109c9-7345-4d2c-b864-a313f72c65b4");
// how strongly the sky lights what the sun does not reach, as a share of its real strength:
// inside a shadow the sky is the only light, so lowering this darkens only the shadows and
// leaves the sunlit side almost as it is. 1.0 is physical, and SSAO darkens the creases
const PLAYROOM_CAMERA_SKY_LIGHT_INTENSITY: f32 = 1.0;
// how far from a crease SSAO looks for what blocks the sky: Bevy's 0.73 m shades a strip
// a few pixels thin seen from across the level, 40 m off, so this is wider, at the cost
// of some haloing and noise around objects, which Bevy warns of
const PLAYROOM_CAMERA_SSAO_RADIUS_M: f32 = 5.0;

// ---- sun --------------------------------------------------------------------
// the direction it shines from, given as a point looking at the origin: a directional
// light ignores its position and keeps only the direction, so the length is free and this
// sets the angle alone. Low, about 20° up, and a little right of -Z, the way the level is
// first seen, so the sun is in view and every shadow falls towards the camera, long
// enough to read
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

// ---- colliders --------------------------------------------------------------
// the water's surface gets none, so the player sinks to the pool's floor instead of walking
// on it. Bevy names a glTF mesh's entity `{mesh}.{material}`, a child of the Blender
// object's own entity, so renaming the mesh or its material in Blender means renaming
// it here
const POOL_WATER_MESH_NAME: &str = "Pool_Water.M_Water";

// ==== Plugin =================================================================

/// Shows the playroom: the level and its colliders, and its light. The camera is the player's.
pub(super) fn plugin(app: &mut App) {
    app.add_systems(
        OnEnter(Screen::Gameplay),
        (spawn_playroom, match_ui_cameras_to_playroom_camera),
    );
    app.add_systems(
        OnExit(Screen::Gameplay),
        (reset_ui_cameras, unload_gameplay_assets),
    );
    // the eye: Bevy's default plugins leave it out
    app.add_plugins(AutoExposurePlugin);
    app.add_systems(Startup, make_eye_curve);
    app.add_plugins(lighting::plugin);
    app.add_plugins(sun_glare::plugin);
    // between the two: after `ScreensCovered` is settled, and before Bevy works out what each
    // camera sees and builds its shadows, which it only does for a camera that is on. One
    // turned back on any later draws an empty frame, and the glass would copy that
    app.add_systems(
        PostUpdate,
        rest_playroom_cameras
            .after(ScreensCoveredSystems)
            .before(CameraUpdateSystems)
            .run_if(in_state(Screen::Gameplay)),
    );
}

// ==== Components =============================================================

/// A camera on the playroom: spawning it brings the camera kit, the exposure, sky light,
/// bloom and ambient occlusion the level is lit for, so every camera that shows the level,
/// whatever moves it, looks the same and keeps the UI cameras matched to it.
#[derive(Component)]
#[require(
    Camera3d,
    Camera {
        order: PLAYROOM_CAMERA_ORDER,
        ..default()
    },
    Exposure {
        ev100: PLAYROOM_CAMERA_EXPOSURE_EV100,
    },
    AtmosphereEnvironmentMapLight {
        intensity: PLAYROOM_CAMERA_SKY_LIGHT_INTENSITY,
        ..default()
    },
    AtmosphereSettings,
    // the eye: it gets used to the dark, up to its limit, as the view goes from the sun
    // into a room. Below the dark limit, a view is measured as at the limit
    AutoExposure {
        range: eye_range_ev(),
        compensation_curve: PLAYROOM_CAMERA_EYE_CURVE,
        ..default()
    },
    // none: outdoors the sky lights what the sun does not, and indoors, with the eye used
    // to the dark, an even ambient light would light the room the interiors keep the sky
    // out of
    AmbientLight {
        brightness: 0.0,
        ..default()
    },
    // a soft halo around the sun and whatever else is brighter than white
    Bloom::NATURAL,
    // darkens creases and contacts; it brings the depth and normal prepasses it reads,
    // and works only without MSAA
    ScreenSpaceAmbientOcclusion {
        radius: PLAYROOM_CAMERA_SSAO_RADIUS_M,
        ..default()
    },
    Msaa::Off,
    // smooths edges in MSAA's place, from the finished image, so SSAO keeps working
    Smaa
)]
pub(crate) struct PlayroomCamera;

// ==== Systems ================================================================

/// Makes the eye's target, the asset the camera kit names by its handle: the same number
/// of stops whatever the view's brightness, a flat line Bevy reads as a table, across the
/// brightness the eye measures.
fn make_eye_curve(mut curves: ResMut<Assets<AutoExposureCompensationCurve>>) {
    let measured = eye_range_ev();
    let flat = LinearSpline::new([
        Vec2::new(*measured.start(), PLAYROOM_CAMERA_EYE_TARGET_EV),
        Vec2::new(*measured.end(), PLAYROOM_CAMERA_EYE_TARGET_EV),
    ]);
    match AutoExposureCompensationCurve::from_curve(flat) {
        Ok(curve) => {
            if let Err(err) = curves.insert(&PLAYROOM_CAMERA_EYE_CURVE, curve) {
                error!("could not keep the eye's target: {err}");
            }
        }
        Err(err) => error!("could not make the eye's target: {err}"),
    }
}

/// Spawns the playroom's level, already loaded by the loading screen, as one static body
/// whose every mesh but the water becomes a collider once the scene is in, the sun that
/// lights it and the sky over it. It needs `GameplayAssets`, so gameplay is entered through
/// the loading screen, never straight from the menu.
fn spawn_playroom(
    mut commands: Commands,
    gameplay_assets: Res<GameplayAssets>,
    mut scattering_mediums: ResMut<Assets<ScatteringMedium>>,
) {
    commands.spawn((
        WorldAssetRoot(gameplay_assets.playroom.clone()),
        RigidBody::Static,
        ColliderConstructorHierarchy::new(ColliderConstructor::TrimeshFromMesh)
            .without_constructor_for_name(POOL_WATER_MESH_NAME),
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

/// Matches the UI cameras, the screen camera and the layer cameras above it, to the
/// playroom's camera: its `Hdr`, which its atmosphere and `Bloom` bring, and its
/// `Msaa::Off`, which its SSAO needs. Cameras on one window share an image only while their
/// format and MSAA match: apart, the screen camera's opaque clear hides the level, its blur
/// never sees it, and the layer cameras, which never clear, keep showing the last frame
/// they shared. Sharing an image is not sharing its clear, though: each camera clears it the
/// first time it draws. With MSAA on, Bevy copied the cameras below into each one's MSAA
/// image first, which hid that; without it, the screen camera must not clear at all.
/// The screen camera also loses its render layers, as the layer cameras never had any: a
/// `Camera2d` draws whatever the world draws on layer 0, and the level's gizmos would show
/// through it as a tiny map in the middle of the screen, a metre to a pixel. UI ignores
/// render layers, so the menus still reach it.
fn match_ui_cameras_to_playroom_camera(
    mut commands: Commands,
    cameras: Query<Entity, With<Camera2d>>,
    screen_camera: Single<(Entity, &mut Camera), With<IsDefaultUiCamera>>,
) {
    for camera in &cameras {
        commands.entity(camera).insert((Hdr, Msaa::Off));
    }
    let (screen_entity, mut screen_camera) = screen_camera.into_inner();
    commands.entity(screen_entity).insert(RenderLayers::none());
    screen_camera.clear_color = ClearColorConfig::None;
}

/// Stops every playroom camera while the screens are wholly hidden behind a layer's blur,
/// as under the pause menu once its copy is made, and starts it again the moment they are
/// not. The level is the costliest thing drawn, its sky, shadows and ambient occlusion
/// included, and none of it would show: a paused game then costs less than a running one.
fn rest_playroom_cameras(
    screens_covered: Res<ScreensCovered>,
    mut cameras: Query<&mut Camera, With<PlayroomCamera>>,
) {
    for mut camera in &mut cameras {
        // compared first: writing the same value would still mark the camera changed
        if camera.is_active == screens_covered.0 {
            camera.is_active = !screens_covered.0;
        }
    }
}

/// Returns the UI cameras to their defaults: outside gameplay there is no level under them.
/// `Msaa` is put back rather than removed, since every camera needs one, while the screen
/// camera's `RenderLayers` is removed, so it is back on layer 0 as `main.rs` spawned it. The
/// layer cameras are left out of that: their `RenderLayers::none()` is their own, for good.
fn reset_ui_cameras(
    mut commands: Commands,
    cameras: Query<Entity, With<Camera2d>>,
    screen_camera: Single<(Entity, &mut Camera), With<IsDefaultUiCamera>>,
) {
    for camera in &cameras {
        commands
            .entity(camera)
            .remove::<Hdr>()
            .insert(Msaa::default());
    }
    let (screen_entity, mut screen_camera) = screen_camera.into_inner();
    commands.entity(screen_entity).remove::<RenderLayers>();
    screen_camera.clear_color = ClearColorConfig::Default;
}

/// Drops the playroom's handles once gameplay ends, so Bevy can free the level's memory
/// while the player is in the menu. A load from the pause menu loads it again.
fn unload_gameplay_assets(mut commands: Commands) {
    commands.remove_resource::<GameplayAssets>();
}

// ==== Helpers ================================================================

/// Returns the brightness the eye measures, in stops: from its dark limit below its target
/// to its bright limit above. Bevy measures a darker view as the darkest of this, so the
/// eye brightens it no further.
fn eye_range_ev() -> RangeInclusive<f32> {
    PLAYROOM_CAMERA_EYE_TARGET_EV - PLAYROOM_CAMERA_EYE_DARK_LIMIT_EV
        ..=PLAYROOM_CAMERA_EYE_TARGET_EV + PLAYROOM_CAMERA_EYE_BRIGHT_LIMIT_EV
}

// ==== Tests ==================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_playroom_camera_brings_what_the_ui_cameras_are_matched_to() {
        let mut world = World::new();
        let camera = world.spawn(PlayroomCamera).id();
        let camera = world.entity(camera);

        assert!(camera.contains::<Camera3d>());
        assert!(camera.contains::<Hdr>());
        assert_eq!(camera.get::<Msaa>(), Some(&Msaa::Off));
        assert_eq!(
            camera.get::<Camera>().map(|camera| camera.order),
            Some(PLAYROOM_CAMERA_ORDER)
        );
    }

    #[test]
    fn the_eye_brightens_the_darkest_view_by_its_dark_limit_alone() {
        // Bevy's correction is the target less the brightness measured
        let darkest_correction_ev = PLAYROOM_CAMERA_EYE_TARGET_EV - *eye_range_ev().start();
        assert!((darkest_correction_ev - PLAYROOM_CAMERA_EYE_DARK_LIMIT_EV).abs() < 1e-6);
    }

    #[test]
    fn a_playroom_camera_has_the_eye_and_no_ambient_light() {
        let mut world = World::new();
        let camera = world.spawn(PlayroomCamera).id();
        let camera = world.entity(camera);

        assert!(camera.contains::<AutoExposure>());
        assert_eq!(
            camera.get::<AmbientLight>().map(|light| light.brightness),
            Some(0.0)
        );
    }
}
