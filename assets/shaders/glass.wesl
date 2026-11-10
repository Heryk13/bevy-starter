// The glass a floating root is drawn with: `src/ui/blur.rs` blurs what the layers below left
// into a copy that covers the screen at half its resolution, and this reads the part of it
// right behind the node, lays the tint over it and fades the whole by the node's alpha.
//
// Bind group 0 is Bevy's for every UI material (the view, then the globals); the material's
// own bindings live in group 1, in the order of its fields in `GlassMaterial`.

import bevy_ui_render::ui_vertex_output::UiVertexOutput;
import bevy_render::view::View;

struct GlassMaterial {
    tint: vec4<f32>,
    alpha: f32
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
    let color = mix(blurred.rgb, material.tint.rgb, material.tint.a);
    return vec4(color, material.alpha);
}
