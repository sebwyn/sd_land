use std::{collections::HashMap};

use legion::{World, IntoQuery};
use winit::dpi::PhysicalPosition;

use crate::{renderer::{render_api::{Subrenderer, RenderApi, MaterialHandle, RenderWork}, view::View, camera::Camera, primitive::{RectangleBuilder, Vertex, Rectangle}, pipeline::Pipeline, shader_types::Matrix}, text::{Font, create_font_material}, buffer::{Buffer, Highlight, BufferRange}, colorscheme::{hex_color, ColorScheme, RUST_HIGHLIGHT_NAMES, get_highlight_for_code_type}, buffer_system::Cursor};

pub struct BufferView {
    view: View,
    camera: Camera,

    line_height: f32,
    font_scale: f32,
    font: Font,

    colorscheme: ColorScheme,
}

impl BufferView {
    const DEFAULT_FONT: &str = "Arial";

    pub fn contains(&self, point: &PhysicalPosition<f64>) -> bool {
        self.view.contains_point(point)
    }

    pub fn scroll_vertically(&mut self, scroll_y: f32) {
        self.camera.eye.y += scroll_y;
        self.camera.target.y = self.camera.eye.y;
    }

   //uses solid defaults
   pub fn new(left: u32, right: u32, top: u32, bottom: u32) -> Self {
        //create the camera
        let camera = Camera::new(right - left, bottom - top);
        let view = View::new(left, right, bottom, top, -100.0, 100.0);

        Self {
            view,
            camera,

            line_height: 50f32,
            font_scale: 0.5,
            font: Font::load(Self::DEFAULT_FONT).unwrap(),

            colorscheme: ColorScheme::default(),
        }
    }

    pub fn font(mut self, font_name: &str) -> Self {
        self.font = Font::load(font_name).unwrap(); self
    }

    pub fn font_scale(mut self, font_scale: f32) -> Self {
        self.font_scale = font_scale; self
    }

    pub fn line_height(mut self, line_height: f32) -> Self {
        self.line_height = line_height; self
    }

    pub fn buffer_position(&self, buffer: &Buffer, screen_position: &PhysicalPosition<f64>) -> Option<(usize, usize)> {
        if let Some(view_position) = self.view.to_view(screen_position) {
            let world_position = self.camera.view_to_world(view_position);
            
            let lines = buffer.lines();

            //calculate what line we're on
            let mut row = if world_position.1 < 0.0 {
                (-world_position.1 / self.line_height) as usize + 1
            } else {
                0usize
            };

            let last_line = lines.len() - 1;
            if row > last_line {
                row = last_line;
                let last_line_text = lines.last().map(|s| s.as_str()).unwrap_or("");
                let col = last_line_text.len();

                return Some((row, col))
            }

            let line = lines.get(row).unwrap();

            let mut column = 0usize;
            let mut width = 0f32;
            let mut chars = line.chars().peekable();
            
            while let Some(char) = chars.next() {
                let new_width = width + self.font.get_char_pixel_width(char, chars.peek().copied(), self.font_scale); 
                if new_width > world_position.0 {
                    if (new_width - world_position.0).abs() > (width - world_position.0).abs() {
                        break;
                    } else {
                        column += 1;
                        break;
                    }
                } else {
                    width = new_width;
                }
                    
                if width > world_position.0 { break }
                column += 1;
            }

            Some((row, column))
        } else {
            None
        }
    }

    pub fn world_position(&self, buffer: &Buffer, (row, col): (usize, usize)) -> (f32, f32) {
        let lines = buffer.lines();

        let y_pos = -1.0 * row as f32 * self.line_height;

        let current_line = lines.get(row).map(|s| s.as_str()).unwrap_or("");
        
        let actual_column = col.clamp(0, current_line.len());

        let preceding_text = current_line.get(0..actual_column).unwrap();
        let x_pos = self.font.get_str_pixel_width(preceding_text, self.font_scale);

        (x_pos, y_pos)
    }   
}

pub struct BufferPass<'a> {
    buffer: &'a Buffer,
    buffer_view: &'a BufferView,

    start_line: usize,
    end_line: usize
}

impl<'a> BufferPass<'a> {
    pub fn new(buffer: &'a Buffer, buffer_view: &'a BufferView) -> Self {
        let mut buffer_pass = Self {
            buffer,
            buffer_view,
            start_line: 0,
            end_line: 0,
        };

        buffer_pass.start_line = if buffer_pass.start_y() > 0f32 { 0usize } else { (- buffer_pass.start_y() / buffer_pass.line_height()) as usize };
        buffer_pass.end_line = if buffer_pass.end_y() > 0f32 { 0usize } else { (- buffer_pass.end_y() / buffer_pass.line_height()) as usize };

        buffer_pass
        
    }

    fn world_position(&self, buffer_position: (usize, usize)) -> (f32, f32) {
        self.buffer_view.world_position(self.buffer, buffer_position)
    }

    #[inline] fn lines(&self) -> &[String] { self.buffer.lines() }
    #[inline] fn line_height(&self) -> f32 { self.buffer_view.line_height }
    #[inline] fn font_scale(&self) -> f32 { self.buffer_view.font_scale }
    #[inline] fn font(&self) -> &Font { &self.buffer_view.font }
    #[inline] fn highlights(&self) -> &[Highlight] { &self.buffer.highlights }
    #[inline] fn colorscheme(&self) -> &ColorScheme { &self.buffer_view.colorscheme }
    #[inline] fn highlight_enabled(&self) -> bool { self.buffer.highlight_enabled }
   
    #[inline] fn start_y(&self) -> f32 { self.buffer_view.camera.view_top() }
    #[inline] fn end_y(&self) -> f32 { self.buffer_view.camera.view_bottom() }

    #[inline] fn buffer_ranges(&self) -> &[BufferRange] { self.buffer.selection.as_slice() }
    #[inline] fn cursors(&self) -> Vec<Cursor> { vec![self.buffer.cursor] }

    pub fn render_buffer_ranges(&self) -> Vec<Vertex> {
        let padding_width = self.font().get_char_pixel_width(' ', None, self.font_scale());

        self.buffer_ranges().iter().flat_map(|range| {
            let (start, end) = range.start_end();

            let mut vertices = Vec::new();

            for (line_num, line) in (start.0..end.0+1).zip(self.lines().get(start.0..end.0+1).unwrap().iter()) {
                let y = -1f32 * line_num as f32 * self.line_height();
                
                let start_x = 
                    if line_num == start.0 {
                        self.world_position(start).0
                    } else {
                        0f32
                    };

                let end_x = 
                    if line_num == end.0 {
                        self.world_position(end).0
                    } else {
                        self.font().get_str_pixel_width(line, self.font_scale()) + padding_width
                    };

                let width = end_x - start_x;

                vertices.extend(RectangleBuilder::default()
                    .position(start_x, y)
                    .size(width, self.line_height())
                    .depth(0.4)
                    .color(hex_color("#9ACCEA").unwrap())
                    .opacity(0.05)
                    .build());
            }

            vertices
        }).collect()

    }

    pub fn render_text(&self) -> Vec<Vertex> {
        let num_lines = self.end_line - self.start_line;

        let source_code_buffer = self.lines().join("\n");
        let start_byte: usize = source_code_buffer.lines().take(self.start_line).map(|l| l.len() + 1).sum();

        let mut vertices = Vec::new();

        let mut highlights = self.highlights().iter()
            .skip_while(|h| h.start_byte < start_byte)
            .peekable();

        let mut current_highlight_color = self.colorscheme().text_color;

        let mut line = 0usize;
        let mut offset_x = 0f32;
        let mut offset_y = -1.0 * self.start_line as f32 * self.line_height();
        for byte in start_byte.. {
            if line > num_lines {
                break
            }

            if self.highlight_enabled()  {
                if let Some(highlight) = highlights.peek() {
                    if highlight.end_byte <= byte {
                        highlights.next();
                        current_highlight_color = self.colorscheme().text_color;
                    }
                }

                if let Some(highlight) = highlights.peek() {
                    if highlight.start_byte <= byte {
                        let code_type = highlight.code_type
                            .map(|index| RUST_HIGHLIGHT_NAMES[index])
                            .unwrap_or("text_color");

                        current_highlight_color = 
                            get_highlight_for_code_type(code_type, self.colorscheme());
                    }
                }
            }

            let current_char = source_code_buffer.as_bytes().get(byte).copied();
            if current_char.is_none() { break }
            let current_char = current_char.unwrap() as char;

            if current_char == '\n' {
                offset_y -= self.line_height();
                line += 1;
                offset_x = 0.0;
                continue;
            }

            //otherwise print the character nicely
            let next_character = source_code_buffer.as_bytes().get(byte + 1).map(|c| *c as char);

            let (right, rectangle) = self.font().layout_character(
                current_char, 
                next_character, 
                (offset_x, offset_y), 
                self.font_scale(), 
                0.5
            ).unwrap();

            vertices.extend(rectangle.color(current_highlight_color).build());

            offset_x = right;
        }

        vertices
    }

    pub fn render_cursors(&self) -> Vec<Vertex> {
        self.cursors().iter().flat_map(|&Cursor(x, y)| {
            let (world_x, world_y) = self.world_position((x, y));

            RectangleBuilder::default()
                .position(world_x, world_y)
                .size(3f32, self.line_height())
                .depth(0.6)
                .build()
        }).collect()
    }
}

#[derive(Default)]
pub struct BufferRenderer {
    fonts: HashMap<String, MaterialHandle>,

    range_material: Option<MaterialHandle>,
    cursor_material: Option<MaterialHandle>,

    initialized: bool
}

impl BufferRenderer {
    fn create_render_work(vertices: Vec<Vertex>, material: MaterialHandle) -> RenderWork {
        let num_rectangles = vertices.len() / 4;
        let indices = (0..num_rectangles)
            .flat_map(|i| 
                Rectangle::INDICES.iter()
                .map(move |e| *e + (i * 4) as u32))
            .collect::<Vec<_>>();

        RenderWork {
            indices,
            vertices,
            material,
        }
    }
}

impl Subrenderer for BufferRenderer {
    fn init(&mut self, renderer: &mut RenderApi) {
        //initialize the font highlight materials
        let untextured_rectangle_pipeline = Pipeline::load::<Vertex>(include_str!("shaders/rect.wgsl")).unwrap();
        let pipeline_handle = renderer.create_pipeline(untextured_rectangle_pipeline);
        let basic_material = renderer.create_material(pipeline_handle).unwrap();

        self.cursor_material = Some(basic_material);
        self.range_material = Some(basic_material);

        self.initialized = true;
    }

    fn render(&mut self, world: &World, renderer: &mut RenderApi) -> Result<(), wgpu::SurfaceError> {
        if !self.initialized {
            panic!("Rendering a buffer before initializing the buffer renderer!")
        }
        
        for (buffer, view) in <(&Buffer, &BufferView)>::query().iter(world) { 
            let buffer_pass = BufferPass::new(buffer, view);

            let text_material = *(self.fonts.entry(view.font.name().to_string()).or_insert_with(|| {
                create_font_material(renderer, &view.font).unwrap()
            }));

            //update the materials using the camera
            let view_proj_matrix = Matrix::from(view.camera.matrix());

            renderer.update_material(text_material, "view_proj", view_proj_matrix.clone());
            renderer.update_material(self.range_material.unwrap(), "view_proj", view_proj_matrix.clone());
            renderer.update_material(self.cursor_material.unwrap(), "view_proj", view_proj_matrix.clone());

            let text_vertices = buffer_pass.render_text();
            let range_vertices = buffer_pass.render_buffer_ranges();
            let cursor_vertices = buffer_pass.render_cursors();

            let text_work = Self::create_render_work(text_vertices, text_material);
            let range_work = Self::create_render_work(range_vertices, self.range_material.unwrap());
            let cursor_work = Self::create_render_work(cursor_vertices, self.cursor_material.unwrap());

            renderer.submit_subrender(&[range_work, text_work, cursor_work], Some(&view.view))?;
        }

        Ok(())
    }
}