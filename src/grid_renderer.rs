use crate::renderer::camera::Camera;
use crate::renderer::pipeline::Pipeline;
use crate::renderer::primitive::{Rectangle, RectangleVertex};
use crate::renderer::render_api::{MaterialHandle, RenderApi, RenderWork};
use crate::renderer::shader_types::Matrix;
use crate::sprite::ActiveSceneCamera;
use legion::systems::Builder;
use legion::{component, system};

pub struct GridLines {
    material: MaterialHandle,

    color: [f32; 3],
    line_weight: f32,

    vertical_increment: f32,
    horizontal_increment: f32,
}

impl GridLines {
    pub fn new(
        vertical_increment: f32,
        horizontal_increment: f32,
        color: [f32; 3],
        line_weight: f32,
        render_api: &mut RenderApi,
    ) -> Self {
        let pipeline = Pipeline::load(include_str!("shaders/instanced_rect.wgsl"))
            .unwrap()
            .with_vertex::<RectangleVertex>()
            .with_instance::<Rectangle>();

        let pipeline_handle = render_api.create_pipeline(pipeline);

        let material = render_api.create_material(pipeline_handle).unwrap();

        Self {
            vertical_increment,
            horizontal_increment,
            color,
            line_weight,
            material,
        }
    }
}

pub fn add_grid_lines_subrender(grid_lines: GridLines, schedule: &mut Builder) {
    schedule.add_system(grid_lines_subrender_system(grid_lines));
}

#[system(for_each)]
#[read_component(Camera)]
#[filter(component::< ActiveSceneCamera > ())]
fn grid_lines_subrender(
    camera: &Camera,
    #[state] grid_lines: &GridLines,
    #[resource] render_api: &mut RenderApi,
    #[resource] screen_size: &(f32, f32),
) {
    let world_line_width = grid_lines.line_weight / screen_size.0 * camera.width;
    let world_line_height = grid_lines.line_weight / screen_size.1 * camera.height;

    let view_proj = Matrix::from(camera.matrix());
    render_api
        .update_material(grid_lines.material, "view_proj", view_proj)
        .unwrap();

    let start_x = camera.eye.x;
    let end_x = camera.eye.x + camera.width;

    let start_y = camera.eye.y;
    let end_y = camera.eye.y + camera.height;

    let mut current_x =
        (start_x / grid_lines.horizontal_increment).ceil() * grid_lines.horizontal_increment;
    let mut current_y =
        (start_y / grid_lines.horizontal_increment).ceil() * grid_lines.vertical_increment;

    let mut instances = Vec::new();

    while current_y < end_y {
        instances.push(
            Rectangle::default()
                .position([camera.eye.x, current_y])
                .dimensions([camera.width, world_line_height])
                .color(grid_lines.color)
                .opacity(1.0)
                .depth(0.9),
        );

        current_y += grid_lines.vertical_increment;
    }

    while current_x < end_x {
        instances.push(
            Rectangle::default()
                .position([current_x, camera.eye.y])
                .dimensions([world_line_width, camera.height])
                .color(grid_lines.color)
                .opacity(1.0)
                .depth(0.9),
        );

        current_x += grid_lines.horizontal_increment;
    }

    let work = RenderWork {
        vertices: Rectangle::VERTICES.to_vec(),
        indices: Rectangle::INDICES.to_vec(),
        instances: Some(instances),
        material: grid_lines.material,
    };

    render_api.submit_subrender(&[work], None).unwrap();
}
