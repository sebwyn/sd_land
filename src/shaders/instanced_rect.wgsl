struct InstanceInput {
    @location(5) i_position: vec2<f32>,
    @location(6) dimensions: vec2<f32>,
    @location(7) color: vec4<f32>,
    @location(8) tex_origin: vec2<f32>,
    @location(9) tex_dimensions: vec2<f32>,
    @location(10) border_radius: f32,
    @location(11) depth: f32,
};

struct VertexInput {
    @location(0) v_position: vec2<f32>,
    @location(1) tex_coord: vec2<f32>,
};

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) color: vec4<f32>,
    @location(1) tex_coord: vec2<f32>,
    @location(2) distance: vec2<f32>,
    @location(3) dimensions: vec2<f32>,
    @location(4) radius: f32,
};

@group(0) @binding(0) 
var<uniform> view_proj: mat4x4<f32>;

@vertex
fn vs_main(
    vertex: VertexInput, 
    instance: InstanceInput,
) -> VertexOutput {
    var out: VertexOutput;


    let position = instance.i_position + instance.dimensions * vertex.v_position;

    out.clip_position = view_proj * vec4<f32>(position, instance.depth, 1.0);
    out.color = instance.color;
    out.tex_coord = instance.tex_origin + instance.tex_dimensions * vertex.tex_coord;
    
    let half_dimensions = instance.dimensions / 2.0;
    out.distance = (vertex.v_position * instance.dimensions - half_dimensions);
    out.dimensions = half_dimensions;
    out.radius = instance.border_radius;

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

    let corner = abs(distance);

    var opacity = 1.0;
    let circle_center = in.dimensions - vec2<f32>(in.radius);
    if (corner.x > circle_center.x && corner.y > circle_center.y) {
        //calculate the distance to the radius point
        if distance(corner, circle_center) < in.radius {
            opacity = 1.0;
        } else {
            opacity = 0.0;
        }
    }
    
    return vec4<f32>(in.color.rgb, in.color.a * opacity);
}