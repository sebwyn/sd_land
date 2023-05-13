use std::collections::HashMap;
use std::io::Write;
use std::{fs::File, io::Read, path::Path};

use std::str;

use legion::{World, IntoQuery, EntityStore, Entity};
use simple_error::SimpleError;
use tree_sitter_highlight::{HighlightConfiguration, Highlighter, HighlightEvent};
use uuid::Uuid;
use winit::event::MouseButton;

use crate::camera::Camera;
use crate::graphics::{Vertex, RectangleBuilder};
use crate::system::{Event, Key};
use crate::text::Font;
use crate::view::{ViewRef, View};
use crate::ui_box::hex_color;

pub struct ColorScheme {
    text_color: [f32; 3],
    keyword_color: [f32; 3],
    type_color: [f32; 3],
    function_color: [f32; 3],
    string_color: [f32; 3],
    primitive_color: [f32; 3],
    property_color: [f32; 3],
    operator_color: [f32; 3],
    comment_color: [f32; 3],
    punctuation_color: [f32; 3],
    line_number_color: [f32; 3]
}

impl Default for ColorScheme {
    fn default() -> Self {
        ColorSchemeBuilder::default().build().unwrap()  
    }
}

pub struct ColorSchemeBuilder {
    text_color: &'static str,        //"#F64740"
    keyword_color: &'static str,     //"#7a28cb"
    type_color: &'static str,        //"#ffd952"
    function_color: &'static str,    //"#166088"
    string_color: &'static str,      //"#4AAD52"
    primitive_color: &'static str,   //"#DC2E3F"
    property_color: &'static str,    //"#F489FA"
    operator_color: &'static str,    //"#FFFFFF"
    comment_color: &'static str,     //"#676779"
    punctuation_color: &'static str, //"#FFFFFF"
    line_number_color: &'static str, //"#FFFFFF"
}

impl Default for ColorSchemeBuilder {
    fn default() -> Self {
        Self { 
            text_color:        "#F64740", 
            keyword_color:     "#7a28cb", 
            type_color:        "#ffd952", 
            function_color:    "#166088", 
            string_color:      "#4AAD52", 
            primitive_color:   "#DC2E3F", 
            property_color:    "#F489FA", 
            operator_color:    "#FFFFFF", 
            comment_color:     "#676779", 
            punctuation_color: "#FFFFFF", 
            line_number_color: "#FFFFFF", 
        }
    }
}

impl ColorSchemeBuilder {
    pub fn build(&self) -> Result<ColorScheme, SimpleError> {
        Ok(ColorScheme {
            text_color: hex_color(self.text_color).map_err(|_| SimpleError::new("Invalid hex format!"))?,
            keyword_color: hex_color(self.keyword_color).map_err(|_| SimpleError::new("Invalid hex format!"))?,
            type_color: hex_color(self.type_color).map_err(|_| SimpleError::new("Invalid hex format!"))?,
            function_color: hex_color(self.function_color).map_err(|_| SimpleError::new("Invalid hex format!"))?,
            string_color: hex_color(self.string_color).map_err(|_| SimpleError::new("Invalid hex format!"))?,
            primitive_color: hex_color(self.primitive_color).map_err(|_| SimpleError::new("Invalid hex format!"))?,
            property_color: hex_color(self.property_color).map_err(|_| SimpleError::new("Invalid hex format!"))?,
            operator_color: hex_color(self.operator_color).map_err(|_| SimpleError::new("Invalid hex format!"))?,
            comment_color: hex_color(self.comment_color).map_err(|_| SimpleError::new("Invalid hex format!"))?,
            punctuation_color: hex_color(self.punctuation_color).map_err(|_| SimpleError::new("Invalid hex format!"))?,
            line_number_color: hex_color(self.line_number_color).map_err(|_| SimpleError::new("Invalid hex format!"))?,
        })
    }

    pub fn text_color(mut self, text_color: &'static str) -> Self { self.text_color = text_color; self }
    pub fn keyword_color(mut self, keyword_color: &'static str) -> Self { self.keyword_color = keyword_color; self }
    pub fn type_color(mut self, type_color: &'static str) -> Self { self.type_color = type_color; self }
    pub fn function_color(mut self, function_color: &'static str) -> Self { self.function_color = function_color; self }
    pub fn string_color(mut self, string_color: &'static str) -> Self { self.string_color = string_color; self }
    pub fn primitive_color(mut self, primitive_color: &'static str) -> Self { self.primitive_color = primitive_color; self }
    pub fn property_color(mut self, property_color: &'static str) -> Self { self.property_color = property_color; self }
    pub fn operator_color(mut self, operator_color: &'static str) -> Self { self.operator_color = operator_color; self }
    pub fn comment_color(mut self, comment_color: &'static str) -> Self { self.comment_color = comment_color; self }
    pub fn punctuation_color(mut self, punctuation_color: &'static str) -> Self { self.punctuation_color = punctuation_color; self }
    pub fn line_number_color(mut self, line_number_color: &'static str) -> Self { self.line_number_color = line_number_color; self }

}

const RUST_HIGHLIGHT_NAMES: &[&str] = &[
    "function",
    "function.method",
    "function.macro",
    "type",
    "type.builtin",
    "constructor",
    "keyword",
    "escape",
    "constant.builtin",
    "property",
    "operator",
    "comment",
    "string",
    "punctuation"
];

#[derive(Clone, Copy)]
pub struct Cursor {
    entity: Entity,
    position: usize,
}

pub fn buffer_on_event(world: &mut World, event: &Event) { 
    match event {
        Event::PrepareRender => {
            let mut camera_query = <(&Buffer, &ViewRef)>::query();

            let mut cameras = HashMap::new();

            for (_, view) in camera_query.iter(world) {
                
                cameras.entry(view.0).or_insert_with(|| {
                    let camera_entity = world.entry_ref(view.0).expect("Expected buffer to be in a view");
                    let camera = camera_entity.get_component::<Camera>()
                        .expect("Expected View to have an associated camera!");

                    camera.clone()
                });
            }

            let mut buffer_query = <(&Buffer, &mut Vec<Vertex>, &ViewRef)>::query();
            
            let mut cursors: Vec<(Cursor, Vec<Vertex>)> = Vec::new();

            for (buffer, vertices, view) in buffer_query.iter_mut(world) {
                let camera = cameras.get(&view.0).expect("No camera found for view!");
                
                let view_top = camera.view_top();
                let view_bottom = camera.view_bottom();

                let new_vertices = buffer.render(view_top, view_bottom);

                *vertices = new_vertices;

                for cursor in &buffer.cursors {
                    //generate a position and a rectangle for the cursor
                    let (world_x, mut world_y) = buffer.world_position(cursor.position);
                    world_y += buffer.font.smallest_ymin(buffer.font_scale);
                    // let height = buffer.font.font_height(buffer.font_scale);


                    let vertices = RectangleBuilder::default()
                        .position(world_x, world_y)
                        .size(3f32, buffer.line_height)
                        .depth(0.4)
                        .build();
                    
                    cursors.push((*cursor, vertices));
                }
            }

            //draw the cursors
            for (cursor, new_vertices) in cursors {
                let mut cursor_entity = world.entry(cursor.entity).unwrap();
                let vertices = cursor_entity.get_component_mut::<Vec<Vertex>>().unwrap();
                *vertices = new_vertices;
            }
        },
        Event::KeyPress(key, modifiers) => {
            if modifiers.logo() && !modifiers.shift() && !modifiers.alt() {
                match key {
                    Key::Char(s, ..) if *s == 's' => {
                        let mut query = <&Buffer>::query();
                        for buffer in query.iter(world) {
                            buffer.save();
                        }
                    }
                    _ => {}
                }
            }

            if !modifiers.logo() && !modifiers.alt() {
                let character = match key {
                    Key::Char(_, uppercase) if modifiers.shift() && uppercase.is_some() => Some(uppercase.unwrap()),
                    Key::Char(lowercase, _) if !modifiers.shift() => Some(*lowercase),
                    Key::Tab => Some('\t'),
                    Key::Return => Some('\n'),
                    _ => None
                };
                
                if let Some(character) = character {
                    for buffer in <&mut Buffer>::query().iter_mut(world) {
                        let positions = buffer.cursors.iter().map(|c| c.position).collect::<Vec<_>>();
    
                        for position in positions {
                            buffer.insert_at(character, position);
                        }
    
                        for cursor in buffer.cursors.iter_mut() {
                            cursor.position += 1;
                        }
                    }
                } else {
                    match key {
                        Key::Backspace => for buffer in <&mut Buffer>::query().iter_mut(world) {
                            let positions = buffer.cursors.iter().map(|c| c.position).collect::<Vec<_>>();
        
                            for position in positions {
                                buffer.remove_at(position);
                            }
        
                            for cursor in buffer.cursors.iter_mut() {
                                cursor.position -= 1;
                            }
                        },
                        Key::Left => for buffer in <&mut Buffer>::query().iter_mut(world) {
                            for cursor in buffer.cursors.iter_mut() {
                                cursor.position -= 1;
                            }
                        },
                        Key::Right => for buffer in <&mut Buffer>::query().iter_mut(world) {
                            for cursor in buffer.cursors.iter_mut() {
                                cursor.position += 1;
                            }
                        },
                        _ => {}
                    }
                }
            }
        },
        Event::MousePress(button, position, key_modifiers) if matches!(button, MouseButton::Left) => {
            let mut buffers_and_positions = HashMap::new();

            for (buffer, view_ref) in <(&Buffer, &ViewRef)>::query().iter(world) {
                assert!(buffer.cursors.len() == 1);

                let view_entity = world.entry_ref(view_ref.0).unwrap();

                let view = view_entity.get_component::<View>().unwrap();
                let camera = view_entity.get_component::<Camera>().unwrap();

                if let Some(view_position) = view.to_view(position) {
                    let world_position = camera.view_to_world(view_position);
                    let (buffer_position, _) = buffer.buffer_position(world_position);

                    buffers_and_positions.insert(buffer.id, buffer_position);
                }
            }

            for (i, buffer) in <&mut Buffer>::query().iter_mut(world).enumerate() {
                if let Some(buffer_position) = buffers_and_positions.get(&buffer.id) {
                    buffer.cursors[0].position = *buffer_position;
                } 
            }
        },
        Event::MouseScroll(scroll, position) => {
            let mut query = <(&Buffer, &ViewRef)>::query();
            
            //sort the elements by depth so we find the one on top

            let view_entities = 
                query.iter(world).map(|(_, view)| view.0).collect::<Vec<_>>();

            for entity in view_entities {
                let mut view_entry = match world.entry(entity) {
                    Some(entry) => entry,
                    None => continue,
                };

                let view = match view_entry.get_component_mut::<View>() {
                    Ok(view) => view,
                    Err(_) => continue,
                };

                if view.contains_point(position) {
                    let camera = match view_entry.get_component_mut::<Camera>() {
                        Ok(camera) => camera,
                        Err(_) => continue,
                    };

                    //scroll the camera
                    camera.eye.y += scroll.y as f32;
                    camera.target.y = camera.eye.y;

                    break;
                }
            }
        },
        _ => {}
    }
}

#[derive(Debug)]
struct Highlight {
    code_type: Option<usize>,
    start_byte: usize,
    end_byte: usize,
}

pub struct Buffer {
    id: Uuid,

    file: String,
    source_code: String,
    
    line_height: f32,
    font_scale: f32,
    font: Font,

    cursors: Vec<Cursor>,

    colorscheme: ColorScheme,

    highlight_enabled: bool,
    rust_highlight_configuration: HighlightConfiguration,
    highlighter: Highlighter,

    highlights: Vec<Highlight>,
}


impl Buffer {
    pub fn insert_cursor(&mut self, entity: Entity) {
        self.cursors.push(
            Cursor {
                entity,
                position: 0,
            }
        );
    }

    pub fn load(file_name: &str, line_height: f32, colorscheme: ColorScheme, font: Font, font_scale: f32) -> Result<Self, SimpleError> {
        let file_path = Path::new(file_name);
        if !file_path.exists() {
            return Err(SimpleError::new("File does not exist!"));
        }

        if !file_path.is_file() {
            return Err(SimpleError::new("Attempting to load a directory!"));
        }

        let mut source_code = String::new();
        let mut file = File::open(file_path)
            .map_err(|e| SimpleError::new(format!("Failed to load the file: {}", e.to_string())))?;
        file.read_to_string(&mut source_code).map_err(|_| SimpleError::new("Failed to read file!"))?;

        //generate initial highlights if available
        let mut highlight_enabled = false;
        if let Some(extension) = file_path.extension() {
            if extension.to_str().unwrap() == "rs" {
                highlight_enabled = true
            }
        };

        let mut rust_highlight_configuration = HighlightConfiguration::new(
                tree_sitter_rust::language(),
                tree_sitter_rust::HIGHLIGHT_QUERY,
                "",
                "",
        ).unwrap();
        rust_highlight_configuration.configure(RUST_HIGHLIGHT_NAMES);

        let highlighter = Highlighter::new();

        let id = Uuid::new_v4();

        let mut buffer = Self {
            id,
            file: file_name.to_string(),
            source_code,

            line_height,
            font_scale,
            font,

            cursors: Vec::new(),

            colorscheme,

            highlight_enabled,
            rust_highlight_configuration,
            highlighter,

            highlights: Vec::new()
        };

        if buffer.highlight_enabled { buffer.update_highlights() }

        Ok(buffer)
    }

    pub fn save(&self) {
        let mut file = File::create(&self.file)
            .expect("Could not find file to save to");

        file.write_all(self.source_code.as_bytes()).expect("Failed to write to file!");
    }

    pub fn remove_at(&mut self, position: usize) {
        self.source_code.remove(position - 1);

        if self.highlight_enabled { self.update_highlights(); }
    }   

    pub fn insert_at(&mut self, character: char, position: usize) {
        self.source_code.insert(position, character);

        if self.highlight_enabled { self.update_highlights(); }
    }

    pub fn update_highlights(&mut self) {
        let highlights = self.highlighter.highlight(
            &self.rust_highlight_configuration, 
            self.source_code.as_bytes(), 
            None, 
            |_| None).unwrap();
        
        self.highlights.clear();
        
        let mut currently_no_higlighting = true;
        for event in highlights {
            match event.unwrap() {
                HighlightEvent::Source {start, end} => {
                    if currently_no_higlighting {
                        self.highlights.push(Highlight { code_type: None, start_byte: start, end_byte: end})
                    } else {
                        let last_highlight = self.highlights.last_mut().expect("Can't find last highlight");
                        last_highlight.start_byte = start;
                        last_highlight.end_byte = end;
                    }
                },
                HighlightEvent::HighlightStart(s) => {
                    self.highlights.push(Highlight { code_type: Some(s.0), start_byte: 0, end_byte: 0});
                    currently_no_higlighting = false;
                },
                HighlightEvent::HighlightEnd => {
                    currently_no_higlighting = true;
                },
            }
        }
    }

    pub fn render(&self, start_y: f32, end_y: f32) -> Vec<Vertex> {
        let start_line = if start_y > 0f32 { 0usize } else { (-start_y / self.line_height) as usize };
        let end_line = if end_y > 0f32 { 0usize } else { (-end_y / self.line_height) as usize };
        
        let num_lines = end_line - start_line;

        let start_byte: usize = self.source_code.lines().take(start_line).map(|l| l.len() + 1).sum();

        let mut vertices = Vec::new();

        let mut highlights = self.highlights.iter()
            .skip_while(|h| h.start_byte < start_byte)
            .peekable();

        let mut current_highlight_color = self.colorscheme.text_color;

        let mut line = 0usize;
        let mut offset_x = 0f32;
        let mut offset_y = -1.0 * start_line as f32 * self.line_height;
        for byte in start_byte.. {
            if line > num_lines {
                break
            }

            if self.highlight_enabled  {
                if let Some(highlight) = highlights.peek() {
                    if highlight.end_byte <= byte {
                        highlights.next();
                        current_highlight_color = self.colorscheme.text_color;
                    }
                }

                if let Some(highlight) = highlights.peek() {
                    if highlight.start_byte <= byte {
                        let code_type = highlight.code_type
                            .map(|index| RUST_HIGHLIGHT_NAMES[index])
                            .unwrap_or("text_color");

                        current_highlight_color = 
                            get_highlight_for_code_type(code_type, &self.colorscheme);
                    }
                }
            }

            let current_char = self.source_code.as_bytes().get(byte).copied();
            if current_char.is_none() { break }
            let current_char = current_char.unwrap() as char;

            if current_char == '\n' {
                offset_y -= self.line_height;
                line += 1;
                offset_x = 0.0;
                continue;
            }

            //otherwise print the character nicely
            let next_character = self.source_code.as_bytes().get(byte + 1).map(|c| *c as char);

            let (right, rectangle) = self.font.layout_character(
                current_char, 
                next_character, 
                (offset_x, offset_y), 
                self.font_scale, 
                0.5
            ).unwrap();

            vertices.extend(rectangle.color(current_highlight_color).build());

            offset_x = right;
        }

        vertices
    }

    pub fn buffer_position(&self, world_position: (f32, f32)) -> (usize, (usize, usize)) {
        //calculate what line we're on
        let row = if world_position.1 < 0.0 {
            (-world_position.1 / self.line_height) as usize + 1
        } else {
            0usize
        };

        let line_character_offset: usize = self.source_code
            .split('\n')
            .take(row)
            .map(|line| line.len() + 1)
            .sum();

        let line = self.source_code.lines().nth(row).unwrap();

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

        let position = (line_character_offset + column, (row, column));
        position
    }

    pub fn world_position(&self, position: usize) -> (f32, f32) {
        let line_lengths = self.source_code.lines().map(|l| l.len() + 1);

        let mut lines_before = 0;
        let mut characters_before = 0;
        for line_length in line_lengths {
            let new_length = characters_before + line_length;
            if new_length > position {
                break;
            } else {
                characters_before = new_length;
                lines_before += 1;
            }
        }

        let text_before = self.source_code.get(characters_before..position).unwrap();  

        let column = position - characters_before;

        let width = self.font.get_str_pixel_width(text_before, self.font_scale);

        (width, -1.0 * self.line_height * lines_before as f32)
    }   
}

fn get_highlight_for_code_type(code_type: &str, color_scheme: &ColorScheme) -> [f32; 3] {
    match code_type {

        "function" | "function.method" | "function.macro" => color_scheme.function_color,
        "type" | "type.builtin" | "constructor" => color_scheme.type_color,
        "keyword" | "escape" => color_scheme.keyword_color,
        
        "constant.builtin" => color_scheme.primitive_color,
        "property" => color_scheme.property_color,
        "operator" => color_scheme.operator_color,
        "comment" => color_scheme.comment_color,
        "string" => color_scheme.string_color,

        a if a.contains("punctuation") 
            => color_scheme.punctuation_color,

        _ => color_scheme.text_color,
    }
}