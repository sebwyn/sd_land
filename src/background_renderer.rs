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
    image_size: (u32, u32),

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

        let image_size = (image_rgba.width(), image_rgba.height());

        Ok(Self { image_rgba, image_size, material: None })
    }

    fn auto_scale(size: (f32, f32), target_size: (f32, f32)) -> [[f32; 2]; 4] {
        let height_ratio = target_size.1 / size.1;
        let width_ratio = target_size.0 / size.0;
        
        if height_ratio > width_ratio {
            let new_width = size.0 * height_ratio;
            let width_difference = (new_width - target_size.0) / new_width / 2.0;

            [[width_difference, 1.0], [width_difference, 0.0], [1.0 - width_difference, 1.0], [1.0 - width_difference, 0.0]]
        } else {
            let new_height = size.1 * width_ratio;
            let height_difference = (new_height - target_size.1) / new_height / 2.0;
            [[0.0, 1.0 - height_difference], [0.0, height_difference], [1.0, 1.0 - height_difference], [1.0, height_difference]]
        }
    }
}

impl Subrenderer for BackgroundRenderer {
    fn init(&mut self, renderer: &mut crate::renderer::render_api::RenderApi) {
        let texture = Texture::new(renderer.create_texture(&self.image_rgba).unwrap());
        let sampler = Sampler::new(renderer.create_sampler());

        let raw_pipeline = Pipeline::load(include_str!("shaders/background.wgsl")).unwrap().with_vertex::<Vertex>();

        let pipeline = renderer.create_pipeline(raw_pipeline);
        let material = renderer.create_material(pipeline).unwrap();

        renderer.update_material(material, "t_diffuse", texture).unwrap();
        renderer.update_material(material, "s_diffuse", sampler).unwrap();
    
        self.material = Some(material);
    }

    fn render(&mut self, _: &legion::World, renderer: &mut crate::renderer::render_api::RenderApi) -> Result<(), wgpu::SurfaceError> {
        let screen_size = (renderer.screen_size().0 as f32, renderer.screen_size().1 as f32);
        let image_size = (self.image_size.0 as f32, self.image_size.1 as f32);

        let tex_coords = Self::auto_scale(image_size, screen_size);
        
        let vertices = RectangleBuilder::default()
            .position(-1.0, -1.0)
            .size(2.0, 2.0)
            .depth(0.1)
            .opacity(0.1)
            .tex_coords(tex_coords)
            .build();

        let render_work = RenderWork::<Vertex, Rectangle> {
            vertices,
            indices: Rectangle::INDICES.to_vec(),
            material: self.material.unwrap(),
            instances: None
        };
        
        renderer.submit_subrender(&[render_work], None)
    }
}
