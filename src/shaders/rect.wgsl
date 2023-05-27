// Vertex shader
struct VertexInput {
    @location(0) position: vec3<f32>,
    @location(1) color: vec4<f32>,
    @location(2) tex_coords: vec2<f32>,
};

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) color: vec4<f32>,
};

@group(0) @binding(0) 
var<uniform> view_proj: mat4x4<f32>;

@vertex
fn vs_main(in: VertexInput, @builtin(vertex_index) in_vertex_index: u32) -> VertexOutput {
    var out: VertexOutput;


    out.clip_position = view_proj * vec4<f32>(in.position, 1.0);
    out.color = in.color;

    return out;
}

// Fragment shader
var<private> distance_colors: array<vec4<f32>, 4> = array<vec4<f32>, 4>(
    vec4<f32>(1.0, 0.0, 0.0, 0.0), vec4<f32>(0.0, 0.0, 1.0, 0.0),
    vec4<f32>(0.0, 0.0, 0.0, 1.0), vec4<f32>(0.0, 1.0, 0.0, 0.0),
);

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {    
    return in.color;
}