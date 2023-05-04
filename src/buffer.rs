use std::collections::HashMap;
use std::{fs::File, io::Read, path::Path};

use std::str;

use legion::{World, IntoQuery};
use simple_error::SimpleError;
use tree_sitter::{Node, QueryCursor, Tree, Parser, InputEdit, Point, TreeCursor};
use winit::dpi::PhysicalPosition;

use crate::camera::Camera;
use crate::graphics::Vertex;
use crate::system::Event;
use crate::text::Font;
use crate::view::{ViewRef, View};
use crate::ui_box::hex_color;

//line_height: 50f32, scale: 

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
    line_number_color: [f32; 3],  
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

pub fn buffer_on_event(world: &mut World, event: &Event) {
    match event {
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

type HighlightGroups = Vec<(String, usize, usize)>;

pub struct Buffer {
    _file: String,
    source_code: String,
    
    line_height: f32,
    colorscheme: ColorScheme,

    font_scale: f32,
    font: Font,

    treesitter_tree: Option<Tree>,
    highlights: Option<Vec<HighlightGroups>>,
    treesitter_parser: Option<Parser>,
}


impl Buffer {
    pub fn remove_at(&mut self, position: (usize, (usize, usize))) {
        self.source_code.remove(position.0 - 1);

        //update the tree and relevant highlights
        if let Some(tree) = &mut self.treesitter_tree {
            
            let input_edit = InputEdit { 
                start_byte: position.0 - 1, 
                old_end_byte: position.0 + 1, 
                new_end_byte: position.0,

                start_position: Point { row: position.1.0, column: position.1.1 - 1 }, 
                old_end_position: Point { row: position.1.0, column: position.1.1 + 1}, 
                new_end_position: Point { row: position.1.0, column: position.1.1 },
            };

            tree.edit(&input_edit);
            self.treesitter_tree = Some(self.treesitter_parser
                .as_mut()
                .unwrap()
                .parse(self.source_code.as_bytes(), Some(tree))
                .unwrap()
            );

            self.micro_update_highlight(position)
        }

    }

    pub fn insert_at(&mut self, character: char, position: (usize, (usize, usize))) {
        self.source_code.insert(position.0, character);

        //update the tree and relevant highlights
        if let Some(tree) = &mut self.treesitter_tree {
            
            let input_edit = InputEdit { 
                start_byte: position.0, 
                old_end_byte: position.0 + 1, 
                new_end_byte: position.0 + 2,

                start_position: Point { row: position.1.0, column: position.1.1 }, 
                old_end_position: Point { row: position.1.0, column: position.1.1 + 1}, 
                new_end_position: Point { row: position.1.0, column: position.1.1 + 2},
            };

            tree.edit(&input_edit);
            self.treesitter_tree = Some(self.treesitter_parser
                .as_mut()
                .unwrap()
                .parse(self.source_code.as_bytes(), Some(tree))
                .unwrap()
            );

            self.micro_update_highlight(position)
        }

    }

    pub fn micro_update_highlight(&mut self, position: (usize, (usize, usize))) {
        let root_node = self.treesitter_tree.as_ref().unwrap().root_node();
        //get the byte offsets of the line
        let line = self.source_code.lines().skip(position.1.0).next().unwrap();
        let line_len = line.len();

        let start_line = position.0 - position.1.1;
        let end_line = start_line + line_len;

        println!("{}", line);
        println!("LINE: {}", &self.source_code[start_line..end_line]);

        let node = root_node.descendant_for_byte_range(start_line, end_line).unwrap();
        println!("FOUND line node: {:?}", node.utf8_text(self.source_code.as_bytes()));

        println!("{}", node.to_sexp());
        println!("{}", node.start_byte());
        println!("{}", node.end_byte());
        println!("{:?}", self.source_code.get(node.start_byte()..node.end_byte()));

        //execute a query on that node and update all the lines
        let mut new_highlights: HashMap<usize, HighlightGroups> = HashMap::new();
        for (line, highlight) in query_highlight(node, &self.source_code).into_iter() {
            let highlight_group = new_highlights.entry(line)
                .or_insert(Vec::new());
        
            highlight_group.push(highlight);
        }    


        for (line, highlight_group) in new_highlights.into_iter() {
            let highlights = self.highlights
                .as_mut()
                .unwrap()
                .get_mut(line)
                .unwrap();

            println!("Updating highlight for line: {} og: {}: {:#?}", line, position.1.0, highlight_group);

            highlights.clear();
            highlights.extend(highlight_group);
            highlights.sort_by(|(_, start_a, _), (_, start_b, _)| start_a.cmp(start_b));
            highlights.dedup();
        }

    }

    pub fn render(&self, start_y: f32, end_y: f32) -> Vec<Vertex> {
        let start_line = if start_y > 0f32 { 0usize } else { (-start_y / self.line_height) as usize };
        let end_line = if end_y > 0f32 { 0usize } else { (-end_y / self.line_height) as usize };
        
        let mut vertices = Vec::new();

        let mut vertical_offset = -1f32 * (start_line as f32 * self.line_height);
        for (number, line) in self.source_code.split('\n').enumerate().take(end_line).skip(start_line) {
            if line.len() == 0 { vertical_offset -= self.line_height; continue;  }
            
            let mut horizontal_offset = 0f32;
            //render the line numbers
            if self.highlights.is_some() {
                let segmented_line = self.segment_line_by_highlight(number, line);
                for (color, segment) in segmented_line {
                    if segment.len() == 0 { continue }
                    
                    let (bounds, rectangles) = self.font.layout_text(
                        segment,
                        (horizontal_offset, vertical_offset),
                        self.font_scale,
                        0.5
                    ).unwrap();

                    vertices.extend(rectangles.into_iter().flat_map(|r| r.color(color).build().vertices));
                    horizontal_offset = bounds.right;
                }
            } else {
                let (bounds, rectangles) = self.font.layout_text(
                    line,
                    (horizontal_offset, vertical_offset),
                    self.font_scale,
                    0.5
                ).unwrap();

                vertices.extend(rectangles.into_iter().flat_map(|r| r.color(self.colorscheme.text_color).build().vertices));
            }

            vertical_offset -= self.line_height;
        }

        vertices
    }

    pub fn buffer_position(&self, world_position: (f32, f32)) -> (usize, (usize, usize)) {
        //calculate what line we're on
        let row = if world_position.1 < 0.0 {
            (-world_position.1 as f32 / self.line_height) as usize + 1
        } else {
            0usize
        };

        let line_character_offset: usize = self.source_code
            .split('\n')
            .take(row)
            .map(|line| line.len() + 1)
            .sum();

        let line = self.source_code.lines().skip(row).next().unwrap();

        let mut column = 0usize;
        let mut width = 0f32;
        let mut chars = line.chars().peekable();
        loop {
            match chars.next() {
                Some(c) => { 
                    let new_width = width + self.font.get_char_pixel_width(c, chars.peek().copied(), self.font_scale); 
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
                },
                None => break,
            }
            column += 1;
        }

        let position = (line_character_offset + column, (row, column));
        println!("{:?}", position);
        position
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
            .map_err(|e| SimpleError::new(&format!("Failed to load the file: {}", e.to_string())))?;
        file.read_to_string(&mut source_code).map_err(|_| SimpleError::new("Failed to read file!"))?;

        let mut treesitter_tree = None;
        let mut treesitter_parser = None;
        let mut highlights = None;
        
        //generate initial highlights if available
        if let Some(extension) = file_path.extension() {
            match extension.to_str().unwrap() {
                "rs" => {
                    let mut parser = tree_sitter::Parser::new();
                    parser.set_language(tree_sitter_rust::language())
                        .map_err(|_| SimpleError::new("Failed to load treesitter language"))?;
                    let tree = parser.parse(source_code.as_bytes(), None)
                        .ok_or(SimpleError::new("Identified rust file, but failed to parse!"))?;
                    
                    highlights = Some(Self::generate_highlights(&tree, &source_code));
                    treesitter_parser = Some(parser);
                    treesitter_tree = Some(tree);
                }
                _ => {}
            }
        };

        Ok(Self {
            _file: file_name.to_string(),
            source_code,

            line_height,
            colorscheme,

            font_scale,
            font,

            treesitter_tree,
            treesitter_parser,
            highlights,

        })
    }

    fn generate_highlights(tree: &Tree, source_code: &str) -> Vec<HighlightGroups> {
        let num_lines = source_code.lines().count();

        let mut highlights_by_row = vec![Vec::new(); num_lines];

        for (row, highlight) in query_highlight(tree.root_node(), source_code).into_iter() {
            highlights_by_row.get_mut(row)
                .expect("Highlight query identified out of bounds line")
                .push(highlight);
        }

        for highlight in highlights_by_row.iter_mut() {
            highlight.sort_by(|(_, start_a, _), (_, start_b, _)| start_a.cmp(start_b));
            highlight.dedup_by(|(_, start_a, _), (_, start_b, _)| start_a == start_b);
        }

        highlights_by_row
    }

    fn segment_line_by_highlight<'a>(&self, line_num: usize, line: &'a str) -> Vec<([f32; 3], &'a str)> {
        let highlights = self.highlights.as_ref().unwrap().get(line_num).unwrap();
        
        let mut words = Vec::new();
        let mut current_position = 0;
        for (code_type, start, end) in highlights {
            if current_position > *start { continue }
            words.push((get_highlight_for_code_type("", &self.colorscheme), &line[current_position..*start]));
            words.push((get_highlight_for_code_type(code_type, &self.colorscheme), &line[*start..*end]));
            current_position = *end;
        }
        words.push((get_highlight_for_code_type("", &self.colorscheme), line.split_at(current_position).1));

        words
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

fn query_string() -> String {
    const QUERY_ADDITION: &'static str = 
r#"
"=>" @punctuation.delimiter
"->" @punctuation.delimeter
"|" @operator
"=" @operator
"+" @operator
"-" @operator
"#;

    format!("{}{}", tree_sitter_rust::HIGHLIGHT_QUERY, QUERY_ADDITION)
}



fn query_highlight(node: Node, text: &str) -> Vec<(usize, (String, usize, usize))> {
    let query = tree_sitter::Query::new(tree_sitter_rust::language(), &query_string()).unwrap();

    QueryCursor::new().matches(&query, node, text.as_bytes()).flat_map(|cap| {
        let mut nodes = Vec::new();

        for c in cap.captures {
            nodes.push(
                (
                    c.node.start_position().row, 
                    (
                        query.capture_names()[c.index as usize].clone(), 
                        c.node.start_position().column, 
                        c.node.end_position().column
                    )
                )
            );
        }

        nodes
    }).collect::<Vec<_>>()
}