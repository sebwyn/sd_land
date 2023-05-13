use regex::Regex;
use simple_error::SimpleError;

use crate::renderer::{
    pipeline::Pipeline, 
    renderer::{Renderer, MaterialHandle}, 
    primitive::{RectangleBuilder, Vertex}
};

pub struct UiBoxFactory {
    material_handle: MaterialHandle
}

impl UiBoxFactory {
    pub fn new(renderer: &mut Renderer) -> Result<Self, SimpleError> {
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

pub fn hex_color(color: &str) -> Result<[f32; 3], SimpleError> {
    let regex = Regex::new(r"#([0-9a-f]{2})([0-9a-f]{2})([0-9a-f]{2})").expect("Failed to compile regex");

    let color = color.to_ascii_lowercase();

    let captures = regex.captures(&color).unwrap();

    let r = captures.get(1)
        .ok_or(SimpleError::new("Failed to parse hex color!"))?
        .as_str();

    let g = captures.get(2)
        .ok_or(SimpleError::new("Failed to parse hex color!"))?
        .as_str();

    let b = captures.get(3)
        .ok_or(SimpleError::new("Failed to parse hex color!"))?
        .as_str();

    let r = u32::from_str_radix(r, 16).map_err(|_| SimpleError::new("hex_color: Failed to convert string to number"))? as f32;
    let g = u32::from_str_radix(g, 16).map_err(|_| SimpleError::new("hex_color: Failed to convert string to number"))? as f32;
    let b = u32::from_str_radix(b, 16).map_err(|_| SimpleError::new("hex_color: Failed to convert string to number"))? as f32;

    Ok([ r / 255f32, g / 255f32, b / 255f32 ])
}