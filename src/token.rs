use crate::define_enum_with_values;

#[derive(Debug, Clone, Copy)]
pub struct Token {
    pub typ: TokenType,
    pub start: *const u8,
    pub length: usize,
    pub line: usize,
}

impl Token {
    pub fn empty() -> Self {
        Self {
            typ: TokenType::EOF,
            start: std::ptr::null(),
            length: 0,
            line: 0,
        }
    }
}

impl Token {
    pub fn as_f64(&self) -> Option<f64> {
        if self.typ != TokenType::NUMBER {
            return None;
        }

        let mut result = 0.0;
        let mut decimal_places = 0.0;
        let mut is_fraction = false;

        unsafe {
            for i in 0..self.length {
                let c = *self.start.add(i);
                match c {
                    b'0'..=b'9' => {
                        let digit = (c - b'0') as f64;
                        if is_fraction {
                            decimal_places += 1.0;
                            result += digit / (10.0_f64.powi(decimal_places as i32));
                        } else {
                            result = result * 10.0 + digit;
                        }
                    }
                    b'.' => is_fraction = true,
                    _ => return None,
                }
            }
        }

        Some(result)
    }
}

define_enum_with_values! {
    #[allow(non_camel_case_types)]
    pub enum TokenType {
        // Single-character tokens.
        LEFT_PAREN = 0 => "LEFT_PAREN",
        RIGHT_PAREN = 1 => "RIGHT_PAREN",
        LEFT_BRACE = 2 => "LEFT_BRACE",
        RIGHT_BRACE = 3 => "RIGHT_BRACE",
        COMMA = 4 => "COMMA",
        DOT = 5 => "DOT",
        MINUS = 6 => "MINUS",
        PLUS = 7 => "PLUS",
        SEMICOLON = 8 => "SEMICOLON",
        SLASH = 9 => "SLASH",
        STAR = 10 => "STAR",
        PERCENT = 11 => "PERCENT",
        // One or two character tokens.
        BANG = 12 => "BANG",
        BANG_EQUAL = 13 => "BANG_EQUAL",
        EQUAL = 14 => "EQUAL",
        EQUAL_EQUAL = 15 => "EQUAL_EQUAL",
        GREATER = 16 => "GREATER",
        GREATER_EQUAL = 17 => "GREATER_EQUAL",
        LESS = 18 => "LESS",
        LESS_EQUAL = 19 => "LESS_EQUAL",
        // Literals.
        IDENTIFIER = 20 => "IDENTIFIER",
        STRING = 21 => "STRING",
        NUMBER = 22 => "NUMBER",
        // Keywords.
        AND = 23 => "AND",
        CLASS = 24 => "CLASS",
        ELSE = 25 => "ELSE",
        FALSE = 26 => "FALSE",
        FOR = 27 => "FOR",
        FUN = 28 => "FUN",
        IF = 29 => "IF",
        NIL = 30 => "NIL",
        OR = 31 => "OR",
        PRINT = 32 => "PRINT",
        RETURN = 33 => "RETURN",
        SUPER = 34 => "SUPER",
        THIS = 35 => "THIS",
        TRUE = 36 => "TRUE",
        VAR = 37 => "VAR",
        WHILE = 38 => "WHILE",
        // Special tokens.
        ERROR = 39 => "ERROR",
        EOF = 40 => "EOF",

        // loops
        BREAK = 41 => "BREAK",
        CONTINUE = 42 => "CONTINUE",
    }
}
