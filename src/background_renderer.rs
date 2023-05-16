use std::{fs::File, io::Read};

use image::{ImageBuffer, Rgba};
use simple_error::SimpleError;

use crate::renderer::{
    render_api::{Subrenderer, MaterialHandle, RenderWork}, 
    pipeline::Pipeline, 
    primitive::{Vertex, RectangleBuilder, Rectangle}, 
    shader_types::{Texture, Sampler}
};

pub struct BackgroundRenderer {
    image_rgba: ImageBuffer<Rgba<u8>, Vec<u8>>,

    material: Option<MaterialHandle>,
}

impl BackgroundRenderer {
    pub fn new(image_path: &str) -> Result<Self, SimpleError> {
        //load
        let mut image_bytes = Vec::new();
        
        File::open(image_path)
            .map_err(|_| SimpleError::new("Failed to find file!"))?
            .read_to_end(&mut image_bytes)
            .map_err(|_| SimpleError::new("Failed to read bytes!"))?;

        let image = image::load_from_memory(&image_bytes)
            .map_err(|_| SimpleError::new("Invalid image!"))?;

        let image_rgba = image.to_rgba8();

        Ok(Self { image_rgba, material: None })
    }
}

impl Subrenderer for BackgroundRenderer {
    fn init(&mut self, renderer: &mut crate::renderer::render_api::RenderApi) {
        let texture = Texture::new(renderer.create_texture(&self.image_rgba).unwrap());
        let sampler = Sampler::new(renderer.create_sampler());

        let raw_pipeline = Pipeline::load::<Vertex>(include_str!("shaders/background.wgsl")).unwrap();

        let pipeline = renderer.create_pipeline(raw_pipeline);
        let material = renderer.create_material(pipeline).unwrap();

        renderer.update_material(material, "t_diffuse", texture);
        renderer.update_material(material, "s_diffuse", sampler);
    
        self.material = Some(material);
    }

    fn render(&mut self, _: &legion::World, renderer: &mut crate::renderer::render_api::RenderApi) -> Result<(), wgpu::SurfaceError> {
        let vertices = RectangleBuilder::default()
            .position(-1.0, -1.0)
            .size(2.0, 2.0)
            .depth(0.1)
            .opacity(0.1)
            .build();

        let render_work = RenderWork {
            vertices,
            indices: Rectangle::INDICES.to_vec(),
            material: self.material.unwrap(),
        };
        
        renderer.submit_subrender(&[render_work], None)
    }
}
