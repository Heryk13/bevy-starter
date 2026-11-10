// One halving of the glass's descent, Marius Bjørge's dual Kawase filter (SIGGRAPH 2015):
// `src/ui/blur.rs` runs it once per level, each writing a texture half the size of the one it
// reads, so the blur widens with every level for a fraction of a Gaussian's samples.
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
    // in UV, y grows downward
    let left_top = uv - t;
    let right_bottom = uv + t;
    let right_top = uv + vec2(t.x, -t.y);
    let left_bottom = uv - vec2(t.x, -t.y);

    let left_top_color = textureSample(source, source_sampler, left_top);
    let right_bottom_color = textureSample(source, source_sampler, right_bottom);
    let right_top_color = textureSample(source, source_sampler, right_top);
    let left_bottom_color = textureSample(source, source_sampler, left_bottom);

    let center_color = textureSample(source, source_sampler, uv);
    // the weights add up to 8, so the image keeps its brightness at every level
    return (4.0 * center_color + left_top_color + right_bottom_color + right_top_color + left_bottom_color) / 8.0;
}
