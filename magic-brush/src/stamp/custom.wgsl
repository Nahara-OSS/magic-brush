@group(1) @binding(0) var tip_sampler: sampler;
@group(1) @binding(1) var tip_texture: texture_2d<f32>;

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
fn customFragment(v: Vertex) -> Fragment {
    let value = textureSample(tip_texture, tip_sampler, v.texture_coords).r;
    return Fragment(v.color * v.flow, v.opacity * value);
}