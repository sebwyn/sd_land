use std::collections::HashMap;
use std::io::Write;
use std::{fs::File, io::Read, path::Path};

use std::str;

use legion::{World, IntoQuery, EntityStore, Entity};
use simple_error::SimpleError;
use tree_sitter_highlight::{HighlightConfiguration, Highlighter, HighlightEvent};
use uuid::Uuid;
use winit::event::MouseButton;

use crate::colorscheme::{ColorScheme, RUST_HIGHLIGHT_NAMES, get_highlight_for_code_type};
use crate::renderer::camera::Camera;
use crate::renderer::primitive::{Vertex, RectangleBuilder};
use crate::system::{Event, Key};
use crate::text::Font;
use crate::renderer::view::{ViewRef, View};

#[derive(Clone, Copy)]
pub struct Cursor {
    entity: Entity,
    position: (usize, usize),
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
                    },
                    Key::Char(s, ..) if *s == 'p' => for buffer in <&mut Buffer>::query().iter_mut(world) {
                        let positions = buffer.cursors.iter().map(|c| c.position).collect::<Vec<_>>();
                        
                        for (i, position) in positions.iter().enumerate() {
                            let new_position = buffer
                                .insert_str_at("\nprintln!(\"Hello, World!\")\n", *position);
                            
                            buffer.cursors[i].position = new_position;
                        }
                    }
                    _ => {}
                }
            }
            if !modifiers.logo() && !modifiers.alt() {
                let character = match key {
                    Key::Char(_, uppercase) if modifiers.shift() && uppercase.is_some() => Some(uppercase.unwrap()),
                    Key::Char(lowercase, _) if !modifiers.shift() => Some(*lowercase),
                    _ => None
                };
                if let Some(character) = character {
                    for buffer in <&mut Buffer>::query().iter_mut(world) {
                        let positions = buffer.cursors.iter().map(|c| c.position).collect::<Vec<_>>();
    
                        for position in positions {
                            buffer.insert_at(character, position);
                        }
    
                        for i in 0..buffer.cursors.len() {
                            buffer.cursors[i].position = buffer.move_right(buffer.cursors[i].position)
                        }
                    }
                } else {
                    match key {
                        Key::Backspace => for buffer in <&mut Buffer>::query().iter_mut(world) {
                            let positions = buffer.cursors.iter().map(|c| c.position).collect::<Vec<_>>();
                            
                            for (i, position) in positions.iter().enumerate() {
                                buffer.cursors[i].position = buffer.move_left(buffer.cursors[i].position);
                                buffer.remove_at(*position);
                            }
                        },
                        Key::Return => for buffer in <&mut Buffer>::query().iter_mut(world) {
                            let positions = buffer.cursors.iter().map(|c| c.position).collect::<Vec<_>>();
        
                            for (i, position) in positions.iter().enumerate() {
                                buffer.insert_line(*position);
                                buffer.cursors[i].position.0 += 1;
                                buffer.cursors[i].position.1 = 0;
                            }
                        },
                        Key::Tab => for buffer in <&mut Buffer>::query().iter_mut(world) {
                            let positions = buffer.cursors.iter().map(|c| c.position).collect::<Vec<_>>();
                            
                            for (i, position) in positions.iter().enumerate() {
                                let new_position = buffer.insert_str_at("    ", *position);
                                buffer.cursors[i].position = new_position;
                            }

                        }
                        Key::Left => for buffer in <&mut Buffer>::query().iter_mut(world) {
                            for i in 0..buffer.cursors.len() {
                                buffer.cursors[i].position = buffer.move_left(buffer.cursors[i].position)
                            }
                        },
                        Key::Right => for buffer in <&mut Buffer>::query().iter_mut(world) {
                            for i in 0..buffer.cursors.len() {
                                buffer.cursors[i].position = buffer.move_right(buffer.cursors[i].position)
                            }
                        },
                        Key::Up => for buffer in <&mut Buffer>::query().iter_mut(world) {
                            for i in 0..buffer.cursors.len() {
                                buffer.cursors[i].position = buffer.move_up(buffer.cursors[i].position)
                            }
                        },
                        Key::Down => for buffer in <&mut Buffer>::query().iter_mut(world) {
                            for i in 0..buffer.cursors.len() {
                                buffer.cursors[i].position = buffer.move_down(buffer.cursors[i].position)
                            }
                        },
                        _ => {}
                    }
                }
            }
        },
        Event::MousePress(button, position, _) if matches!(button, MouseButton::Left) => {
            let mut buffers_and_positions = HashMap::new();

            for (buffer, view_ref) in <(&Buffer, &ViewRef)>::query().iter(world) {
                assert!(buffer.cursors.len() == 1);

                let view_entity = world.entry_ref(view_ref.0).unwrap();

                let view = view_entity.get_component::<View>().unwrap();
                let camera = view_entity.get_component::<Camera>().unwrap();

                if let Some(view_position) = view.to_view(position) {
                    let world_position = camera.view_to_world(view_position);
                    let buffer_position = buffer.buffer_position(world_position);

                    buffers_and_positions.insert(buffer.id, buffer_position);
                }
            }

            for buffer in <&mut Buffer>::query().iter_mut(world) {
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

    lines: Vec<String>,
    
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
                position: (0, 0),
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

        let lines = source_code.lines().map(|s| s.to_string()).collect::<Vec<_>>();

        //generate initial highlights if available
        let mut highlight_enabled = false;
        if let Some(extension) = file_path.extension() {
            if extension.to_str().unwrap() == "rs" {
                highlight_enabled = true
            }
        };

        // println!("{}", tree_sitter_rust::HIGHLIGHT_QUERY);

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

            lines,

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
        let source_code_buffer = self.lines.join("\n");


        let mut file = File::create(&self.file)
            .expect("Could not find file to save to");

        file.write_all(source_code_buffer.as_bytes()).expect("Failed to write to file!");
    }

    pub fn remove_at(&mut self, (row, mut col): (usize, usize)) {
        assert!(row < self.lines.len());

        //clamp the position to the lines columns
        col = col.clamp(0, self.lines[row].len());

        if col == 0 && row > 0 {
            //this line needs to removed
            let current_line = self.lines[row].clone();

            self.lines[row - 1].push_str(&current_line);
            self.lines.remove(row);
        } else {
            self.lines[row].remove(col - 1);
        }

        if self.highlight_enabled { self.update_highlights(); }
    }   

    pub fn insert_at(&mut self, character: char, (row, mut col): (usize, usize)) {
        col = col.clamp(0, self.lines[row].len());
        
        if let Some(line) = self.lines.get_mut(row) {
            line.insert(col, character);
        }

        if self.highlight_enabled { self.update_highlights(); }
    }

    pub fn insert_str_at(&mut self, str: &str, (row, mut col): (usize, usize)) -> (usize, usize) {
        col = col.clamp(0, self.lines[row].len());
        
        
        let (preceding_text, following_text) = self.lines[row].split_at(col);
        let preceding_text = preceding_text.to_string();
        let following_text = following_text.to_string();

        let mut current_row = row;

        let mut lines = str.split('\n').peekable();

        self.lines[current_row] = preceding_text;
        while let Some(line) = lines.next() {
            let current_line = &mut self.lines[current_row];
            *current_line += line;

            if lines.peek().is_some() {
                current_row += 1;
                self.insert_line((current_row, 0))
            }
        }
        let end_column = self.lines[current_row].len();
        self.lines[current_row] += &following_text;

        if self.highlight_enabled { self.update_highlights(); }

        (current_row, end_column)
    }

    pub fn insert_line(&mut self, (row, mut col): (usize, usize)) {
        col = col.clamp(0, self.lines[row].len());
        
        let (prev_line, new_line) = {
            let current_line = &self.lines[row];
            let (prev_line, new_line) = current_line.split_at(col);

            (prev_line.to_string(), new_line.to_string())
        };

        self.lines[row] = new_line;
        self.lines.insert(row, prev_line);

        if self.highlight_enabled { self.update_highlights(); }
    }

    pub fn update_highlights(&mut self) {
        let buffer = self.lines.join("\n");

        let highlights = self.highlighter.highlight(
            &self.rust_highlight_configuration, 
            buffer.as_bytes(), 
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

        let source_code_buffer = self.lines.join("\n");
        let start_byte: usize = source_code_buffer.lines().take(start_line).map(|l| l.len() + 1).sum();

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

            let current_char = source_code_buffer.as_bytes().get(byte).copied();
            if current_char.is_none() { break }
            let current_char = current_char.unwrap() as char;

            if current_char == '\n' {
                offset_y -= self.line_height;
                line += 1;
                offset_x = 0.0;
                continue;
            }

            //otherwise print the character nicely
            let next_character = source_code_buffer.as_bytes().get(byte + 1).map(|c| *c as char);

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

    pub fn buffer_position(&self, world_position: (f32, f32)) -> (usize, usize) {
        //calculate what line we're on
        let mut row = if world_position.1 < 0.0 {
            (-world_position.1 / self.line_height) as usize + 1
        } else {
            0usize
        };

        let last_line = self.lines.len() - 1;
        if row > last_line {
            row = last_line;
            let last_line_text = self.lines.last().map(|s| s.as_str()).unwrap_or("");
            let col = last_line_text.len();

            return (row, col)
        }

        let line = self.lines.get(row).unwrap();

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

        (row, column)
    }

    pub fn world_position(&self, (row, col): (usize, usize)) -> (f32, f32) {
        let y_pos = -1.0 * row as f32 * self.line_height;

        let current_line = self.lines.get(row).map(|s| s.as_str()).unwrap_or("");
        
        let actual_column = col.clamp(0, current_line.len());

        let preceding_text = current_line.get(0..actual_column).unwrap();
        let x_pos = self.font.get_str_pixel_width(preceding_text, self.font_scale);

        (x_pos, y_pos)
    }   

    pub fn move_right(&self, (row, col): (usize, usize)) -> (usize, usize) {
        let current_line = self.lines.get(row).unwrap();

        if col < current_line.len() {
            (row, col + 1)
        } else if row < self.lines.len() - 1 {
            (row + 1, 0)
        } else {
            (row, col)
        }
    }

    pub fn move_left(&self, (row, mut col): (usize, usize)) -> (usize, usize) {
        let current_line = self.lines.get(row).unwrap();

        col = col.clamp(0, current_line.len());

        if col > 0 {
            (row, col - 1)
        } else if row > 0 {
            let previous_row = &self.lines[row - 1];
            (row - 1, previous_row.len())
        } else {
            (row, col)
        }
    }

    pub fn move_up(&self, (row, col): (usize, usize)) -> (usize, usize) {
        if row > 0 {
            (row - 1, col)
        } else {
            (row, col)
        }
    }

    pub fn move_down(&self, (row, col): (usize, usize)) -> (usize, usize) {
        if row < self.lines.len() - 1 {
            (row + 1, col)
        } else {
            (row, col)
        }
    }
}
