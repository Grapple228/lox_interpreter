use crate::{scanner::Scanner, token::TokenType};

pub struct Compiler;

impl Compiler {
    pub fn new() -> Self {
        Self
    }

    pub fn compile(&mut self, source: *const u8) {
        let mut scanner = Scanner::new(source);
        let mut line = 0;

        loop {
            let token = scanner.scan_token();

            if cfg!(debug_assertions) {
                if token.line != line {
                    print!("{:04} ", token.line);
                    line = token.line;
                } else {
                    print!("   | ");
                }

                let lexeme = unsafe {
                    std::str::from_utf8_unchecked(std::slice::from_raw_parts(
                        token.start,
                        token.length,
                    ))
                };

                // token.typ как число (как в книге)
                println!("{:2} '{}'", token.typ as u8, lexeme);
            }

            if token.typ == TokenType::EOF {
                break;
            }
        }
    }
}
