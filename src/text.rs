use std::{io::Read, fs::File, collections::HashMap};

use fontdue::Metrics;
use image::Luma;
use simple_error::SimpleError;

use crate::{renderer::{MaterialHandle, Renderer}, pipeline::Pipeline, graphics::{Vertex, Rectangle, RectangleBuilder}, shader_types::{Texture, Sampler}};

use font_loader::system_fonts;

type TexCoords = [[f32; 2]; 4];

pub struct Font {
    characters: HashMap<char, (TexCoords, Metrics)>,
    texture: Texture,
}

impl Font {
    fn load_system_font(name: &str) -> Result<Vec<u8>, SimpleError> {
        // let fonts = system_fonts::query_all();
        let font_path = system_fonts::FontPropertyBuilder::new()
            .family(name)
            .build()
            .font_path()
            .ok_or("Cannot find system font path")?;

        let mut font_bytes = Vec::new();
        File::open(font_path)
            .map_err(|_| SimpleError::new("Cannot load font file!"))?
            .read_to_end(&mut font_bytes)
            .map_err(|_| SimpleError::new("Could not read font file as bytes!"))?;

        Ok(font_bytes)
    }

    pub fn load(renderer: &mut Renderer, system_font: &str) -> Result<Self, SimpleError> {
        let font_bytes = Self::load_system_font(system_font)?;

        let font = fontdue::Font::from_bytes(font_bytes, fontdue::FontSettings::default()).unwrap();

        let width = 123u32 - 97u32;

        let mut char_data = Vec::new();
        for c in 97u8..123 {
            let c = c as char;
            let (metrics, bitmap) = font.rasterize(c, 50f32);
            char_data.push((c, metrics, bitmap));
        }

        let max_height = char_data.iter()
            .max_by(|(_, a, _), (_, b, _)| a.height.cmp(&b.height))
            .map(|(_, m, _)| m.height)
            .unwrap() as u32;

        let max_width = char_data.iter()
            .max_by(|(_, a, _), (_, b, _)| a.width.cmp(&b.width))
            .map(|(_, m, _)| m.width)
            .unwrap() as u32;

        let font_image = image::GrayImage::from_fn((max_width * width) as u32, max_height, 
            |x, y| {
                let in_character_x: u32 = x % max_width;
                let character = (x - in_character_x) / max_width;
                let (_, metrics, bitmap) = char_data
                    .get(character as usize)
                    .expect("Failed to locate character");

                if y >= metrics.height as u32 || in_character_x >= metrics.width as u32 {
                    Luma([0u8])
                } else {
                    let in_character_index = (y * (metrics.width as u32) + in_character_x) as usize;
                    let byte: u8 = *bitmap
                        .get(in_character_index)
                        .unwrap_or(&0u8);

                    Luma([byte])
                }
            });

        font_image.save("font.jpg").unwrap();

        let texture = Texture::new(renderer.create_texture(font_image).unwrap());
        
        let mut characters = HashMap::new();
        for (i, (c, metrics, _)) in char_data.into_iter().enumerate() {
            let tex_coords = Self::tex_coords(
                i as u32, 0, 
                width, 1, 
                max_height as f32, max_width as f32, 
                &metrics
            );

            characters.insert(c, (tex_coords, metrics));
        }

        Ok(Self {
            characters,
            texture,
        })
    }

    fn tex_coords(tile_x: u32, tile_y: u32, width: u32, height: u32, tile_height: f32, tile_width: f32, metrics: &Metrics) -> TexCoords {
        //bottom of the char
        let tile_width_sf = 1f32 / (width as f32);
        let tile_height_sf = 1f32 / (height as f32);

        let top = (tile_y as f32) / (height as f32);
        let bottom = top + tile_height_sf * (metrics.bounds.height / tile_height);

        let left = (tile_x as f32) / (width as f32);
        let right = left + tile_width_sf * (metrics.bounds.width / tile_width);

        [[left, bottom], [left, top], [right, bottom], [right, top]]
    }

    fn layout_text(&self, text: &str, mut origin: (f32, f32), scale: f32) -> Result<Vec<Rectangle>, SimpleError> {
        let mut rectangles = Vec::new();
        for c in text.chars() {
            let (tex_coords, metrics) = self.characters.get(&c)
                .ok_or(SimpleError::new("That character hasn't been loaded in this font!"))?;
            
            //get the bottom left position 
            let bottom = origin.1 + (metrics.bounds.ymin * scale);
            let height = metrics.bounds.height * scale;
            let left = origin.0 + (metrics.bounds.xmin * scale);
            let width = metrics.bounds.width * scale;

            rectangles.push(RectangleBuilder::default()
                .position(left, bottom)
                .size(width, height)
                .tex_coords(*tex_coords)
                .build());

            origin.0 += metrics.advance_width * scale;
        }

        Ok(rectangles)
    }

}


pub struct TextBoxFactory {
    material_handle: MaterialHandle,
    font: Font,
}

impl TextBoxFactory {
    pub fn new(renderer: &mut Renderer) -> Result<Self, SimpleError> {
        let font = Font::load(renderer,"FiraCode Nerd Font")?;


        let text_pipeline = Pipeline::load::<Vertex>(include_str!("text_shader.wgsl"))?;
        let pipeline_handle = renderer.create_pipeline(text_pipeline);

        let material_handle = renderer.create_material(pipeline_handle)?;
        renderer.update_material(material_handle, "t_diffuse", font.texture.clone());

        let sampler = Sampler::new(renderer.create_sampler());
        renderer.update_material(material_handle, "s_diffuse", sampler);

        Ok(Self {
            material_handle,
            font
        })
    }

    pub fn create(&self) -> Vec<(Rectangle, MaterialHandle)> {    
        self.font.layout_text("helloworld", (0.0, 0.0), 0.001).unwrap()
            .into_iter()
            .map(|rect| (rect, self.material_handle))
            .collect::<Vec<_>>()
    }
}

