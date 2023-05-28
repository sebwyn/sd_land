use legion::{Query, system};
use legion::world::SubWorld;

use crate::{renderer::{
    pipeline::Pipeline, 
    render_api::{RenderApi, MaterialHandle, RenderWork},
    primitive::{Rectangle, RectangleVertex}, camera::Camera, shader_types::Matrix
}, layout::Transform};

pub struct UiBox {
    pub color: [f32; 3],
    pub opacity: f32,

    pub corner_radius: f32,
    pub border_color: [f32; 3],
    pub border_width: f32,

    pub image_path: Option<String>,
}

impl Default for UiBox {
    fn default() -> Self {
        Self {
            color: [0f32; 3], 
            opacity: 1f32, 
            corner_radius: 0f32,
            border_color: [0f32; 3], 
            border_width: 0f32,
            image_path: None
        }
    }
}

pub struct UiBoxRenderer {
    material: MaterialHandle
}

impl UiBoxRenderer {
    fn new(renderer: &mut RenderApi) -> Self {
        let pipeline = Pipeline::load(include_str!("shaders/instanced_rect.wgsl"))
            .unwrap()
            .with_vertex::<RectangleVertex>()
            .with_instance::<Rectangle>();

        let pipeline_handle = renderer.create_pipeline(pipeline);
        let material = renderer.create_material(pipeline_handle).unwrap();

        Self {
            material
        }
    }
}

#[system]
fn render_ui_box(
    #[state] ui_box_renderer: &UiBoxRenderer,
    world: &SubWorld,
    query: &mut Query<(&UiBox, &Transform)>,
    #[resource] renderer: &mut RenderApi
) {
    let rectangles = query.iter(world)
        .filter_map(|(ui_box, transform)| {
            if transform.visible {
                let rectangle = Rectangle::default()
                    .position([transform.position.0, transform.position.1])
                    .dimensions([transform.size.0, transform.size.1])
                    .color(ui_box.color)
                    .opacity(ui_box.opacity)
                    .depth(transform.depth)
                    .corner_radius(ui_box.corner_radius)
                    .border_width(ui_box.border_width)
                    .border_color(ui_box.border_color);

                Some(rectangle)
            } else {
                None
            }
        })
        .collect::<Vec<_>>();

    let (screen_width, screen_height) = renderer.screen_size();
    let screen_camera = Matrix::from(Camera::new(screen_width, screen_height).matrix());

    let material = ui_box_renderer.material;
    renderer.update_material(material, "view_proj", screen_camera).unwrap();

    let work = RenderWork::<RectangleVertex, Rectangle> {
        vertices: Rectangle::VERTICES.to_vec(),
        indices: Rectangle::INDICES.to_vec(),
        material: ui_box_renderer.material,
        instances: Some(rectangles),
    };

    renderer.submit_subrender(&[work], None).unwrap();
}