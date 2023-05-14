use std::io::Write;
use std::{fs::File, io::Read, path::Path};

use std::str;

use regex::Regex;
use simple_error::SimpleError;
use tree_sitter_highlight::{HighlightConfiguration, Highlighter, HighlightEvent};

use crate::buffer_system::Cursor;
use crate::colorscheme::RUST_HIGHLIGHT_NAMES;

#[derive(Debug)]
pub struct Highlight {
    pub(super) code_type: Option<usize>,
    pub(super) start_byte: usize,
    pub(super) end_byte: usize,
}

pub struct HighlightedRange {
    pub(super) start: (usize, usize),
    pub(super) end: (usize, usize),
}

pub struct Buffer {
    file: String,

    lines: Vec<String>,

    pub(super) cursors: Vec<Cursor>,
    pub(super) highlighted_ranges: Vec<HighlightedRange>,


    rust_highlight_configuration: HighlightConfiguration,
    highlighter: Highlighter,

    pub(super) highlight_enabled: bool,
    pub(super) highlights: Vec<Highlight>,
}

impl Buffer {
    pub fn lines(&self) -> &[String] {
        &self.lines
    }

    pub fn insert_cursor(&mut self) {
        self.cursors.push(
            Cursor(0, 0)
        );
    }

    pub fn insert_highlighted_range(&mut self, start: (usize, usize), end: (usize, usize)) {
        self.highlighted_ranges.push(
            HighlightedRange {
                start, 
                end 
            }
        );
    }

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

        let mut rust_highlight_configuration = HighlightConfiguration::new(
                tree_sitter_rust::language(),
                tree_sitter_rust::HIGHLIGHT_QUERY,
                "",
                "",
        ).unwrap();

        rust_highlight_configuration.configure(RUST_HIGHLIGHT_NAMES);

        let highlighter = Highlighter::new();

        let mut buffer = Self {
            file: file_name.to_string(),

            lines,

            cursors: Vec::new(),
            highlighted_ranges: Vec::new(),

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

    pub fn remove_at(&mut self, cursor: Cursor) {
        let row = cursor.0;
        let mut col = cursor.1;

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

    pub fn insert_at(&mut self, character: char, cursor: Cursor) {
        let row = cursor.0;
        let mut col = cursor.1;

        col = col.clamp(0, self.lines[row].len());
        
        if let Some(line) = self.lines.get_mut(row) {
            line.insert(col, character);
        }

        if self.highlight_enabled { self.update_highlights(); }
    }

    pub fn insert_str_at(&mut self, str: &str, cursor: Cursor) -> Cursor {
        let row = cursor.0;
        let mut col = cursor.1;

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
                self.insert_line(Cursor(current_row, 0))
            }
        }
        let end_column = self.lines[current_row].len();
        self.lines[current_row] += &following_text;

        if self.highlight_enabled { self.update_highlights(); }

        Cursor(current_row, end_column)
    }

    pub fn insert_line(&mut self, cursor: Cursor) {
        let row = cursor.0;
        let mut col = cursor.1;

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

    pub fn move_right(&self, cursor: Cursor) -> Cursor {
        let row = cursor.0;
        let col = cursor.1;

        let current_line = self.lines.get(row).unwrap();

        if col < current_line.len() {
            Cursor(row, col + 1)
        } else if row < self.lines.len() - 1 {
            Cursor(row + 1, 0)
        } else {
            Cursor(row, col)
        }
    }

    pub fn move_left(&self, cursor: Cursor) -> Cursor {
        let row = cursor.0;
        let mut col = cursor.1;

        let current_line = self.lines.get(row).unwrap();

        col = col.clamp(0, current_line.len());

        if col > 0 {
            Cursor(row, col - 1)
        } else if row > 0 {
            let previous_row = &self.lines[row - 1];
            Cursor(row - 1, previous_row.len())
        } else {
            Cursor(row, col)
        }
    }

    pub fn move_up(&self, cursor: Cursor) -> Cursor {
        let row = cursor.0;
        let col = cursor.1;

        if row > 0 {
            Cursor(row - 1, col)
        } else {
            Cursor(row, col)
        }
    }

    pub fn move_down(&self, cursor: Cursor) -> Cursor {
        let row = cursor.0;
        let col = cursor.1;

        if row < self.lines.len() - 1 {
            Cursor(row + 1, col)
        } else {
            Cursor(row, col)
        }
    }

    pub fn move_forward_word(&self, cursor: Cursor) -> Cursor {
        let row = cursor.0;
        let col = cursor.1;

        let line_bounday_regex = Regex::new(r"(\b|$)").unwrap();

        let line_text = &self.lines[row];
        
        if let Some(m) = line_bounday_regex.find_iter(line_text).find(|m| m.start() > col) {
            return Cursor(row, m.start());
        }
        if let Some(next_line) = self.lines.get(row + 1) {
            if next_line.is_empty() {
                let next_line_match = line_bounday_regex
                    .find(next_line)
                    .map(|m| m.start())
                    .unwrap_or(0);
                Cursor(row + 1, next_line_match)
            } else {
                Cursor(row + 1, 0)
            }
            
        } else {
            Cursor(row, col)
        } 
    }

    pub fn move_backward_word(&self, cursor: Cursor) -> Cursor {
        let row = cursor.0;
        let col = cursor.1;

        let line_bounday_regex = Regex::new(r"(\b|$|^)").unwrap();

        let line_text = &self.lines[row];
        
        let mut matches = line_bounday_regex.find_iter(line_text).peekable();
        
        //if the first match is greater than the c
        if col == 0 {
            if row > 0 {
                return Cursor(row - 1, self.lines[row - 1].len())
            } else {
                return Cursor(row, col)
            }
        }

        while let Some(m) = matches.next() {
            if matches.peek().map(|m| m.start() >= col).unwrap_or(false) {
                return Cursor(row, m.start());
            }
        }

        Cursor(row, col)
    }
}
