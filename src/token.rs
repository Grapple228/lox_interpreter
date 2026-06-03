use crate::define_enum_with_values;

#[derive(Debug, Clone, Copy)]
pub struct Token {
    pub typ: TokenType,
    pub start: *const u8,
    pub length: usize,
    pub line: usize,
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
        // One or two character tokens.
        BANG = 11 => "BANG",
        BANG_EQUAL = 12 => "BANG_EQUAL",
        EQUAL = 13 => "EQUAL",
        EQUAL_EQUAL = 14 => "EQUAL_EQUAL",
        GREATER = 15 => "GREATER",
        GREATER_EQUAL = 16 => "GREATER_EQUAL",
        LESS = 17 => "LESS",
        LESS_EQUAL = 18 => "LESS_EQUAL",
        // Literals.
        IDENTIFIER = 19 => "IDENTIFIER",
        STRING = 20 => "STRING",
        NUMBER = 21 => "NUMBER",
        // Keywords.
        AND = 22 => "AND",
        CLASS = 23 => "CLASS",
        ELSE = 24 => "ELSE",
        FALSE = 25 => "FALSE",
        FOR = 26 => "FOR",
        FUN = 27 => "FUN",
        IF = 28 => "IF",
        NIL = 29 => "NIL",
        OR = 30 => "OR",
        PRINT = 31 => "PRINT",
        RETURN = 32 => "RETURN",
        SUPER = 33 => "SUPER",
        THIS = 34 => "THIS",
        TRUE = 35 => "TRUE",
        VAR = 36 => "VAR",
        WHILE = 37 => "WHILE",
        // Special tokens.
        ERROR = 38 => "ERROR",
        EOF = 39 => "EOF",
    }
}
