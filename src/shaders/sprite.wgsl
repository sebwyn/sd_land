struct InstanceInput {
    @location(5) i_position: vec2<f32>,
    @location(6) dimensions: vec2<f32>,
    @location(7) color: vec4<f32>,
    @location(8) tex_origin: vec2<f32>,
    @location(9) tex_dimensions: vec2<f32>,
    @location(10) corner_radius: f32,
    @location(11) border_width: f32,
    @location(12) border_color: vec3<f32>,
    @location(13) depth: f32,
};

struct VertexInput {
    @location(0) v_position: vec2<f32>,
    @location(1) tex_coord: vec2<f32>,
};

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) distance: vec2<f32>,
    @location(1) color: vec4<f32>,
    @location(2) dimensions: vec2<f32>,
    @location(3) radius: f32,
    @location(4) border_width: f32,
    @location(5) border_color: vec3<f32>,
    @location(6) tex_coord: vec2<f32>,
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

    let half_dimensions = instance.dimensions / 2.0;
    out.dimensions = half_dimensions;

    out.distance = (vertex.v_position * instance.dimensions - half_dimensions);

    out.radius = instance.corner_radius;
    out.border_width = instance.border_width;
    out.border_color = instance.border_color;
    out.tex_coord = instance.tex_origin + instance.tex_dimensions * vertex.tex_coord;

    return out;
}

@group(1) @binding(0)
var t_diffuse: texture_2d<f32>;
@group(1) @binding(1)
var s_diffuse: sampler;

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {

    let corner = abs(in.distance);

    var color = in.color.rgb;

    var opacity = 1.0;
    let circle_center = in.dimensions - vec2<f32>(in.radius);
    if (corner.x > circle_center.x && corner.y > circle_center.y) {

        //whacky branch less logic for seeing if a point is on the border or outside of the corner radius
        let distance_from_corner_point = distance(corner, circle_center);
        let is_in_radius = 1.0 - floor(distance_from_corner_point / in.radius);
        let is_border = is_in_radius * floor(distance_from_corner_point / (in.radius - in.border_width));
        color = (1.0 - is_border) * color + is_border * in.border_color;
        opacity = is_in_radius;

    } else if (
        corner.x > in.dimensions.x - in.border_width ||
        corner.y > in.dimensions.y - in.border_width
    ) {
        color = in.border_color;
    }

    let sampled_color = textureSample(t_diffuse, s_diffuse, in.tex_coord);

    // return vec4<f32>(1.0);

    return sampled_color * vec4<f32>(color, in.color.a * opacity);
}