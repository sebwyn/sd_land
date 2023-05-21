use std::{fs::File, io::Read, ops::Range, collections::HashMap, cmp::Ordering};

use colors_transform::{Rgb, Color, Hsl};
use image::{ImageBuffer};
use simple_error::SimpleError;

use crate::renderer::{primitive::{RectangleBuilder, Vertex}};

#[derive(Clone)]
pub struct Image {
    buffer: ImageBuffer<image::Rgba<u8>, Vec<u8>>,
    opacity: f32,

    position: (f32, f32),
    size: (f32, f32),

    depth: f32,
    tex_coords: [[f32; 2]; 4]
}

//builder
impl Image {
    pub fn position(mut self, position: (f32, f32)) -> Self {
        self.position = position; self
    }

    pub fn size(mut self, size: (f32, f32)) -> Self {
        self.size = size; self
    }

    pub fn depth(mut self, depth: f32) -> Self {
        self.depth = depth; self
    }

    pub fn opacity(mut self, opacity: f32) -> Self {
        self.opacity = opacity; self
    }
}


//think about using some graphics acceleration for some of this image mod
impl Image {
    //will display the whole image by default
    const DEFAULT_TEX_COORDS: [[f32; 2]; 4] = [[0.0, 1.0], [0.0, 0.0], [1.0, 1.0], [1.0, 0.0]];
    const DEFAULT_POSITION: (f32, f32) = (0f32, 0f32);
    const DEFAULT_SIZE: (f32, f32) = (1f32, 1f32);
    const DEFAULT_DEPTH: f32 = 0.5;

    pub fn load(file_path: &str) -> Result<Self, SimpleError> {
        let mut image_bytes = Vec::new();

        File::open(file_path)
            .map_err(|_| SimpleError::new("Failed to find file!"))?
            .read_to_end(&mut image_bytes)
            .map_err(|_| SimpleError::new("Failed to read bytes!"))?;

        let dynamic_image = image::load_from_memory(&image_bytes)
            .map_err(|_| SimpleError::new("Invalid image!"))?;

        let buffer = dynamic_image.to_rgba8();


        Ok(Self {
            buffer,
            opacity: 1.0,

            position: Self::DEFAULT_POSITION,
            size: Self::DEFAULT_SIZE,
            depth: Self::DEFAULT_DEPTH,

            tex_coords: Self::DEFAULT_TEX_COORDS
        })
    }

    pub fn auto_scale(&mut self, target_size: (f32, f32)) {
        let size = (self.buffer.dimensions().0 as f32, self.buffer.dimensions().1 as f32);

        let height_ratio = target_size.1 / size.1;
        let width_ratio = target_size.0 / size.0;
        
        self.tex_coords = 
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

    pub fn mask_colors_by_hue(&self, included_hue_range: Range<f32>) -> Self {
        let mut variant = self.clone();

        let (image_width, image_height) = variant.buffer.dimensions();

        let buf = variant.buffer.pixels().flat_map(|color| {
            let rgb = Rgb::from(color.0[0] as f32, color.0[1] as f32, color.0[2] as f32);
            let hsl = rgb.to_hsl();

            if included_hue_range.contains(&hsl.get_hue()) {
                [rgb.get_red() as u8, rgb.get_green() as u8, hsl.get_blue() as u8, 255u8]
            } else {
                [hsl.get_hue() as u8, hsl.get_hue() as u8, hsl.get_hue() as u8, 0u8]
            }
        }).collect::<Vec<u8>>();

        let image_buffer = 
            ImageBuffer::<image::Rgba<u8>, Vec<u8>>::from_raw(image_width, image_height, buf)
                .unwrap();

        variant.buffer =  image_buffer;

        variant
    }

    pub fn find_color_ranges(&self, color_ranges: &[(&str, Range<f32>)]) -> HashMap<String, Vec<Hsl>> {
        let pixel_vec = self.buffer.pixels().cloned().collect::<Vec<_>>();

        let saturation_range = 95f32..100f32;
        let lightness_range = 0f32..80f32;

        let mut colors_options: HashMap<String, Vec<Hsl>> = HashMap::new();

        for color in pixel_vec.iter() {
            let rgb = Rgb::from(color.0[0] as f32, color.0[1] as f32, color.0[2] as f32);
            let hsl = rgb.to_hsl();

            if saturation_range.contains(&hsl.get_saturation()) && 
               lightness_range.contains(&hsl.get_lightness()) {
                for (name, range) in color_ranges {
                    if range.contains(&hsl.get_hue()) {
                        let color_vec = colors_options.entry(name.to_string()).or_insert(Vec::new());
                        color_vec.push(hsl);
                        break;
                    }
                }
            }
        }

        for color_options in colors_options.values_mut() {
            color_options.sort_by(|a, b| 
                a.get_lightness().partial_cmp(&b.get_lightness())
                    .unwrap_or(Ordering::Equal)
            );

            color_options.dedup()
        }


        colors_options
    } 

    pub fn vertices(&self) -> Vec<Vertex> {
        RectangleBuilder::default()
            .position(self.position.0, self.position.1)
            .size(self.size.0, self.size.1)
            .depth(self.depth)
            .tex_coords(self.tex_coords)
            .opacity(self.opacity)
            .build()
    }

    pub fn image_data(&self) -> &ImageBuffer<image::Rgba<u8>, Vec<u8>> { &self.buffer }
}