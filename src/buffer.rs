use std::collections::HashMap;

use legion::World;
use tree_sitter::{Node, QueryCursor};

use crate::{text::TextBoxFactory, ui_box::{UiBoxFactory, hex_color}};

fn query_highlight(node: Node, text: &str) -> Vec<(usize, (String, usize, usize))> {
    let custom_highlight = format!(
r#"
{}
"=>" @punctuation.delimiter
"->" @punctuation.delimeter
"|" @operator
"=" @operator
"+" @operator
"-" @operator
"#, tree_sitter_rust::HIGHLIGHT_QUERY);
    
    
    let query = tree_sitter::Query::new(tree_sitter_rust::language(), &custom_highlight).unwrap();

    QueryCursor::new().matches(&query, node, text.as_bytes()).flat_map(|cap| {
        let mut nodes = Vec::new();

        for c in cap.captures {
            nodes.push((c.node.start_position().row, 
                            (
                                query.capture_names()[c.index as usize].clone(), 
                                c.node.start_position().column, 
                                c.node.end_position().column
                            )
            ));
        }

        nodes
    }).collect::<Vec<_>>()
}

pub fn render_code(text: &str, world: &mut World, text_box_factory: &TextBoxFactory, _: &UiBoxFactory) {
    let text_color = hex_color("#F64740").unwrap();
    let keyword_color = hex_color("#7a28cb").unwrap();
    let type_color = hex_color("#ffd952").unwrap();
    let function_color = hex_color("#166088").unwrap();
    let string_color = hex_color("#4AAD52").unwrap(); //819E57, 4AAD52
    let primitive_color = hex_color("#DC2E3F").unwrap();
    let property_color = hex_color("#F489FA").unwrap();
    let operator_color = hex_color("#FFFFFF").unwrap(); //E26628
    let comment_color = hex_color("#676779").unwrap();
    let punctuation_color = hex_color("#FFFFFF").unwrap();
    let line_number_color = hex_color("#FFFFFF").unwrap();


    let mut parser = tree_sitter::Parser::new();
    parser.set_language(tree_sitter_rust::language()).unwrap();
    let tree = parser.parse(text, None).unwrap();

    let mut highlights_by_row: HashMap<usize, Vec<(String, usize, usize)>> = HashMap::new();

    for (row, highlight) in query_highlight(tree.root_node(), text).into_iter() {

        let v = highlights_by_row.entry(row)
            .or_insert(vec![]);

        v.push(highlight);
    }

    let line_height = 50f32;
    let mut text_components = Vec::new();

    for (i, line) in text.split('\n').enumerate() {
        let line_num = i;

        let y_position = 1200f32 - ((i + 1) as f32 * line_height);

        let line_num_str = format!("{}", line_num+1);
        let line_num_str = format!("{:indent$}", line_num_str, indent=(8-line_num_str.len()));

        let (bounds, characters) = 
                text_box_factory.create( &line_num_str, (10f32, y_position), 0.9, 0.75f32, line_number_color);
            text_components.extend(characters);

        let mut current_offset = bounds.right;

        if line.len() == 0 {
            continue
        }

        if let Some(mut highlights) = highlights_by_row.get(&line_num).cloned() {
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

            for (code_type, word) in words {
                let color = match code_type.as_str() {
                    "keyword" | "escape" => keyword_color,
                    "type" | "type.builtin" | "constructor" => type_color,
                    "function" | "function.method" | "function.macro" => function_color,
                    "string" => string_color,
                    "constant.builtin" => primitive_color,
                    "property" => property_color,
                    "operator" => operator_color,
                    "comment" => comment_color,
                    a if a.contains("punctuation") => punctuation_color,
                    _ => text_color,
                };

                let (bounds, characters) = 
                    text_box_factory.create(word ,(current_offset, y_position), 0.9, 0.75f32, color);
                text_components.extend(characters);

                current_offset = bounds.right;
            }
        } else {
            let (_, characters) = 
                text_box_factory.create(line ,(current_offset, y_position), 0.9, 0.75f32, text_color);
            text_components.extend(characters);
        }        
    }   

    world.extend(text_components);
}