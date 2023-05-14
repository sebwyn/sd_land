use regex::Regex;
use simple_error::SimpleError;

pub fn hex_color(color: &str) -> Result<[f32; 3], SimpleError> {
    let regex = Regex::new(r"#([0-9a-f]{2})([0-9a-f]{2})([0-9a-f]{2})").expect("Failed to compile regex");

    let color = color.to_ascii_lowercase();

    let captures = regex.captures(&color).unwrap();

    let r = captures.get(1)
        .ok_or(SimpleError::new("Failed to parse hex color!"))?
        .as_str();

    let g = captures.get(2)
        .ok_or(SimpleError::new("Failed to parse hex color!"))?
        .as_str();

    let b = captures.get(3)
        .ok_or(SimpleError::new("Failed to parse hex color!"))?
        .as_str();

    let r = u32::from_str_radix(r, 16).map_err(|_| SimpleError::new("hex_color: Failed to convert string to number"))? as f32;
    let g = u32::from_str_radix(g, 16).map_err(|_| SimpleError::new("hex_color: Failed to convert string to number"))? as f32;
    let b = u32::from_str_radix(b, 16).map_err(|_| SimpleError::new("hex_color: Failed to convert string to number"))? as f32;

    Ok([ r / 255f32, g / 255f32, b / 255f32 ])
}

pub struct ColorScheme {
    pub(super) text_color: [f32; 3],
    pub(super) keyword_color: [f32; 3],
    pub(super) type_color: [f32; 3],
    pub(super) function_color: [f32; 3],
    pub(super) string_color: [f32; 3],
    pub(super) primitive_color: [f32; 3],
    pub(super) property_color: [f32; 3],
    pub(super) operator_color: [f32; 3],
    pub(super) comment_color: [f32; 3],
    pub(super) punctuation_color: [f32; 3],
    pub(super) line_number_color: [f32; 3]
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

pub const RUST_HIGHLIGHT_NAMES: &[&str] = &[
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
