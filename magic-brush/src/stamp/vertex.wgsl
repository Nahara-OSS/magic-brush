@group(0) @binding(0) var<uniform> world_to_clip: mat4x4f;

struct Stamp {
    @location(0) color: vec4f,
    @location(1) world_coords: vec2f,
    @location(2) size: f32,
    @location(3) flow: f32,
    @location(4) opacity: f32,
}

struct Vertex {
    @builtin(position) position: vec4f,
    @location(0) texture_coords: vec2f,
    @location(1) color: vec4f,
    @location(2) flow: f32,
    @location(3) opacity: f32,
}

@vertex
fn vertexShader(
    @builtin(vertex_index) i: u32,
    stamp: Stamp
) -> Vertex {
    const positions = array(vec2f(-0.5,  0.5), vec2f(0.5,  0.5), vec2f(-0.5, -0.5), vec2f(0.5, -0.5));
    const texture_coords = array(vec2f(0.0, 0.0), vec2f(1.0, 0.0), vec2f(0.0, 1.0), vec2f(1.0, 1.0));

    return Vertex(
        world_to_clip * vec4f(stamp.world_coords + positions[i] * stamp.size, 0.0, 1.0),
        texture_coords[i],
        stamp.color,
        stamp.flow,
        stamp.opacity
    );
}