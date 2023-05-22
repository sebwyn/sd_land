use simple_error::SimpleError;

use crate::{
    renderer::{render_api::{Subrenderer, MaterialHandle, RenderWork}, 
        camera::Camera, 
        shader_types::Matrix, primitive::{Rectangle, Vertex}}, 
    text::{Font, create_font_material}, layout::Transform};

use legion::IntoQuery;

pub struct TextBox {
    pub text: String,
    pub text_color: [f32; 3],
    pub line_height: f32,
    pub font_scale: f32,
}

pub struct TextRenderer {
    font: Font,
    material: Option<MaterialHandle>
}

impl TextRenderer {
    pub fn new(font_name: &str) -> Result<Self, SimpleError> {
        let font = Font::load(font_name)?;

        Ok(Self {
            font,
            material: None
        })
    }
}

impl Subrenderer for TextRenderer {
    fn init(&mut self, renderer: &mut crate::renderer::render_api::RenderApi) {
        self.material = Some(create_font_material(renderer, &self.font).unwrap());
    }

    fn render(&mut self, world: &legion::World, renderer: &mut crate::renderer::render_api::RenderApi) -> Result<(), wgpu::SurfaceError> {
        
        let mut vertices = Vec::new();
        
        for (transform, text_box) in <(&Transform, &TextBox)>::query().iter(world) {
            
            let mut current_y = transform.position.1 + transform.size.1 - text_box.line_height;
            let mut lines = text_box.text.lines();

            while current_y > transform.position.1 {
                //get the next line
                if let Some(current_line) = lines.next() {
                    //render the current line, one character at a time, until its good
                    let mut current_x = transform.position.0;
                    let mut chars = current_line.chars().peekable();
                    while let Some(c) = chars.next() {
                        let (bounds, rectangle) = self.font
                            .layout_character(c, chars.peek().cloned(), (current_x, current_y), text_box.font_scale, 0.5)
                            .unwrap();

                        current_x = bounds;
                        if current_x < (transform.position.0 + transform.size.0) {
                            vertices.extend(rectangle.color(text_box.text_color).build())
                        } else {
                            break
                        }
                    }
                } else {
                    break
                }

                current_y -= text_box.line_height;
            }
        }

        let num_rectangles = vertices.len() / 4;
        let indices = (0..num_rectangles)
            .flat_map(|offset| Rectangle::INDICES.map(|i| i + (4 * offset) as u32))
            .collect::<Vec<_>>();

        let (screen_width, screen_height) = renderer.screen_size();
        let screen_camera = Matrix::from(Camera::new(screen_width, screen_height).matrix());

        let material = self.material.unwrap();
        renderer.update_material(material, "view_proj", screen_camera).unwrap();

        let work = RenderWork::<Vertex, Rectangle> {
            vertices,
            indices,
            material,
            instances: None
        };

        renderer.submit_subrender(&[work], None).unwrap();

        Ok(())
    }
}