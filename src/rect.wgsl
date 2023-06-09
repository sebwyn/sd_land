// Vertex shader
struct VertexInput {
    @location(0) position: vec3<f32>,
    @location(1) color: vec3<f32>,
    @location(2) tex_coords: vec2<f32>,
};

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) color: vec4<f32>, 
    @interpolate(linear) @location(1) distance: vec4<f32>,
};

// @group(0) @binding(0) var<uniform> ortho_matrix: mat4x4<f32>;
// @group(1) @binding(0) var<uniform> radius: f32;

@vertex
fn vs_main(in: VertexInput, @builtin(vertex_index) in_vertex_index: u32) -> VertexOutput {
    var out: VertexOutput;

    let b1 = in_vertex_index & u32(1);
    let n_b1 = !in_vertex_index & u32(1);
    let b2 = (in_vertex_index & u32(2)) >> u32(1);
    let n_b2 = (!in_vertex_index & u32(2)) >> u32(1);

    let r = f32(b1 & n_b2 | n_b1 & n_b2);
    let b = f32(b1 & b2 | b2 & n_b1);

    let a = f32(b1 & n_b2 | b1 & b2);
    let g = f32(b2 & n_b1 | n_b1 & n_b2);

    out.distance = vec4<f32>(r, g, b, a);

    out.clip_position = vec4<f32>(in.position, 1.0);
    out.color = vec4<f32>(in.color, 1.0);

    return out;
}

// Fragment shader
var<private> distance_colors: array<vec4<f32>, 4> = array<vec4<f32>, 4>(
    vec4<f32>(1.0, 0.0, 0.0, 0.0), vec4<f32>(0.0, 0.0, 1.0, 0.0),
    vec4<f32>(0.0, 0.0, 0.0, 1.0), vec4<f32>(0.0, 1.0, 0.0, 0.0),
);

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    var distance = in.distance;
    
    /*var lowest_dist = 100.0;
    var closest_color = vec4<f32>(0.0);
    for(var i = 0; i < 4; i=i+1) {
        let corner = distance_colors[i];
        
        let dist_2 = 
            pow((corner.r - distance.r), 2.0) +
            pow((corner.g - distance.g), 2.0) +
            pow((corner.b - distance.b), 2.0) +
            pow((corner.a - distance.a), 2.0);

        let dist = sqrt(dist_2);
        if dist < lowest_dist {
            lowest_dist = dist;
            closest_color = corner;
        }
    }*/

    let color = vec3<f32>(
        step(0.2,
        smoothstep(1.0, 0.95, distance.b) * 
        smoothstep(1.0, 0.95, distance.r) *
        smoothstep(1.0, 0.8, distance.g) *
        smoothstep(1.0, 0.8, distance.a))
    );
    
    return in.color * vec4<f32>(color, 1.0);
    // return distance;
}