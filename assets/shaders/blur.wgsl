// One direction of a Gaussian blur over the whole screen camera's image; `src/blur.rs` runs
// it twice, across then down, which together blur in both directions for a fraction of the
// samples.
//
// Texels are read with `textureLoad` rather than sampled: the pass's sampler is the default
// nearest one, and loading whole texels needs no filtering at all.

#import bevy_core_pipeline::fullscreen_vertex_shader::FullscreenVertexOutput

struct Blur {
    // (1, 0) across or (0, 1) down
    direction: vec2<f32>,
    // the Gaussian's standard deviation, in pixels
    sigma_px: f32,
}

@group(0) @binding(0) var screen_texture: texture_2d<f32>;
@group(0) @binding(1) var screen_sampler: sampler;
@group(0) @binding(2) var<uniform> blur: Blur;

@fragment
fn fragment(in: FullscreenVertexOutput) -> @location(0) vec4<f32> {
    let size = vec2<i32>(textureDimensions(screen_texture));
    let center = vec2<i32>(in.position.xy);
    let step = vec2<i32>(blur.direction);
    // a strength ramping up from zero would divide by zero below
    let sigma = max(blur.sigma_px, 0.01);
    // three standard deviations hold all but a sliver of the curve's weight
    let support = i32(ceil(sigma * 3.0));
    let exp_factor = -1.0 / (2.0 * sigma * sigma);

    var sum = vec3<f32>(0.0);
    var weight_sum = 0.0;
    for (var i = -support; i <= support; i++) {
        // clamped, so the edges blur with themselves instead of with black
        let coord = clamp(center + step * i, vec2<i32>(0), size - vec2<i32>(1));
        let weight = exp(exp_factor * f32(i * i));
        sum += textureLoad(screen_texture, coord, 0).rgb * weight;
        weight_sum += weight;
    }
    return vec4<f32>(sum / weight_sum, 1.0);
}
