use std::{ops::Range};
use simple_error::SimpleError;

use crate::image::Image;

pub struct Theme {
    pub background_image: Image,

}

impl Theme {
    const COLOR_LABELS: [(&'static str, Range<f32>); 7] = [
        ("red",      0f32.. 15f32),
        ("orange",  15f32.. 45f32),
        ("yellow",  45f32.. 72f32),
        ("green",   72f32..172f32),
        ("blue",   172f32..255f32),
        ("purple", 255f32..294f32),
        ("pink",   294f32..360f32),
    ];


    pub fn new(background_image_path: &str) -> Result<Self, SimpleError> {
        let background_image = Image::load(background_image_path).unwrap()
            .position((-1f32, -1f32))
            .size((2f32, 2f32))
            .opacity(0.2);

        Ok(Self {
            background_image
        })
    }

    pub fn generate_color_palette(&self) {
        let labeled_colors = self.background_image.find_color_ranges(&Self::COLOR_LABELS);

        for (label, colors) in labeled_colors.iter() {
            println!("Found {} variants in the background image for color: {}", colors.len(), label.to_uppercase());
        }
    }
}