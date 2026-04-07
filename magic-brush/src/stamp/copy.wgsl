@group(0) @binding(0) var<uniform> world_to_clip: mat4x4f;
@group(1) @binding(0) var source_sampler: sampler;
@group(1) @binding(1) var source_texture: texture_2d<f32>;
@group(1) @binding(2) var source_opacity: texture_depth_2d;

struct Vertex {
    @builtin(position) position: vec4f,
    @location(0) texture_coords: vec2f,
}

@vertex
fn vertexShader(@builtin(vertex_index) i: u32) -> Vertex {
    const positions = array(vec2f(-1.0, 1.0), vec2f(1.0, 1.0), vec2f(-1.0, -1.0), vec2f(1.0, -1.0));
    const texture_coords = array(vec2f(0.0, 0.0), vec2f(1.0, 0.0), vec2f(0.0, 1.0), vec2f(1.0, 1.0));
    return Vertex(world_to_clip * vec4f(positions[i], 0.0, 1.0), texture_coords[i]);
}

@fragment
fn fragmentShader(v: Vertex) -> @location(0) vec4f {
    let color = textureSample(source_texture, source_sampler, v.texture_coords);
    let opacity = textureSample(source_opacity, source_sampler, v.texture_coords);
    return color * opacity;
}