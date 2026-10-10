use bevy::asset::RenderAssetUsages;
use bevy::camera::visibility::VisibilitySystems;
use bevy::light::NotShadowCaster;
use bevy::prelude::*;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};

use super::SUN_POSITION;
use crate::screens::Screen;

// ==== Config =================================================================

// ---- look -------------------------------------------------------------------
// how wide the glare is on screen, edge to edge, in degrees of the still camera's view
const SUN_GLARE_DEGREES: f32 = 30.0;
// the glare is three layers added together; every size below is a share of its half-width.
// The blinding core: a white ball, wide enough to hide the sun's shape
const SUN_GLARE_CORE_RADIUS_SHARE: f32 = 0.14;
// the wide glow that lightens the sky around the core, and how far out it reaches before
// fading to about a third
const SUN_GLARE_HALO_STRENGTH: f32 = 0.3;
const SUN_GLARE_HALO_REACH_SHARE: f32 = 0.12;
// the rays, as the blades of a camera's aperture make them: how many, how narrow (the
// higher, the thinner), how bright, and how far out each reaches before fading to about a
// third. Short, as in a photo
const SUN_GLARE_RAY_COUNT: u32 = 6;
const SUN_GLARE_RAY_SHARPNESS: f32 = 200.0;
const SUN_GLARE_RAY_STRENGTH: f32 = 0.4;
const SUN_GLARE_RAY_LENGTH_SHARE: f32 = 0.09;
// where the glare starts fading to nothing, so the square it is drawn on never shows its
// edge
const SUN_GLARE_FADE_START_SHARE: f32 = 0.6;
// warm white, past 1.0 so the core blows out and feeds `Bloom`; unlit, so no exposure
// dims it on the way to the screen
const SUN_GLARE_COLOR: Color = Color::linear_rgb(4.0, 3.6, 3.0);

// ---- placement --------------------------------------------------------------
// how far from the camera the glare stands: past the whole level, so nothing behind it is
// hidden, and short of the camera's far plane of 1000 m, so it is drawn at all
const SUN_GLARE_DISTANCE_M: f32 = 900.0;
// the side of the generated texture
const SUN_GLARE_TEXTURE_PX: u32 = 256;

// ==== Plugin =================================================================

/// Draws the sun as a camera sees it: a blinding glow with rays, hidden by whatever stands
/// in front of it.
pub(super) fn plugin(app: &mut App) {
    app.add_systems(OnEnter(Screen::Gameplay), spawn_sun_glare);
    // after propagation, so the camera's place in the world is known even on its first frame
    // and even as the child of a body, and before culling, which reads the glare's own
    app.add_systems(
        PostUpdate,
        place_sun_glare
            .after(TransformSystems::Propagate)
            .before(VisibilitySystems::CheckVisibility)
            .run_if(in_state(Screen::Gameplay)),
    );
}

// ==== Components =============================================================

/// Marks the square the sun's glare is drawn on.
#[derive(Component)]
struct SunGlare;

// ==== Systems ================================================================

/// Spawns the square the glare is drawn on, its texture generated from `glare_brightness`.
fn spawn_sun_glare(
    mut commands: Commands,
    mut images: ResMut<Assets<Image>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let size_m = glare_size_m(SUN_GLARE_DISTANCE_M, SUN_GLARE_DEGREES);
    commands.spawn((
        SunGlare,
        Mesh3d(meshes.add(Rectangle::new(size_m, size_m))),
        MeshMaterial3d(materials.add(StandardMaterial {
            base_color: SUN_GLARE_COLOR,
            base_color_texture: Some(images.add(glare_image())),
            unlit: true,
            // only adds light, so the sky behind it is never darkened
            alpha_mode: AlphaMode::Add,
            fog_enabled: false,
            ..default()
        })),
        // standing in the sun's path, it would otherwise shade the whole level
        NotShadowCaster,
        DespawnOnExit(Screen::Gameplay),
    ));
}

/// Keeps the glare on the sun, at its distance from the still camera and facing it. It runs
/// after propagation, so it sets the glare's `GlobalTransform` too: it has no parent, so the
/// two are the same.
fn place_sun_glare(
    // the one 3D camera: a second one, from a glTF or a branch below, would skip this silently
    camera: Single<&GlobalTransform, (With<Camera3d>, Without<SunGlare>)>,
    glare: Single<(&mut Transform, &mut GlobalTransform), With<SunGlare>>,
) {
    let (mut transform, mut global_transform) = glare.into_inner();
    let camera_position = camera.translation();
    let position = camera_position + SUN_POSITION.normalize() * SUN_GLARE_DISTANCE_M;
    // the rectangle faces +Z, so its -Z looks away from the camera
    *transform =
        Transform::from_translation(position).looking_to(position - camera_position, Vec3::Y);
    *global_transform = GlobalTransform::from(*transform);
}

// ==== Helpers ================================================================

/// Builds the glare's texture: `glare_brightness` sampled across a square, as grey.
fn glare_image() -> Image {
    let side = SUN_GLARE_TEXTURE_PX;
    let mut data = Vec::with_capacity((side * side * 4) as usize);
    for y in 0..side {
        for x in 0..side {
            // from -1 to 1 across the square, sampled at each texel's centre
            let offset = (Vec2::new(x as f32, y as f32) + 0.5) / side as f32 * 2.0 - 1.0;
            let brightness = glare_brightness(offset);
            data.extend_from_slice(
                &Color::linear_rgb(brightness, brightness, brightness)
                    .to_srgba()
                    .to_u8_array(),
            );
        }
    }
    Image::new(
        Extent3d {
            width: side,
            height: side,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        data,
        // sRGB keeps more of the faint tail than linear 8-bit would
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::RENDER_WORLD,
    )
}

/// Returns how bright the glare is at `offset` from its centre, measured in half-widths of
/// its square: a blinding core, a wide glow around it and short rays, added together and
/// all reaching zero by the square's edge.
fn glare_brightness(offset: Vec2) -> f32 {
    let radius = offset.length();
    let core = (-(radius / SUN_GLARE_CORE_RADIUS_SHARE).powi(2)).exp();
    let halo = SUN_GLARE_HALO_STRENGTH * (-radius / SUN_GLARE_HALO_REACH_SHARE).exp();
    let angle = offset.y.atan2(offset.x);
    let ray_count = SUN_GLARE_RAY_COUNT as f32;
    let ray = ((1.0 + (ray_count * angle).cos()) / 2.0).powf(SUN_GLARE_RAY_SHARPNESS);
    let rays = SUN_GLARE_RAY_STRENGTH * ray * (-radius / SUN_GLARE_RAY_LENGTH_SHARE).exp();
    let fade = 1.0
        - EaseFunction::SmoothStep.sample_clamped(
            (radius - SUN_GLARE_FADE_START_SHARE) / (1.0 - SUN_GLARE_FADE_START_SHARE),
        );
    ((core + halo + rays) * fade).clamp(0.0, 1.0)
}

/// Returns the side, in metres, of a square at `distance_m` that spans `degrees` of view.
fn glare_size_m(distance_m: f32, degrees: f32) -> f32 {
    2.0 * distance_m * (degrees.to_radians() / 2.0).tan()
}

// ==== Tests ==================================================================

#[cfg(test)]
mod tests {
    use std::f32::consts::TAU;

    use super::*;

    const EPSILON: f32 = 1e-5;
    // halfway from the core to where the fade starts, so clearly dimmer than the centre yet
    // untouched by the fade, wherever those two constants move
    const HALFWAY: f32 = (SUN_GLARE_CORE_RADIUS_SHARE + SUN_GLARE_FADE_START_SHARE) / 2.0;

    #[test]
    fn glare_is_brightest_at_its_centre() {
        let centre = glare_brightness(Vec2::ZERO);
        assert!(centre > glare_brightness(Vec2::new(HALFWAY, 0.0)));
        assert!(centre > glare_brightness(Vec2::new(0.0, SUN_GLARE_CORE_RADIUS_SHARE)));
    }

    #[test]
    fn glare_is_dark_at_the_edge_of_its_square() {
        assert!(glare_brightness(Vec2::new(1.0, 0.0)).abs() < EPSILON);
        assert!(glare_brightness(Vec2::new(0.0, -1.0)).abs() < EPSILON);
        assert!(glare_brightness(Vec2::new(1.0, 1.0)).abs() < EPSILON);
    }

    #[test]
    fn glare_is_brighter_along_a_ray_than_between_two() {
        // past the core, where a ray still has a good share of its strength
        let radius = SUN_GLARE_CORE_RADIUS_SHARE + SUN_GLARE_RAY_LENGTH_SHARE;
        let between = TAU / SUN_GLARE_RAY_COUNT as f32 / 2.0;
        let on_ray = glare_brightness(Vec2::new(radius, 0.0));
        let off_ray = glare_brightness(Vec2::from_angle(between) * radius);
        assert!(on_ray > off_ray + 0.01);
    }

    #[test]
    fn glare_spans_the_angle_it_is_given() {
        let size_m = glare_size_m(SUN_GLARE_DISTANCE_M, 90.0);
        assert!((size_m - 2.0 * SUN_GLARE_DISTANCE_M).abs() < 1e-2);
    }
}
