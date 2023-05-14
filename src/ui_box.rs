use simple_error::SimpleError;

use crate::{renderer::{
    pipeline::Pipeline, 
    render_api::{RenderApi, MaterialHandle}, 
    primitive::{RectangleBuilder, Vertex}
}, colorscheme::hex_color};

pub struct UiBoxFactory {
    material_handle: MaterialHandle
}

impl UiBoxFactory {
    pub fn new(renderer: &mut RenderApi) -> Result<Self, SimpleError> {
        let pipeline = Pipeline::load::<Vertex>(include_str!("shaders/rect.wgsl"))?;
        // renderer
        let pipeline_handle = renderer.create_pipeline(pipeline);
        let material_handle = renderer.create_material(pipeline_handle)?;

        Ok(Self {
            material_handle
        })
    }

    pub fn material(&self) -> MaterialHandle { self.material_handle }

    pub fn create(&self, color: &str, position: (f32, f32), size: (f32, f32), depth: f32) 
        -> Result<Vec<Vertex>, SimpleError> 
    {
        //convert a hex color here
        let color = hex_color(color)?;
        
        let rectangle = RectangleBuilder::default()
            .position(position.0, position.1)
            .size(size.0, size.1)
            .color(color)
            .depth(depth)
            .build();

        Ok(rectangle)
    }
}