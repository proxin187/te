use serde_json::Value;
use console::Style;
use lib_lexin::{Lexer, Token, Section};

use std::path::Path;
use std::ffi::OsStr;
use std::fs;
use std::env;

#[derive(Clone, Debug)]
pub struct Colors {
    keywords: Style,
    types: Style,
    operators: Style,
    integers: Style,
    strings: Style,

    pub default: Style,

    pub bar: Style,
    pub mode: Style,
    pub line_numbers: Style,

    pub background: u8,
}

#[derive(Clone, Debug)]
pub struct Syntax {
    keywords: Vec<String>,
    types: Vec<String>,
    operators: Vec<String>,

    lexer: Lexer,

    pub colors: Colors,
    pub filetype: String,
}

fn remove_str_sym(string: String) -> String {
    let mut chars = string.chars();
    chars.next();
    chars.next_back();
    chars.as_str().to_string().replace("\\", "")
}

impl Syntax {
    fn value_to_section(name: &str, value_array: &Value) -> Result<Section, Box<dyn std::error::Error>> {
        let array = value_array.as_array().ok_or::<Box<dyn std::error::Error>>("failed to parse config".into())?;
        return if array.len() == 2 {
            Ok(Section::new(name, &remove_str_sym(array[0].to_string()), &remove_str_sym(array[1].to_string())))
        } else {
            Err("section need a start and end".into())
        }
    }

    fn value_to_vec(value_array: &Value) -> Vec<String> {
        let mut vec: Vec<String> = Vec::new();
        for value in value_array.as_array().unwrap_or(&Vec::new()) {
            vec.push(remove_str_sym(value.to_string()));
        }

        vec
    }

    fn vec_to_symbols(vector: &Vec<String>) -> Vec<(char, String)> {
        let mut symbols: Vec<(char, String)> = Vec::new();
        for symbol in vector {
            symbols.push((symbol.chars().next().unwrap(), String::new()));
        }

        symbols
    }

    pub fn new(filename: &str) -> Result<Syntax, Box<dyn std::error::Error>> {
        let mut syntax = Syntax {
            keywords: Vec::new(),
            types: Vec::new(),
            operators: Vec::new(),

            lexer: Lexer::new(
                &[],
                &[],
                &[],
                true,
            ),

            colors: Colors {
                keywords: Style::new(),
                types: Style::new(),
                operators: Style::new(),
                integers: Style::new(),
                strings: Style::new(),

                default: Style::new(),

                bar: Style::new(),
                mode: Style::new(),
                line_numbers: Style::new(),

                background: 0,
            },
            filetype: String::new(),
        };

        // LOAD SYNTAX
        let extension = Path::new(filename)
            .extension()
            .unwrap_or(&OsStr::new(""))
            .to_str()
            .unwrap_or("");

        let home = env::var("HOME")?;
        match extension {
            "rs" => {
                let config = fs::read_to_string(home.clone() + "/.config/te/rust.json")?;
                let json = serde_json::from_str::<Value>(&config)?;

                // keywords
                let keywords = Self::value_to_vec(&json["keywords"]);
                syntax.lexer.keywords = keywords.clone();
                syntax.keywords = keywords;

                // types
                let types = Self::value_to_vec(&json["types"]);
                syntax.lexer.keywords.extend(types.clone());
                syntax.types = types;

                // types
                syntax.lexer.symbols = Self::vec_to_symbols(&Self::value_to_vec(&json["symbols"]));
                syntax.lexer.symbols.push((' ', String::new()));

                // load string colors
                syntax.lexer.sections.push(Self::value_to_section("string", &json["strings"])?);

                syntax.operators = Self::value_to_vec(&json["operators"]);

                syntax.filetype = String::from("rust");
            },
            _ => {
                syntax.filetype = String::from("text");
            },
        }

        // LOAD COLORS
        let colors = fs::read_to_string(home + "/.config/te/colors.json")?;
        let colors_json = serde_json::from_str::<Value>(&colors)?;

        // include the colors
        syntax.colors.background = colors_json["bg"].as_u64().unwrap_or(0) as u8;

        // builtin
        syntax.colors.keywords = Self::fg_color(&colors_json, "keywords").bold().on_color256(syntax.colors.background);
        syntax.colors.types = Self::fg_color(&colors_json, "types").on_color256(syntax.colors.background);
        syntax.colors.operators = Self::fg_color(&colors_json, "operators").on_color256(syntax.colors.background);

        // literals
        syntax.colors.integers = Self::fg_color(&colors_json, "integers").on_color256(syntax.colors.background);
        syntax.colors.strings = Self::fg_color(&colors_json, "strings").on_color256(syntax.colors.background);

        // default color
        syntax.colors.default = Style::new().on_color256(syntax.colors.background);

        // line numbers
        syntax.colors.line_numbers = Self::fg_color(&colors_json, "line_numbers").on_color256(syntax.colors.background);

        // bar color
        syntax.colors.bar = Self::bg_color(&colors_json, "bar");

        // mode colors
        let mode_bg = colors_json["mode_bg"].as_u64().unwrap_or(0) as u8;
        let mode_fg = colors_json["mode_fg"].as_u64().unwrap_or(0) as u8;
        syntax.colors.mode = Style::new().color256(mode_fg).on_color256(mode_bg).bold();

        Ok(syntax)
    }

    fn fg_color(colors: &Value, name: &str) -> Style {
        Style::new().color256(colors[name].as_u64().unwrap_or(0) as u8)
    }

    fn bg_color(colors: &Value, name: &str) -> Style {
        Style::new().on_color256(colors[name].as_u64().unwrap_or(0) as u8)
    }

    fn colorize(&self, token: Token) -> String {
        let token_str = token.as_string();
        return if let Ok(content) = token.is_section("string") {
            self.colors.strings.apply_to(format!("\"{}\"", content)).to_string()
        } else if self.keywords.contains(&token_str) {
            format!("{}", self.colors.keywords.apply_to(token_str))
        } else if self.types.contains(&token_str) {
            format!("{}", self.colors.types.apply_to(token_str))
        } else if self.operators.contains(&token_str) {
            format!("{}", self.colors.operators.apply_to(token_str))
        } else if token.as_string().parse::<usize>().is_ok() {
            format!("{}", self.colors.integers.apply_to(token_str))
        } else {
            format!("{}", self.colors.default.apply_to(token_str))
        };
    }

    pub fn highlight(&mut self, buffer: String) -> Result<String, Box<dyn std::error::Error>> {
        let mut output = String::new();

        self.lexer.load_str(&(buffer + "  "));

        for token in self.lexer.tokenize()? {
            output = output + &self.colorize(token);
        }

        Ok(output)
    }

    fn token_length(&mut self, token: &Token) -> usize {
        let length = token.as_string().len();

        return if token.is_section("string").is_ok() {
            length + 2
        } else {
            length
        }
    }

    pub fn next_token(&mut self, buffer: String, x: usize) -> Result<usize, Box<dyn std::error::Error>> {
        let mut token_index = 0;

        self.lexer.load_str(&(buffer + "  "));

        for token in self.lexer.tokenize()? {
            if token_index > x {
                return Ok(token_index);
            }
            token_index += self.token_length(&token);
        }

        Ok(x)
    }

    pub fn previous_token(&mut self, buffer: String, x: usize) -> Result<usize, Box<dyn std::error::Error>> {
        let mut token_index = 0;

        self.lexer.load_str(&(buffer + "  "));

        for token in self.lexer.tokenize()? {
            token_index += self.token_length(&token);
            if token_index >= x {
                return Ok(token_index - self.token_length(&token));
            }
        }

        Ok(x)
    }
}


