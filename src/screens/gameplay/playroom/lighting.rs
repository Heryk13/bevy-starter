use bevy::gltf::GltfExtras;
use bevy::light::{EnvironmentMapLight, LightProbe};
use bevy::prelude::*;
use serde_json::Value;

use crate::screens::Screen;

// ==== Config =================================================================

// ---- the level's marks: custom properties set in Blender --------------------
// the property naming what an empty is, and the `kind` of one whose box is an interior, of
// one that is a lamp and of one where the sky's light comes in, the way the level names a
// door "door"
const KIND_PROPERTY: &str = "kind";
const INTERIOR_KIND: &str = "interior";
const LAMP_KIND: &str = "light_point";
const SKY_PORTAL_KIND: &str = "sky_portal";
// the property that says how much of the sky gets into an interior
const SKY_LIGHT_PROPERTY: &str = "sky_light";
// how wide a Cube empty of display size 1 is before its scale: it spans -1 to 1, where
// Bevy's light probe spans -0.5 to 0.5
const BLENDER_CUBE_SIZE_M: f32 = 2.0;

// ---- interiors --------------------------------------------------------------
// how much of the sky gets into an interior whose empty says nothing: 0.0 is none, 1.0 is
// as much as outside. Next to none, so far from its openings an interior is dark, and a
// sky portal is the light at the end of the tunnel. Tuned by eye
const INTERIOR_SKY_LIGHT: f32 = 0.005;
// the sky's brightness in cd/m², what a `sky_light` of 1.0 and a sky portal stand for: the
// sky's light on the ground with the sun low, about 10 000 lux, spread over the sky's half
// of the view. 5 000 lit the floor at a portal brighter than the ground outside. Tuned
// once, so a portal's floor is never brighter than outside, then left alone
const SKY_LUMINANCE_NITS: f32 = 3_000.0;
// how far into an interior's walls its darkness fades into the sky's light: the empty is
// drawn on the walls' inner faces, and the box grows by this on every side, so the fade
// happens inside the walls. At an opening it lies on the floor, and with a sky portal there
// a fade of any width is a step, dark or bright, so this is as thin as a fade can be and
// still hide the box's edge: 0.1 m drew a line across the corridor's threshold. Past the
// level's 0.2 m walls the outside of a wall would darken too
const INTERIOR_BLEND_M: f32 = 0.02;

// ---- lamps ------------------------------------------------------------------
// how much light a lamp gives, in lumens, every way at once. Far past a real bulb's 800:
// the eye stops a few stops short of a real one's adaptation to the dark, so a lamp has to
// stand up to a sunny day's exposure. Tuned by eye
const LAMP_INTENSITY_LM: f32 = 100_000.0;
// how far a lamp reaches, and its shadows with it: past the room it hangs in
const LAMP_RANGE_M: f32 = 12.0;

// ---- sky portals: the sky's light coming in at an opening -------------------
// the sky's blue, paler than the sky drawn, as the light it sheds on a wall
const SKY_PORTAL_COLOR: Color = Color::srgb(0.75, 0.85, 1.0);
// how far in it reaches: the light fades with the distance well before this
const SKY_PORTAL_RANGE_M: f32 = 12.0;

// ==== Plugin =================================================================

/// Lights the level from what its Blender file marks, by each empty's `kind`: an interior
/// keeps the sky out of its box, a lamp hangs a light, and a sky portal lets the sky's light
/// in at an opening.
pub(super) fn plugin(app: &mut App) {
    app.init_resource::<InteriorLight>();
    app.add_systems(
        Update,
        (spawn_interiors, spawn_lamps, spawn_sky_portals).run_if(in_state(Screen::Gameplay)),
    );
}

// ==== Resources ==============================================================

/// The light inside an interior, standing in for the sky's: one white pixel on every face
/// of a cubemap, made once and shared by every box, each at its own intensity.
#[derive(Resource)]
struct InteriorLight(EnvironmentMapLight);

impl FromWorld for InteriorLight {
    fn from_world(world: &mut World) -> Self {
        let mut images = world.resource_mut::<Assets<Image>>();
        Self(EnvironmentMapLight::solid_color(&mut images, Color::WHITE))
    }
}

// ==== Systems ================================================================

/// Gives each interior a light probe the size of its box and a margin past it, lit at its
/// `sky_light`. A probe that covers a surface wholly replaces the camera's sky there, which
/// is what keeps the sky out. A `sky_light` that cannot be read is warned of, and the
/// default is used.
fn spawn_interiors(
    mut commands: Commands,
    marks: Query<(Entity, &GltfExtras, &Transform, Option<&Name>), Added<GltfExtras>>,
    interior_light: Res<InteriorLight>,
) {
    for (entity, extras, transform, name) in &marks {
        let Some(extras) = marked(extras, INTERIOR_KIND) else {
            continue;
        };
        let name = name.map_or("an interior", Name::as_str);
        let sky_light = sky_light(&extras).unwrap_or_else(|bad| {
            warn!(
                "{name} has {SKY_LIGHT_PROPERTY} {bad}, not a number from 0 to 1: using {INTERIOR_SKY_LIGHT}"
            );
            INTERIOR_SKY_LIGHT
        });
        // a mirrored empty has a negative scale, and its box is as big
        let box_size_m = transform.scale.abs() * BLENDER_CUBE_SIZE_M;
        // a child of the empty, so its scale is over the empty's own, the sign included
        let scale = probe_size_m(box_size_m) / transform.scale;
        if !scale.is_finite() {
            warn!("{name} is scaled to nothing on an axis: it keeps no sky out");
            continue;
        }
        commands.entity(entity).with_child((
            LightProbe {
                falloff: probe_falloff(box_size_m),
            },
            interior_light
                .0
                .clone()
                .with_intensity(sky_light * SKY_LUMINANCE_NITS),
            Transform::from_scale(scale),
        ));
    }
}

/// Hangs a point light with shadows from each lamp: a bulb that lights every way, the
/// ceiling included. Its shadow is a cubemap, six passes where a spot's is one, which a few
/// lamps can afford.
fn spawn_lamps(mut commands: Commands, marks: Query<(Entity, &GltfExtras), Added<GltfExtras>>) {
    for (entity, extras) in &marks {
        if marked(extras, LAMP_KIND).is_none() {
            continue;
        }
        commands.entity(entity).with_child(PointLight {
            intensity: LAMP_INTENSITY_LM,
            range: LAMP_RANGE_M,
            shadow_maps_enabled: true,
            ..default()
        });
    }
}

/// Lights each sky portal, an Image empty scaled to an opening, with a `RectLight` its size
/// shining along its Z in Blender: the sky's light coming in through the whole opening,
/// bright at it and fading inside, where an interior's box can only change all at once. It
/// shows the sky the interiors keep out, so a bigger opening lets more in. A `RectLight`
/// casts no shadows in Bevy 0.20, so it lights through walls within its range: an opening
/// near another room lights that room's side of the wall a little.
fn spawn_sky_portals(
    mut commands: Commands,
    marks: Query<(Entity, &GltfExtras, &Transform), Added<GltfExtras>>,
) {
    for (entity, extras, transform) in &marks {
        if marked(extras, SKY_PORTAL_KIND).is_none() {
            continue;
        }
        // Blender's X and Y, the opening's sides, are the glTF's X and Z
        let area_m2 = (transform.scale.x * transform.scale.z).abs();
        commands.entity(entity).with_child((
            // a unit rectangle: the empty's scale, which Bevy applies to a `RectLight`, makes
            // it the opening's size
            RectLight {
                color: SKY_PORTAL_COLOR,
                intensity: sky_portal_lumens(area_m2),
                range: SKY_PORTAL_RANGE_M,
                width: 1.0,
                height: 1.0,
            },
            // a `RectLight` shines along its -Z, here the empty's Z in Blender, the glTF's
            // +Y; set back half the interiors' margin, in the middle of the strip of floor
            // where an interior fades out at the opening: on the opening the strip was
            // behind the light, a dark line, and at the margin's edge right in front of
            // it, sky and light both, a bright one. Over the empty's Z scale in Blender,
            // left at 1
            Transform::from_translation(Vec3::NEG_Y * INTERIOR_BLEND_M / 2.0)
                .looking_to(Vec3::Y, Vec3::Z),
        ));
    }
}

// ==== Helpers ================================================================

/// Returns an empty's custom properties when its `kind` is `kind`, and `None` for any
/// other kind, none, or properties that are not JSON.
fn marked(extras: &GltfExtras, kind: &str) -> Option<Value> {
    let properties = serde_json::from_str::<Value>(&extras.value).ok()?;
    (properties.get(KIND_PROPERTY)?.as_str()? == kind).then_some(properties)
}

/// Returns the lumens a sky portal of `area_m2` lets in: the sky's brightness over its
/// area, times π, what a surface that bright gives off on one side.
fn sky_portal_lumens(area_m2: f32) -> f32 {
    std::f32::consts::PI * SKY_LUMINANCE_NITS * area_m2
}

/// Returns an interior's `sky_light`, or the default when it has none. `Err` holds what it
/// has instead when that is not a number from 0 to 1.
fn sky_light(extras: &Value) -> Result<f32, String> {
    let Some(value) = extras.get(SKY_LIGHT_PROPERTY) else {
        return Ok(INTERIOR_SKY_LIGHT);
    };
    match value.as_f64() {
        Some(share) if (0.0..=1.0).contains(&share) => Ok(share as f32),
        _ => Err(value.to_string()),
    }
}

/// Returns how big an interior's probe is: its box, grown by the margin on every side.
fn probe_size_m(box_size_m: Vec3) -> Vec3 {
    box_size_m + 2.0 * INTERIOR_BLEND_M
}

/// Returns the probe's falloff, so it fades over the margin alone. Bevy's shader
/// (`light_probe.wesl`) fades each face over the falloff's share of the whole size, not
/// half of it as `LightProbe`'s docs say: a falloff twice this let the sky into the
/// corridor and onto the floors.
fn probe_falloff(box_size_m: Vec3) -> Vec3 {
    INTERIOR_BLEND_M / probe_size_m(box_size_m)
}

// ==== Tests ==================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn extras(properties: &Value) -> GltfExtras {
        GltfExtras {
            value: properties.to_string(),
        }
    }

    #[test]
    fn an_empty_is_marked_by_its_kind_alone() {
        let interior = extras(&json!({ "kind": "interior", "sky_light": 0.3 }));
        assert!(marked(&interior, INTERIOR_KIND).is_some());
        assert!(marked(&interior, LAMP_KIND).is_none());
        assert!(marked(&extras(&json!({ "sky_light": 0.3 })), INTERIOR_KIND).is_none());
    }

    #[test]
    fn properties_that_are_not_json_mark_nothing() {
        let broken = GltfExtras {
            value: "{ kind: interior".to_string(),
        };
        assert!(marked(&broken, INTERIOR_KIND).is_none());
    }

    #[test]
    fn an_interior_without_sky_light_takes_the_default() {
        assert_eq!(
            sky_light(&json!({ "kind": "interior" })),
            Ok(INTERIOR_SKY_LIGHT)
        );
    }

    #[test]
    fn an_interior_keeps_its_own_sky_light() {
        let share = sky_light(&json!({ "sky_light": 0.3 })).unwrap_or(f32::NAN);
        assert!((share - 0.3).abs() < 1e-6);
    }

    #[test]
    fn a_sky_light_outside_0_to_1_or_not_a_number_is_refused() {
        assert!(sky_light(&json!({ "sky_light": 1.5 })).is_err());
        assert!(sky_light(&json!({ "sky_light": -0.1 })).is_err());
        assert!(sky_light(&json!({ "sky_light": "dark" })).is_err());
    }

    #[test]
    fn a_portal_twice_as_big_lets_twice_the_sky_in() {
        let small_lm = sky_portal_lumens(1.8 * 3.0);
        assert!((sky_portal_lumens(2.0 * 1.8 * 3.0) - 2.0 * small_lm).abs() < 1.0);
        assert!((sky_portal_lumens(1.0) / std::f32::consts::PI - SKY_LUMINANCE_NITS).abs() < 1e-3);
    }

    #[test]
    fn the_probe_grows_past_the_box_by_the_margin_on_every_side() {
        let box_size_m = Vec3::new(10.0, 3.0, 2.0);
        let grown = probe_size_m(box_size_m) - box_size_m;
        assert!(grown.abs_diff_eq(Vec3::splat(2.0 * INTERIOR_BLEND_M), 1e-6));
    }

    #[test]
    fn the_probe_fades_over_the_margin_alone() {
        let box_size_m = Vec3::new(10.0, 3.0, 2.0);
        // the shader fades each face over the falloff's share of the whole size
        let fade_m = probe_falloff(box_size_m) * probe_size_m(box_size_m);
        assert!(fade_m.abs_diff_eq(Vec3::splat(INTERIOR_BLEND_M), 1e-6));
    }
}
