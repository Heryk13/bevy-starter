// The glass a floating root and its card are drawn with: `src/ui/blur.rs` blurs what the
// layers below left into a copy that covers the screen at half its resolution, and this reads
// the part of it right behind the node, lays the tint over it, fades the whole by the node's
// alpha and cuts the node's rounded corners. Under Reduce Transparency it draws the tint alone.
//
// Bind group 0 is Bevy's for every UI material (the view, then the globals); the material's
// own bindings live in group 1, in the order of its fields in `GlassMaterial`.

import bevy_ui_render::ui_vertex_output::UiVertexOutput;
import bevy_render::view::View;

struct GlassMaterial {
    tint: vec4<f32>,
    alpha: f32,
    // 1 under Reduce Transparency, 0 otherwise
    flat: u32
}

@group(0) @binding(0) var<uniform> view: View;

@group(1) @binding(0) var<uniform> material: GlassMaterial;
@group(1) @binding(1) var copy: texture_2d<f32>;
@group(1) @binding(2) var copy_sampler: sampler;

@fragment
fn fragment(in: UiVertexOutput) -> @location(0) vec4<f32> {
    // in.uv runs across the node, but the copy covers the screen: the fragment's pixel over the
    // screen's size finds the part of it behind the node
    let screen_uv = in.position.xy / view.viewport.zw;
    let blurred = textureSample(copy, copy_sampler, screen_uv);
    // the tint's own alpha is how dark it lays over the blur; the material's alpha fades the
    // glass in and out with its root
    let glass = mix(blurred.rgb, material.tint.rgb, material.tint.a);
    // flat, nothing was blurred and the copy holds nothing to read: the tint alone is drawn,
    // as opaque as its own alpha, so a backdrop dims the sharp screen and a card is solid
    let flat = material.flat == 1u;
    let color = select(glass, material.tint.rgb, flat);
    let opacity = select(1.0, material.tint.a, flat);
    // Bevy hands a material the whole rectangle of its node, so the rounded corners are cut
    // here: in.uv runs 0 to 1 across the node, which makes this the pixel counted from its center
    let point = (in.uv - 0.5) * in.size;
    let distance = sd_rounded_box(point, in.size, in.border_radius_x.x);
    return vec4(color, opacity * material.alpha * antialias(distance));
}

// How far a point is from the edge of a box with rounded corners, in pixels: negative inside,
// positive outside. `point` is counted from the box's center.
//
// ponytail: one round radius for all four corners, the top left one's. A node whose corners
// differ, or are ellipses, needs `sd_rounded_box` from `bevy_ui_render/src/ui.wesl` copied
// here with the helpers it calls, since a material cannot import them.
fn sd_rounded_box(point: vec2<f32>, size: vec2<f32>, radius: f32) -> f32 {
    // from the center of the nearest corner's circle to the point
    let q = abs(point) - 0.5 * size + radius;
    return length(max(q, vec2(0.0))) + min(max(q.x, q.y), 0.0) - radius;
}

// How much of a pixel this far from the edge is covered: all of it half a pixel inside, none
// half a pixel outside, so the edge is a one pixel ramp instead of a staircase. The same as
// Bevy's own, which a material cannot import either.
fn antialias(distance: f32) -> f32 {
    return saturate(0.5 - distance);
}
