@group(1) @binding(0) var stamp_sampler: sampler;
@group(1) @binding(1) var stamp_depthmap: texture_1d<f32>;

struct Vertex {
    @builtin(position) position: vec4f,
    @location(0) texture_coords: vec2f,
    @location(1) color: vec4f,
    @location(2) flow: f32,
    @location(3) opacity: f32,
}

struct Fragment {
    @location(0) color: vec4f,
    @builtin(frag_depth) opacity: f32,
}

@fragment
fn circleFragment(v: Vertex) -> Fragment {
    let dist = distance(v.texture_coords * 2.0, vec2f(1.0));
    let value = textureSample(stamp_depthmap, stamp_sampler, dist).r * step(dist, 1.0);
    return Fragment(v.color * v.flow, v.opacity * value);
}

@fragment
fn squareFragment(v: Vertex) -> Fragment {
    let dist = max(abs(v.texture_coords.x * 2.0 - 1.0), abs(v.texture_coords.y * 2.0 - 1.0));
    let value = textureSample(stamp_depthmap, stamp_sampler, dist).r;
    return Fragment(v.color * v.flow, v.opacity * value);
}