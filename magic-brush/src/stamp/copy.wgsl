@group(0) @binding(0) var source_sampler: sampler;
@group(0) @binding(1) var source_texture: texture_2d<f32>;
@group(0) @binding(2) var source_opacity: texture_depth_2d;

struct Vertex {
    @builtin(position) position: vec4f,
    @location(0) texture_coords: vec2f,
}

@vertex
fn vertexShader(@builtin(vertex_index) i: u32) -> Vertex {
    const vertices = array(
        Vertex(vec4f(-1.0,  1.0, 0.0, 1.0), vec2f(0.0, 0.0)),
        Vertex(vec4f( 1.0,  1.0, 0.0, 1.0), vec2f(1.0, 0.0)),
        Vertex(vec4f(-1.0, -1.0, 0.0, 1.0), vec2f(0.0, 1.0)),
        Vertex(vec4f( 1.0, -1.0, 0.0, 1.0), vec2f(1.0, 1.0))
    );
    return vertices[i];
}

@fragment
fn fragmentShader(v: Vertex) -> @location(0) vec4f {
    let color = textureSample(source_texture, source_sampler, v.texture_coords);
    let opacity = textureSample(source_opacity, source_sampler, v.texture_coords);
    return color * opacity;
}