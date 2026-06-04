use crate::{
    common::{object::Obj, Chunk, OpCode, Value},
    parser::Parser,
    precedence::Precedence,
    scanner::Scanner,
    token::{
        Token,
        TokenType::{self},
    },
    vm::Vm,
};

type ParseFn = fn(&mut Compiler, &mut Vm, &mut Chunk);

pub struct ParseRule {
    prefix: Option<ParseFn>,
    infix: Option<ParseFn>,
    precedence: Precedence,
}

const RULES: [ParseRule; 40] = [
    // TokenType индексы должны соответствовать порядку в enum
    ParseRule {
        prefix: Some(Compiler::grouping),
        infix: None,
        precedence: Precedence::NONE,
    }, // LEFT_PAREN
    ParseRule {
        prefix: None,
        infix: None,
        precedence: Precedence::NONE,
    }, // RIGHT_PAREN
    ParseRule {
        prefix: None,
        infix: None,
        precedence: Precedence::NONE,
    }, // LEFT_BRACE
    ParseRule {
        prefix: None,
        infix: None,
        precedence: Precedence::NONE,
    }, // RIGHT_BRACE
    ParseRule {
        prefix: None,
        infix: None,
        precedence: Precedence::NONE,
    }, // COMMA
    ParseRule {
        prefix: None,
        infix: None,
        precedence: Precedence::NONE,
    }, // DOT
    ParseRule {
        prefix: Some(Compiler::unary),
        infix: Some(Compiler::binary),
        precedence: Precedence::TERM,
    }, // MINUS
    ParseRule {
        prefix: None,
        infix: Some(Compiler::binary),
        precedence: Precedence::TERM,
    }, // PLUS
    ParseRule {
        prefix: None,
        infix: None,
        precedence: Precedence::NONE,
    }, // SEMICOLON
    ParseRule {
        prefix: None,
        infix: Some(Compiler::binary),
        precedence: Precedence::FACTOR,
    }, // SLASH
    ParseRule {
        prefix: None,
        infix: Some(Compiler::binary),
        precedence: Precedence::FACTOR,
    }, // STAR
    ParseRule {
        prefix: Some(Compiler::unary),
        infix: None,
        precedence: Precedence::NONE,
    }, // BANG
    ParseRule {
        prefix: None,
        infix: Some(Compiler::binary),
        precedence: Precedence::EQUALITY,
    }, // BANG_EQUAL
    ParseRule {
        prefix: None,
        infix: None,
        precedence: Precedence::NONE,
    }, // EQUAL
    ParseRule {
        prefix: None,
        infix: Some(Compiler::binary),
        precedence: Precedence::EQUALITY,
    }, // EQUAL_EQUAL
    ParseRule {
        prefix: None,
        infix: Some(Compiler::binary),
        precedence: Precedence::COMPARISON,
    }, // GREATER
    ParseRule {
        prefix: None,
        infix: Some(Compiler::binary),
        precedence: Precedence::COMPARISON,
    }, // GREATER_EQUAL
    ParseRule {
        prefix: None,
        infix: Some(Compiler::binary),
        precedence: Precedence::COMPARISON,
    }, // LESS
    ParseRule {
        prefix: None,
        infix: Some(Compiler::binary),
        precedence: Precedence::COMPARISON,
    }, // LESS_EQUAL
    ParseRule {
        prefix: None,
        infix: None,
        precedence: Precedence::NONE,
    }, // IDENTIFIER
    ParseRule {
        prefix: Some(Compiler::string),
        infix: None,
        precedence: Precedence::NONE,
    }, // STRING
    ParseRule {
        prefix: Some(Compiler::number),
        infix: None,
        precedence: Precedence::NONE,
    }, // NUMBER
    ParseRule {
        prefix: None,
        infix: None,
        precedence: Precedence::NONE,
    }, // AND
    ParseRule {
        prefix: None,
        infix: None,
        precedence: Precedence::NONE,
    }, // CLASS
    ParseRule {
        prefix: None,
        infix: None,
        precedence: Precedence::NONE,
    }, // ELSE
    ParseRule {
        prefix: Some(Compiler::literal),
        infix: None,
        precedence: Precedence::NONE,
    }, // FALSE
    ParseRule {
        prefix: None,
        infix: None,
        precedence: Precedence::NONE,
    }, // FOR
    ParseRule {
        prefix: None,
        infix: None,
        precedence: Precedence::NONE,
    }, // FUN
    ParseRule {
        prefix: None,
        infix: None,
        precedence: Precedence::NONE,
    }, // IF
    ParseRule {
        prefix: Some(Compiler::literal),
        infix: None,
        precedence: Precedence::NONE,
    }, // NIL
    ParseRule {
        prefix: None,
        infix: None,
        precedence: Precedence::NONE,
    }, // OR
    ParseRule {
        prefix: None,
        infix: None,
        precedence: Precedence::NONE,
    }, // PRINT
    ParseRule {
        prefix: None,
        infix: None,
        precedence: Precedence::NONE,
    }, // RETURN
    ParseRule {
        prefix: None,
        infix: None,
        precedence: Precedence::NONE,
    }, // SUPER
    ParseRule {
        prefix: None,
        infix: None,
        precedence: Precedence::NONE,
    }, // THIS
    ParseRule {
        prefix: Some(Compiler::literal),
        infix: None,
        precedence: Precedence::NONE,
    }, // TRUE
    ParseRule {
        prefix: None,
        infix: None,
        precedence: Precedence::NONE,
    }, // VAR
    ParseRule {
        prefix: None,
        infix: None,
        precedence: Precedence::NONE,
    }, // WHILE
    ParseRule {
        prefix: None,
        infix: None,
        precedence: Precedence::NONE,
    }, // ERROR
    ParseRule {
        prefix: None,
        infix: None,
        precedence: Precedence::NONE,
    }, // EOF
];

fn get_rule(typ: TokenType) -> &'static ParseRule {
    &RULES[typ as usize]
}

pub struct Compiler {
    parser: Parser,
    scanner: Option<Scanner>,
}

impl Compiler {
    pub fn new() -> Self {
        Self {
            parser: Parser::new(),
            scanner: None,
        }
    }

    fn error_at(&mut self, token: Token, message: &'static str) {
        if self.parser.panic_mode {
            return;
        }
        self.parser.panic_mode = true;

        eprint!("[line {}] Error", token.line);

        match token.typ {
            TokenType::EOF => {
                eprint!(" at end");
            }
            TokenType::ERROR => {
                // Nothing
            }
            _ => {
                let lexeme = unsafe {
                    std::str::from_utf8_unchecked(std::slice::from_raw_parts(
                        token.start,
                        token.length,
                    ))
                };

                eprint!(" at '{}'", lexeme);
            }
        }

        eprint!(": {}\n", message);
        self.parser.had_error = true;
    }

    fn error(&mut self, message: &'static str) {
        self.error_at(self.parser.previous, message);
    }

    fn error_at_current(&mut self, message: &'static str) {
        self.error_at(self.parser.current, message);
    }

    fn consume(&mut self, typ: TokenType, message: &'static str) {
        if self.parser.current.typ == typ {
            self.advance();
            return;
        }

        self.error_at_current(message);
    }

    pub fn advance(&mut self) {
        self.parser.previous = self.parser.current;

        loop {
            let scanner = self
                .scanner
                .as_mut()
                .expect("Scanner should be initialized at this point");
            self.parser.current = scanner.scan_token();
            if self.parser.current.typ != TokenType::ERROR {
                break;
            }

            let message = unsafe {
                std::str::from_utf8_unchecked(std::slice::from_raw_parts(
                    self.parser.current.start,
                    self.parser.current.length,
                ))
            };

            self.error_at_current(message);
        }
    }

    fn emit_constant(&mut self, chunk: &mut Chunk, value: Value) {
        let line = self
            .scanner
            .as_ref()
            .expect("Scanner should be initialized at this point")
            .line();
        chunk.write_constant(value, line);
    }

    fn emit_byte(&mut self, chunk: &mut Chunk, byte: u8) {
        let line = self.parser.previous.line;
        chunk.write(byte, line);
    }

    fn emit_bytes(&mut self, chunk: &mut Chunk, byte1: u8, byte2: u8) {
        self.emit_byte(chunk, byte1);
        self.emit_byte(chunk, byte2);
    }

    fn emit_return(&mut self, chunk: &mut Chunk) {
        self.emit_byte(chunk, OpCode::OP_RETURN as u8);
    }

    fn end(&mut self, chunk: &mut Chunk) {
        self.emit_return(chunk);

        if cfg!(debug_assertions) {
            if !self.parser.had_error {
                chunk.disassemble("code");
            }
        }
    }

    fn parse_precedence(&mut self, vm: &mut Vm, chunk: &mut Chunk, precedence: Precedence) {
        self.advance();

        let Some(prefix_rule) = get_rule(self.parser.previous.typ).prefix else {
            self.error("Expect expression.");
            return;
        };

        prefix_rule(self, vm, chunk);

        while precedence <= get_rule(self.parser.current.typ).precedence {
            self.advance();

            let Some(infix_rule) = get_rule(self.parser.previous.typ).infix else {
                self.error("Expect expression.");
                return;
            };

            infix_rule(self, vm, chunk);
        }
    }

    fn expression(&mut self, vm: &mut Vm, chunk: &mut Chunk) {
        self.parse_precedence(vm, chunk, Precedence::ASSIGNMENT);
    }

    fn binary(&mut self, vm: &mut Vm, chunk: &mut Chunk) {
        let operator_type = self.parser.previous.typ;
        let rule = get_rule(operator_type);
        self.parse_precedence(vm, chunk, rule.precedence.next());

        match operator_type {
            // BASIC OPS
            TokenType::PLUS => self.emit_byte(chunk, OpCode::OP_ADD as u8),
            TokenType::MINUS => self.emit_byte(chunk, OpCode::OP_SUBSTRACT as u8),
            TokenType::STAR => self.emit_byte(chunk, OpCode::OP_MULTIPLY as u8),
            TokenType::SLASH => self.emit_byte(chunk, OpCode::OP_DIVIDE as u8),

            // EQUALITY
            TokenType::BANG_EQUAL => {
                self.emit_bytes(chunk, OpCode::OP_EQUAL as u8, OpCode::OP_NOT as u8)
            }
            TokenType::EQUAL_EQUAL => self.emit_byte(chunk, OpCode::OP_EQUAL as u8),
            TokenType::GREATER => self.emit_byte(chunk, OpCode::OP_GREATER as u8),
            TokenType::GREATER_EQUAL => {
                self.emit_bytes(chunk, OpCode::OP_LESS as u8, OpCode::OP_NOT as u8)
            }
            TokenType::LESS => self.emit_byte(chunk, OpCode::OP_LESS as u8),
            TokenType::LESS_EQUAL => {
                self.emit_bytes(chunk, OpCode::OP_GREATER as u8, OpCode::OP_NOT as u8)
            }

            _ => unreachable!("Should not reach here"),
        }
    }

    fn unary(&mut self, vm: &mut Vm, chunk: &mut Chunk) {
        let operator_type = self.parser.previous.typ;

        // Compile the operand.
        self.parse_precedence(vm, chunk, Precedence::UNARY);

        // Emit the operator instruction.
        match operator_type {
            TokenType::MINUS => {
                self.emit_byte(chunk, OpCode::OP_NEGATE as u8);
            }
            TokenType::BANG => {
                self.emit_byte(chunk, OpCode::OP_NOT as u8);
            }
            _ => unreachable!("Should not reach here"),
        }
    }

    fn grouping(&mut self, vm: &mut Vm, chunk: &mut Chunk) {
        self.expression(vm, chunk);
        self.consume(TokenType::RIGHT_PAREN, "Expect ')' after expression");
    }

    fn literal(&mut self, _vm: &mut Vm, chunk: &mut Chunk) {
        match self.parser.previous.typ {
            TokenType::FALSE => {
                self.emit_byte(chunk, OpCode::OP_FALSE as u8);
            }
            TokenType::TRUE => {
                self.emit_byte(chunk, OpCode::OP_TRUE as u8);
            }
            TokenType::NIL => {
                self.emit_byte(chunk, OpCode::OP_NIL as u8);
            }
            _ => unreachable!("Should not reach here"),
        }
    }

    fn string(&mut self, vm: &mut Vm, chunk: &mut Chunk) {
        let chars = unsafe { self.parser.previous.start.add(1) };
        let length = self.parser.previous.length - 2;

        let obj_string = vm.copy_string(chars, length);
        let value = Value::Obj(obj_string as *mut Obj);
        self.emit_constant(chunk, value);
    }

    fn number(&mut self, _vm: &mut Vm, chunk: &mut Chunk) {
        let num = self.parser.previous.as_f64().unwrap_or_else(|| {
            panic!("Invalid number at line {}", self.parser.previous.line);
        });

        self.emit_constant(chunk, Value::Number(num));
    }

    pub fn compile(&mut self, vm: &mut Vm, source: *const u8, chunk: &mut Chunk) -> bool {
        self.scanner = Some(Scanner::new(source));

        self.parser.had_error = false;
        self.parser.panic_mode = false;

        self.advance();
        self.expression(vm, chunk);
        self.consume(TokenType::EOF, "Expect end of expression");

        self.end(chunk);

        !self.parser.had_error
    }
}
