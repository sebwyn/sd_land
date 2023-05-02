use std::{collections::HashMap, fs::File, io::Read, path::Path};

use std::str;

use legion::{World, Entity};
use simple_error::SimpleError;
use tree_sitter::{Node, QueryCursor, Tree};

use crate::graphics::Rectangle;
use crate::renderer::{Renderer, MaterialHandle, RenderStage};
use crate::view::ViewRef;
use crate::{text::TextBoxFactory, ui_box::hex_color};

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

pub struct Buffer {
    _file: String,
    source_code: String,
    tree: Option<Tree>,
}

impl Buffer {
    pub fn load(file_name: &str) -> Result<Self, SimpleError> {
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

        let mut tree = None;
        if let Some(extension) = file_path.extension() {
            if extension == "rs" {
                let mut parser = tree_sitter::Parser::new();
                parser.set_language(tree_sitter_rust::language())
                    .map_err(|_| SimpleError::new("Failed to load treesitter language"))?;
                tree = Some(parser.parse(source_code.as_bytes(), None)
                    .ok_or("Identified rust file, but Treesitter failed!")?
                );
            }
        }

        Ok(Self {
            _file: file_name.to_string(),
            source_code,
            tree,

        })
    }

    pub fn emplace_in_view(
        &self,
        renderer: &mut Renderer,
        world: &mut World, 
        view: Entity, 
        color_scheme: Option<ColorScheme>,
        line_height: f32,
        font_scale: f32,
        font: &str
    ) {
        let text_box_factory = TextBoxFactory::new(renderer, font).unwrap();
        
        let view_less_elements = 
        if let Some(tree) = &self.tree {
            let color_scheme = color_scheme.unwrap_or_else(|| ColorSchemeBuilder::default().build().unwrap() );
            self.generate_code_with_highlight(tree, &text_box_factory, &color_scheme, line_height, font_scale)
        } else {
            let color_scheme = color_scheme.unwrap_or_else(|| 
                ColorSchemeBuilder::default().text_color("#FFFFFF").build().unwrap() 
            );
            self.generate(&text_box_factory, &color_scheme, line_height, font_scale)
        };

        let elements: Vec<(ViewRef, Rectangle, MaterialHandle, RenderStage)> = 
            view_less_elements
                .into_iter()
                .map(|components| (ViewRef(view), components.0, components.1, components.2))
                .collect();

        world.extend(elements);
    }

    fn generate(
        &self,
        text_box_factory: &TextBoxFactory,
        color_scheme: &ColorScheme,
        line_height: f32,
        font_scale: f32
    ) -> Vec<(Rectangle, MaterialHandle, RenderStage)> {
        let mut text_components = Vec::new();
        for (i, line) in self.source_code.split('\n').enumerate() {
            let line_num = i;

            let y_position = 1200f32 - ((i + 1) as f32 * line_height);

            let line_num_str = format!("{}", line_num+1);
            let line_num_str = format!("{:indent$}", line_num_str, indent=(8-line_num_str.len()));

            let (bounds, characters) = 
                    text_box_factory.create( &line_num_str, (10f32, y_position), 0.9, font_scale, color_scheme.line_number_color);
                text_components.extend(characters);

            if line.len() == 0 {
                continue
            }
        
            let (_, characters) = 
                text_box_factory.create(line ,(bounds.right, y_position), 0.9, font_scale, color_scheme.text_color);
            text_components.extend(characters);
        }
        text_components
    }

    fn generate_code_with_highlight(
        &self,
        tree: &Tree,
        text_box_factory: &TextBoxFactory,
        color_scheme: &ColorScheme,
        line_height: f32,
        font_scale: f32
    ) -> Vec<(Rectangle, MaterialHandle, RenderStage)> {

        let mut highlights_by_row: HashMap<usize, Vec<(String, usize, usize)>> = HashMap::new();

        for (row, highlight) in query_highlight(tree.root_node(), &self.source_code).into_iter() {
            let v = highlights_by_row.entry(row)
                .or_insert(vec![]);

            v.push(highlight);
        }

        let mut text_components = Vec::new();
        for (i, line) in self.source_code.split('\n').enumerate() {
            let line_num = i;

            let y_position = 1200f32 - ((i + 1) as f32 * line_height);

            let line_num_str = format!("{}", line_num+1);
            let line_num_str = format!("{:indent$}", line_num_str, indent=(8-line_num_str.len()));

            let (bounds, characters) = 
                    text_box_factory.create( &line_num_str, (10f32, y_position), 0.9, font_scale, color_scheme.line_number_color);
                text_components.extend(characters);

            let mut current_offset = bounds.right;

            if line.len() == 0 {
                continue
            }

            if let Some(mut highlights) = highlights_by_row.get(&line_num).cloned() {
                //break the line up by highlight groups
                highlights.sort_by(|(_, start_a, _), (_, start_b, _)| start_a.cmp(start_b));
                highlights.dedup_by(|(_, start_a, _), (_, start_b, _)| start_a == start_b);
                
                let mut words = Vec::new();
                let mut current_position = 0;
                for (code_type, start, end) in highlights {
                    if current_position > start { continue }
                    words.push(("".to_string(), &line[current_position..start]));
                    words.push((code_type, &line[start..end]));
                    current_position = end;
                }
                words.push(("".to_string(), line.split_at(current_position).1));

                let words = words.into_iter().filter(|w| w.1.len() != 0).collect::<Vec<_>>();

                //create text for each highlight group on the line
                for (code_type, word) in words {
                    let color = get_highlight_for_code_type(&code_type, color_scheme);

                    let (bounds, characters) = 
                        text_box_factory.create(word ,(current_offset, y_position), 0.9, font_scale, color);
                    text_components.extend(characters);

                    current_offset = bounds.right;
                }
            } else {
                let (_, characters) = 
                    text_box_factory.create(line ,(current_offset, y_position), 0.9, font_scale, color_scheme.text_color);
                text_components.extend(characters);
            }        
        }   

        text_components
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

const QUERY_ADDITION: &'static str = 
r#"
"=>" @punctuation.delimiter
"->" @punctuation.delimeter
"|" @operator
"=" @operator
"+" @operator
"-" @operator
"#;

fn query_highlight(node: Node, text: &str) -> Vec<(usize, (String, usize, usize))> {
    let custom_highlight = format!("{}{}", QUERY_ADDITION, tree_sitter_rust::HIGHLIGHT_QUERY);
    
    let query = tree_sitter::Query::new(tree_sitter_rust::language(), &custom_highlight).unwrap();

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