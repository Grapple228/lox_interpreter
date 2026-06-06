#![allow(static_mut_refs)]

use crate::token::{Token, TokenType};

pub struct Parser {
    pub current: Token,
    pub previous: Token,
    pub had_error: bool,
    pub panic_mode: bool,
}

impl Parser {
    pub fn new() -> Self {
        Self {
            current: Token {
                typ: TokenType::EOF,
                start: std::ptr::null(),
                length: 0,
                line: 0,
            },
            previous: Token {
                typ: TokenType::EOF,
                start: std::ptr::null(),
                length: 0,
                line: 0,
            },
            had_error: false,
            panic_mode: false,
        }
    }
}

static mut PARSER: Parser = Parser {
    current: Token {
        typ: TokenType::EOF,
        start: std::ptr::null(),
        length: 0,
        line: 0,
    },
    previous: Token {
        typ: TokenType::EOF,
        start: std::ptr::null(),
        length: 0,
        line: 0,
    },
    had_error: false,
    panic_mode: false,
};

pub fn get_parser() -> &'static mut Parser {
    unsafe { &mut PARSER }
}

pub fn init_parser() {
    unsafe {
        PARSER = Parser::new();
    }
}
