use legion::{Entity, IntoQuery};

use crate::{renderer::{
    pipeline::Pipeline, 
    render_api::{RenderApi, MaterialHandle, Subrenderer, RenderWork}, 
    primitive::{Vertex, RectangleBuilder, Rectangle}, camera::Camera, shader_types::Matrix
}, layout::Transform};

pub struct UiBox {
    pub color: [f32; 3],
    pub opacity: f32,
    pub view: Option<Entity>
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
        let pipeline = Pipeline::load::<Vertex>(include_str!("shaders/rect.wgsl")).unwrap();
        let pipeline_handle = renderer.create_pipeline(pipeline);
        self.material = Some(renderer.create_material(pipeline_handle).unwrap());
    }

    fn render(&mut self, world: &legion::World, renderer: &mut RenderApi) -> Result<(), wgpu::SurfaceError> {
        let vertices = 
            <(&UiBox, &Transform)>::query().iter(world)
                .filter_map(|(ui_box, transform)| {
                    if transform.visible {
                        Some(RectangleBuilder::default()
                                                .color(ui_box.color)
                                                .opacity(ui_box.opacity)
                                                .position(transform.position.0, transform.position.1)
                                                .size(transform.size.0, transform.size.1)
                                                .depth(transform.depth)
                                                .build())
                    } else {
                        None
                    }
                })
                .flatten()
                .collect::<Vec<_>>();
        
        let num_rectangles = vertices.len() / 4;
        let indices = (0..num_rectangles)
            .flat_map(|offset| Rectangle::INDICES.map(|i| i + (Rectangle::INDICES.len() * offset) as u32))
            .collect::<Vec<_>>();
        
        let (screen_width, screen_height) = renderer.screen_size();
        let screen_camera = Matrix::from(Camera::new(screen_width, screen_height).matrix());

        let material = self.material.unwrap();
        renderer.update_material(material, "view_proj", screen_camera).unwrap();

        let work = RenderWork { 
            vertices, 
            indices, 
            material: self.material.unwrap()
        };

        renderer.submit_subrender(&[work], None)?;

        Ok(())
    }
}