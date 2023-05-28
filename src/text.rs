use std::{io::Read, fs::File, cmp::Ordering};

use fontdue::Metrics;
use image::{Luma, ImageBuffer};
use simple_error::SimpleError;

use crate::renderer::{
    render_api::{MaterialHandle, RenderApi}, 
    pipeline::Pipeline, 
    primitive::{Vertex, RectangleBuilder}, 
    shader_types::{Texture, Sampler}
};

use font_loader::system_fonts;

type TexCoords = [[f32; 2]; 4];

#[derive(Debug)]
pub struct Bounds {
    pub left: f32,
    pub right: f32,
    pub top: f32,
    pub bottom: f32,
}

#[derive(Clone)]
pub struct Font {
    font_name: String,

    characters: Vec<Option<(TexCoords, Metrics)>>,
    font: fontdue::Font,
    
    smallest_y_min: f32,
    greatest_y: f32,

    font_image: ImageBuffer<Luma<u8>, Vec<u8>>,
}

impl Font {
    pub fn name(&self) -> &str { &self.font_name }

    pub fn smallest_y_min(&self, scale: f32) -> f32 {
        self.smallest_y_min * scale
    }

    pub fn font_height(&self, scale: f32) -> f32 {
        (self.greatest_y - self.smallest_y_min) * scale
    }

    pub fn load_font(font_path: &str) -> Result<Self, SimpleError> {
        let mut font_bytes = Vec::new();
        File::open(font_path)
            .map_err(|_| SimpleError::new("Cannot load font file!"))?
            .read_to_end(&mut font_bytes)
            .map_err(|_| SimpleError::new("Coulnt not read font file as bytes!"))?;
        
        Self::load(font_path, font_bytes)
    }

    pub fn load_system_font(name: &str) -> Result<Self, SimpleError> {
        let font_property = system_fonts::FontPropertyBuilder::new()
            .family(name)
            .build();
        
        let (font_bytes, _) = system_fonts::get(&font_property)
            .ok_or(SimpleError::new("Failed to load font data for system font!"))?;

        Self::load(name, font_bytes)
    }

    fn load(font_path: &str, font_bytes: Vec<u8>) -> Result<Self, SimpleError> {
        let font_settings = fontdue::FontSettings {
            collection_index: 3,
            scale: 70f32,
        };

        let font = fontdue::Font::from_bytes(font_bytes, font_settings).unwrap();

        let width = 127u32 - 32u32;

        let mut char_data = Vec::new();
        for c in 32u8..127 {
            let c = c as char;
            let (metrics, bitmap) = font.rasterize(c, 70f32);
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

        let smallest_y_min = char_data.iter()
            .min_by(|(_, a, _), (_, b, _)| 
                a.bounds.ymin.partial_cmp(&b.bounds.ymin)
                    .unwrap_or(Ordering::Equal) 
            )
            .map(|(_, m, _)| m.bounds.ymin)
            .unwrap();

        let greatest_y = char_data.iter()
            .max_by(|(_, a, _), (_, b, _)| {
                let a_y_max = a.bounds.ymin + a.bounds.height;
                let b_y_max = b.bounds.ymin + b.bounds.height;

                a_y_max.partial_cmp(&b_y_max)
                    .unwrap_or(Ordering::Equal) 
            })
            .map(|(_, m, _)| m.bounds.ymin + m.bounds.height)
            .unwrap();

        let font_image = image::GrayImage::from_fn(max_width * width, max_height, 
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

        let mut characters: Vec<Option<(TexCoords, Metrics)>> = vec![None; 128];

        for (i, (c, metrics, _)) in char_data.into_iter().enumerate() {
            let tex_coords = Self::tex_coords(
                i as u32, 0, 
                width, 1, 
                max_height as f32, max_width as f32, 
                &metrics
            );

            let c_ascii = TryInto::<u8>::try_into(c).unwrap() as usize;

            characters[c_ascii] = Some((tex_coords, metrics));
        }

        Ok(Self {
            font_name: font_path.to_string(),

            characters,
            font,
            smallest_y_min,
            greatest_y,
            font_image,
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

    pub fn get_char_pixel_width(&self, c: char, next_c: Option<char>, scale: f32) -> f32 {
        let c_ascii = TryInto::<u8>::try_into(c).unwrap() as usize;

        let (_, metrics) = self.characters.get(c_ascii)
            .ok_or(SimpleError::new("That character hasn't been loaded in this font!"))
            .unwrap()
            .ok_or(SimpleError::new("That character hasn't been loaded in this font!"))
            .unwrap();

        let mut character_width = scale * metrics.advance_width;
        if let Some(next_c) = next_c {
            character_width += scale * self.font.horizontal_kern(c, next_c, 1f32).unwrap_or(0f32)
        }
        character_width
    }

    pub fn get_str_pixel_width(&self, text: &str, scale: f32) -> f32 {
        let mut width = 0f32;

        let mut chars = text.chars().peekable();

        while let Some(char) = chars.next() {
            width += self.get_char_pixel_width(char, chars.peek().cloned(), scale)            
        }

        width
    }

    pub fn layout_character(&self, c: char, next_char: Option<char>, mut origin: (f32, f32), scale: f32, depth: f32) -> Result<(f32, RectangleBuilder), SimpleError> {
        origin.1 += -self.smallest_y_min * scale;

        let c_ascii = TryInto::<u8>::try_into(c).unwrap() as usize;

        let (tex_coords, metrics) = self.characters.get(c_ascii)
                .ok_or(SimpleError::new("That character hasn't been loaded in this font!"))?
                .ok_or(SimpleError::new("That character hasn't been loaded in this font!"))?;


        if metrics.bounds.ymin < self.smallest_y_min {
            panic!("Uh oh!");
        }

        //get the bottom left position 
        let bottom = origin.1 + (metrics.bounds.ymin * scale);
        let height = metrics.bounds.height * scale;
        let left = origin.0 + (metrics.bounds.xmin * scale);
        let width = metrics.bounds.width * scale;

        let rectangle = RectangleBuilder::default()
            .position(left, bottom)
            .size(width, height)
            .tex_coords(tex_coords)
            .depth(depth);

        if let Some(next_character) = next_char {
            if let Some(kerning) = self.font.horizontal_kern(c, next_character, 1f32) {
                origin.0 += (metrics.advance_width + kerning) * scale;
            }
        }
        origin.0 += metrics.advance_width * scale;

        Ok((origin.0, rectangle))
    }

    pub fn layout_text(&self, text: &str, mut origin: (f32, f32), scale: f32, depth: f32) -> Result<(Bounds, Vec<RectangleBuilder>), SimpleError> {
        origin.1 += -self.smallest_y_min * scale;
        
        let left = origin.0;

        let mut rectangles = Vec::new();
        let characters = text.chars().collect::<Vec<_>>();
        for (i, c) in characters.iter().enumerate() {
            let characters = text.chars().collect::<Vec<_>>();

            let c_ascii = TryInto::<u8>::try_into(*c).unwrap() as usize;

            let (tex_coords, metrics) = self.characters.get(c_ascii)
                .ok_or(SimpleError::new("That character hasn't been loaded in this font!"))?
                .ok_or(SimpleError::new("That character hasn't been loaded in this font!"))?;

            if metrics.bounds.ymin < self.smallest_y_min {
                panic!("Uh oh!");
            }

            //get the bottom left position 
            let bottom = origin.1 + (metrics.bounds.ymin * scale);
            let height = metrics.bounds.height * scale;
            let left = origin.0 + (metrics.bounds.xmin * scale);
            let width = metrics.bounds.width * scale;

            rectangles.push(RectangleBuilder::default()
                .position(left, bottom)
                .size(width, height)
                .tex_coords(tex_coords)
                .depth(depth)
            );

            if let Some(next_character) = characters.get(i + 1) {
                if let Some(kerning) = self.font.horizontal_kern(*c, *next_character, 1f32) {
                    origin.0 += (metrics.advance_width + kerning) * scale;
                    continue
                }
            }
            origin.0 += metrics.advance_width * scale;
        }

        let right = origin.0;

        Ok((Bounds { left, right, top: scale * self.greatest_y, bottom: scale * self.smallest_y_min}, rectangles))
    }

}

pub fn create_font_texture(renderer: &mut RenderApi, font: &Font) -> Result<(Texture, Sampler), SimpleError> {
    let texture = Texture::new(renderer.create_texture(&font.font_image)?);
    let sampler = Sampler::new(renderer.create_sampler());

    Ok((texture, sampler))
}


pub fn create_font_material(renderer: &mut RenderApi, font: &Font) -> Result<MaterialHandle, SimpleError> {
    let texture = Texture::new(renderer.create_texture(&font.font_image).unwrap());

    let text_pipeline = Pipeline::load(include_str!("shaders/text_shader.wgsl"))?.with_vertex::<Vertex>();
    let pipeline_handle = renderer.create_pipeline(text_pipeline);

    let material_handle = renderer.create_material(pipeline_handle)?;
    renderer.update_material(material_handle, "t_diffuse", texture).unwrap();

    let sampler = Sampler::new(renderer.create_sampler());
    renderer.update_material(material_handle, "s_diffuse", sampler).unwrap();

    Ok(material_handle)
}

