// One doubling of the glass's ascent, the other half of Marius Bjørge's dual Kawase filter
// (SIGGRAPH 2015): `src/ui/blur.rs` runs it once per level on the way back up, each writing a
// texture twice the size of the one it reads, which smooths away the blocks the descent left.
//
// Texels are sampled rather than loaded: with a filtering sampler, a read that lands on the
// corner between four texels averages all four for the price of one.

import bevy_core_pipeline::fullscreen_vertex_shader::FullscreenVertexOutput;

@group(0) @binding(0) var source: texture_2d<f32>;
@group(0) @binding(1) var source_sampler: sampler;
// how far apart the reads land, in texels of the source
@group(0) @binding(2) var<uniform> offset: f32;

@fragment
fn fragment(in: FullscreenVertexOutput) -> @location(0) vec4<f32> {
    let uv = in.uv;
    let size = vec2<f32>(textureDimensions(source));
    let texel = 1.0 / size;
    let t = texel * offset;
    // the cross reaches twice as far as the diagonals, filling the gaps between them so the
    // doubled image does not come out as a checkerboard
    let left = uv - vec2(2.0 * t.x, 0.0);
    let right = uv + vec2(2.0 * t.x, 0.0);
    let top = uv - vec2(0.0, 2.0 * t.y);
    let bottom = uv + vec2(0.0, 2.0 * t.y);

    // in UV, y grows downward
    let left_top = uv - t;
    let right_bottom = uv + t;
    let right_top = uv + vec2(t.x, -t.y);
    let left_bottom = uv - vec2(t.x, -t.y);

    let left_color = textureSample(source, source_sampler, left);
    let right_color = textureSample(source, source_sampler, right);
    let top_color = textureSample(source, source_sampler, top);
    let bottom_color = textureSample(source, source_sampler, bottom);

    let left_top_color = textureSample(source, source_sampler, left_top);
    let right_bottom_color = textureSample(source, source_sampler, right_bottom);
    let right_top_color = textureSample(source, source_sampler, right_top);
    let left_bottom_color = textureSample(source, source_sampler, left_bottom);

    // the diagonals sit closer to the pixel, so they weigh twice the cross; the weights add up
    // to 12, so the image keeps its brightness at every level
    return (left_color + right_color + top_color + bottom_color + 2.0 * (left_top_color + right_bottom_color + right_top_color + left_bottom_color)) / 12.0;
}
