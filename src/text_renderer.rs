use simple_error::SimpleError;

use crate::{
    renderer::{render_api::{MaterialHandle, RenderWork},
        camera::Camera, 
        shader_types::Matrix, primitive::{Rectangle, Vertex}},
    text::Font, layout::Transform};

use legion::{IntoQuery, Query, system};
use legion::world::SubWorld;
use crate::renderer::pipeline::Pipeline;
use crate::renderer::render_api::RenderApi;
use crate::text::create_font_texture;

pub struct TextBox {
    pub text: String,
    pub text_color: [f32; 3],
    pub line_height: f32,
    pub font_scale: f32,
}

pub struct TextRenderer {
    font: Font,
    material: MaterialHandle
}

impl TextRenderer {
    pub fn new(font_path: &str, renderer: &mut RenderApi) -> Result<Self, SimpleError> {
        let font = Font::load_font(font_path)?;

        let (texture, sampler) = create_font_texture(renderer, &font)?;

        let text_pipeline = Pipeline::load(include_str!("shaders/text_shader.wgsl"))?
            .with_vertex::<Vertex>();

        let pipeline_handle = renderer.create_pipeline(text_pipeline);

        let material = renderer.create_material(pipeline_handle)?;
        renderer.update_material(material, "t_diffuse", texture).unwrap();
        renderer.update_material(material, "s_diffuse", sampler).unwrap();

        Ok(Self {
            font,
            material
        })
    }
}

#[system]
fn render_text(#[state] text_renderer: &TextRenderer, world: &SubWorld, query: &mut Query<(&Transform, &TextBox)>, #[resource] renderer: &mut RenderApi) {
    let mut vertices = Vec::new();

    for (transform, text_box) in query.iter(world) {
        let mut current_y = transform.position.1 + transform.size.1 - text_box.line_height;
        let mut lines = text_box.text.lines();

        while current_y > transform.position.1 {
            if let Some(current_line) = lines.next() {
                let mut current_x = transform.position.0;
                let mut chars = current_line.chars().peekable();
                while let Some(c) = chars.next() {
                    let (bounds, rectangle) = text_renderer.font
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

    let material = text_renderer.material;
    renderer.update_material(material, "view_proj", screen_camera).unwrap();

    let work = RenderWork::<Vertex, Rectangle> {
        vertices,
        indices,
        material,
        instances: None
    };

    renderer.submit_subrender(&[work], None).unwrap();
}