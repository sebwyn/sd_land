use legion::IntoQuery;

use crate::{renderer::{
    pipeline::Pipeline, 
    render_api::{RenderApi, MaterialHandle, Subrenderer, RenderWork}, 
    primitive::{Rectangle, RectangleVertex}, camera::Camera, shader_types::Matrix
}, layout::Transform};

#[derive(Default)]
pub struct UiBox {
    pub color: [f32; 3],
    pub opacity: f32,

    pub border_radius: Option<f32>,
    pub border_color: Option<[f32; 3]>,
    pub border_width: f32,
}



#[derive(Default)]
pub struct UiBoxRenderer {
    material: Option<MaterialHandle>
}


impl UiBoxRenderer {
    pub fn new() -> Self {
        Self::default()
    }
}

impl Subrenderer for UiBoxRenderer {
    fn init(&mut self, renderer: &mut RenderApi) {
        // renderer
        let pipeline = Pipeline::load(include_str!("shaders/instanced_rect.wgsl"))
            .unwrap()
            .with_vertex::<RectangleVertex>()
            .with_instance::<Rectangle>();


        let pipeline_handle = renderer.create_pipeline(pipeline);
        self.material = Some(renderer.create_material(pipeline_handle).unwrap());
    }

    fn render(&mut self, world: &legion::World, renderer: &mut RenderApi) -> Result<(), wgpu::SurfaceError> {
        let rectangles = 
            <(&UiBox, &Transform)>::query().iter(world)
                .filter_map(|(ui_box, transform)| {
                    if transform.visible {
                        let rectangle = Rectangle::default()
                            .position([transform.position.0, transform.position.1])
                            .dimensions([transform.size.0, transform.size.1])
                            .color(ui_box.color)
                            .opacity(ui_box.opacity)
                            .depth(transform.depth)
                            .border_radius(ui_box.border_radius.unwrap_or(0f32));

                        Some(rectangle)
                    } else {
                        None
                    }
                })
                .collect::<Vec<_>>();
        
        let (screen_width, screen_height) = renderer.screen_size();
        let screen_camera = Matrix::from(Camera::new(screen_width, screen_height).matrix());

        let material = self.material.unwrap();
        renderer.update_material(material, "view_proj", screen_camera).unwrap();

        let work = RenderWork::<RectangleVertex, Rectangle> { 
            vertices: Rectangle::VERTICES.to_vec(), 
            indices: Rectangle::INDICES.to_vec(), 
            material: self.material.unwrap(),
            instances: Some(rectangles),
        };

        renderer.submit_subrender(&[work], None)?;

        Ok(())
    }
}