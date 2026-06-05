use crate::{
    common::{object::Obj, Chunk, OpCode, Value},
    parser::Parser,
    precedence::Precedence,
    scanner::Scanner,
    table::Table,
    token::{
        Token,
        TokenType::{self},
    },
    vm::Vm,
};

type ParseFn = fn(&mut Compiler, &mut Vm, &mut Chunk, bool);

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
        prefix: Some(Compiler::variable),
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

const U8_COUNT: usize = u8::MAX as usize + 1;

#[derive(Debug, Clone, Copy)]
struct Local {
    name: Token,
    depth: Option<usize>,
}

impl Local {
    fn empty() -> Self {
        Self {
            name: Token::empty(),
            depth: None,
        }
    }
}

pub struct Compiler {
    parser: Parser,
    scanner: Option<Scanner>,
    vars_cache: Table,

    locals: [Local; U8_COUNT],
    local_count: usize,
    scope_depth: usize,
}

impl Compiler {
    pub fn new() -> Self {
        Self {
            parser: Parser::new(),
            scanner: None,
            vars_cache: Table::new(),

            local_count: 0,
            scope_depth: 0,
            locals: [Local::empty(); U8_COUNT],
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

    fn emit_constant(&mut self, chunk: &mut Chunk, value: Value) -> usize {
        let line = self
            .scanner
            .as_ref()
            .expect("Scanner should be initialized at this point")
            .line();
        chunk.write_constant(value, line)
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

    fn emit_jump(&mut self, chunk: &mut Chunk, instruction: u8) -> usize {
        self.emit_byte(chunk, instruction);
        self.emit_byte(chunk, 0xff);
        self.emit_byte(chunk, 0xff);

        chunk.count() - 2
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

        let can_assign = precedence <= Precedence::ASSIGNMENT;
        prefix_rule(self, vm, chunk, can_assign);

        while precedence <= get_rule(self.parser.current.typ).precedence {
            self.advance();

            let Some(infix_rule) = get_rule(self.parser.previous.typ).infix else {
                self.error("Expect expression.");
                return;
            };

            infix_rule(self, vm, chunk, can_assign);
        }

        if can_assign && self.matches(TokenType::EQUAL) {
            self.error("Invalid assignment target.");
        }
    }

    fn expression(&mut self, vm: &mut Vm, chunk: &mut Chunk) {
        self.parse_precedence(vm, chunk, Precedence::ASSIGNMENT);
    }

    fn binary(&mut self, vm: &mut Vm, chunk: &mut Chunk, can_assign: bool) {
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

    fn unary(&mut self, vm: &mut Vm, chunk: &mut Chunk, can_assign: bool) {
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

    fn grouping(&mut self, vm: &mut Vm, chunk: &mut Chunk, can_assign: bool) {
        self.expression(vm, chunk);
        self.consume(TokenType::RIGHT_PAREN, "Expect ')' after expression");
    }

    fn literal(&mut self, _vm: &mut Vm, chunk: &mut Chunk, can_assign: bool) {
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

    fn string(&mut self, vm: &mut Vm, chunk: &mut Chunk, can_assign: bool) {
        let chars = unsafe { self.parser.previous.start.add(1) };
        let length = self.parser.previous.length - 2;

        let obj_string = vm.copy_string(chars, length);
        let value = Value::Obj(obj_string as *mut Obj);
        self.emit_constant(chunk, value);
    }

    fn number(&mut self, _vm: &mut Vm, chunk: &mut Chunk, can_assign: bool) {
        let num = self.parser.previous.as_f64().unwrap_or_else(|| {
            panic!("Invalid number at line {}", self.parser.previous.line);
        });

        self.emit_constant(chunk, Value::Number(num));
    }

    fn variable(&mut self, vm: &mut Vm, chunk: &mut Chunk, can_assign: bool) {
        self.named_variable(vm, chunk, self.parser.previous, can_assign);
    }

    fn resolve_local(&mut self, name: Token) -> Option<usize> {
        for i in (0..self.local_count).rev() {
            let local = self.locals[i];
            if Self::identifiers_equal(name, local.name) {
                if local.depth.is_none() {
                    self.error("Can't read local variable in its own initializer.");
                    return None;
                }

                return Some(i);
            }
        }

        None
    }

    fn named_variable(&mut self, vm: &mut Vm, chunk: &mut Chunk, name: Token, can_assign: bool) {
        let (get_op, set_op, arg) = match self.resolve_local(name) {
            Some(arg) => (OpCode::OP_GET_LOCAL, OpCode::OP_SET_LOCAL, arg),
            None => {
                let arg = self.identifier_constant(vm, chunk, name);
                (OpCode::OP_GET_GLOBAL, OpCode::OP_SET_GLOBAL, arg)
            }
        };

        if can_assign && self.matches(TokenType::EQUAL) {
            self.expression(vm, chunk);
            self.emit_bytes(chunk, set_op as u8, arg as u8);
        } else {
            self.emit_bytes(chunk, get_op as u8, arg as u8);
        }
    }

    fn matches(&mut self, typ: TokenType) -> bool {
        if !self.check(typ) {
            return false;
        }

        self.advance();

        true
    }

    fn check(&self, typ: TokenType) -> bool {
        self.parser.current.typ == typ
    }

    fn declaration(&mut self, vm: &mut Vm, chunk: &mut Chunk) {
        if self.matches(TokenType::VAR) {
            self.var_declaration(vm, chunk);
        } else {
            self.statement(vm, chunk);
        }

        if self.parser.panic_mode {
            self.synchronize(vm, chunk);
        }
    }

    fn var_declaration(&mut self, vm: &mut Vm, chunk: &mut Chunk) {
        let global = self.parse_variable(vm, chunk, "Expect variable name.");

        if self.matches(TokenType::EQUAL) {
            self.expression(vm, chunk);
        } else {
            self.emit_byte(chunk, OpCode::OP_NIL as u8);
        }

        self.consume(
            TokenType::SEMICOLON,
            "Expect ';' after variable declaration.",
        );

        self.define_variable(chunk, global);
    }

    fn mark_initialized(&mut self) {
        self.locals[self.local_count - 1].depth = Some(self.scope_depth);
    }

    fn define_variable(&mut self, chunk: &mut Chunk, global: usize) {
        if self.scope_depth > 0 {
            self.mark_initialized();
            return;
        }

        self.emit_bytes(chunk, OpCode::OP_DEFINE_GLOBAL as u8, global as u8);
    }

    fn parse_variable(
        &mut self,
        vm: &mut Vm,
        chunk: &mut Chunk,
        error_message: &'static str,
    ) -> usize {
        self.consume(TokenType::IDENTIFIER, error_message);

        self.declare_variable();
        if self.scope_depth > 0 {
            return 0;
        }

        self.identifier_constant(vm, chunk, self.parser.previous)
    }

    fn declare_variable(&mut self) {
        if self.scope_depth == 0 {
            return;
        }

        let name = self.parser.previous;

        for i in (0..self.local_count).rev() {
            let local = self.locals[i];

            if let Some(local_depth) = local.depth {
                if local_depth < self.scope_depth {
                    break;
                }
            }

            if Self::identifiers_equal(name, local.name) {
                self.error("Already a variable with this name in this scope.");
            }
        }

        self.add_local(name);
    }

    fn identifiers_equal(a: Token, b: Token) -> bool {
        if a.length != b.length {
            return false;
        }

        // TODO replace to memcmp, like  std::ptr::copy_nonoverlapping(a.start, b.start, a.length) == 0
        let a_slice = unsafe { std::slice::from_raw_parts(a.start, a.length) };
        let b_slice = unsafe { std::slice::from_raw_parts(b.start, b.length) };

        a_slice == b_slice
    }

    fn add_local(&mut self, name: Token) {
        if self.local_count == U8_COUNT {
            self.error("Too many variables in function");
            return;
        }

        self.locals[self.local_count] = Local { name, depth: None };
        self.local_count += 1;
    }

    fn identifier_constant(&mut self, vm: &mut Vm, chunk: &mut Chunk, name: Token) -> usize {
        let obj_string = vm.copy_string(name.start, name.length);

        // Check if exists in cache
        let mut index = Value::Nil;
        if self.vars_cache.get(obj_string, &mut index) {
            tracing::debug!("Existing constant identifier");

            if let Some(index_num) = index.as_index() {
                return index_num;
            }
        }

        tracing::debug!("Not existing constant identifier");

        // Создаем новую константу
        let value = Value::Obj(obj_string as *mut Obj);
        let idx = chunk.add_constant(value);

        // Сохраняем в кэш
        self.vars_cache.set(obj_string, Value::Index(idx));

        idx
    }

    fn synchronize(&mut self, vm: &mut Vm, chunk: &mut Chunk) {
        self.parser.panic_mode = false;

        while self.parser.current.typ != TokenType::EOF {
            if self.parser.previous.typ == TokenType::SEMICOLON {
                return;
            }

            match self.parser.current.typ {
                TokenType::CLASS
                | TokenType::FUN
                | TokenType::VAR
                | TokenType::FOR
                | TokenType::IF
                | TokenType::WHILE
                | TokenType::PRINT
                | TokenType::RETURN => {
                    return;
                }

                _ => {
                    // Nothing
                }
            }

            self.advance();
        }
    }

    fn statement(&mut self, vm: &mut Vm, chunk: &mut Chunk) {
        if self.matches(TokenType::PRINT) {
            self.print_statement(vm, chunk);
        } else if self.matches(TokenType::IF) {
            self.if_statement(vm, chunk);
        } else if self.matches(TokenType::LEFT_BRACE) {
            self.begin_scope(vm, chunk);
            self.block(vm, chunk);
            self.end_scope(vm, chunk);
        } else {
            self.expression_statement(vm, chunk);
        }
    }

    fn if_statement(&mut self, vm: &mut Vm, chunk: &mut Chunk) {
        self.consume(TokenType::LEFT_PAREN, "Expect '(' after if.");
        self.expression(vm, chunk);
        self.consume(TokenType::RIGHT_PAREN, "Expect ')' after condition.");

        let then_jump = self.emit_jump(chunk, OpCode::OP_JUMP_IF_FALSE as u8);
        self.emit_byte(chunk, OpCode::OP_POP as u8);
        self.statement(vm, chunk);

        let else_jump = self.emit_jump(chunk, OpCode::OP_JUMP as u8);

        self.patch_jump(chunk, then_jump);
        self.emit_byte(chunk, OpCode::OP_POP as u8);

        if self.matches(TokenType::ELSE) {
            self.statement(vm, chunk);
        }

        self.patch_jump(chunk, else_jump);
    }

    fn patch_jump(&mut self, chunk: &mut Chunk, offset: usize) {
        // -2 to adjust for the bytecode for the jump offset itself.
        let jump = chunk.count() - offset - 2;

        if jump > u16::MAX as usize {
            self.error("Too much code to jump over");
        }

        chunk.code.set(offset, ((jump >> 8) & 0xff) as u8);
        chunk.code.set(offset + 1, (jump & 0xff) as u8);
    }

    fn begin_scope(&mut self, vm: &mut Vm, chunk: &mut Chunk) {
        self.scope_depth += 1;
    }

    fn end_scope(&mut self, vm: &mut Vm, chunk: &mut Chunk) {
        self.scope_depth -= 1;

        while self.local_count > 0 {
            let local = self.locals[self.local_count - 1];
            if local.depth.unwrap_or(0) > self.scope_depth {
                self.emit_byte(chunk, OpCode::OP_POP as u8);
                self.local_count -= 1;
            } else {
                break;
            }
        }
    }

    fn block(&mut self, vm: &mut Vm, chunk: &mut Chunk) {
        while !self.check(TokenType::RIGHT_BRACE) && !self.check(TokenType::EOF) {
            self.declaration(vm, chunk);
        }

        self.consume(TokenType::RIGHT_BRACE, "Expect '}' after block.");
    }

    fn expression_statement(&mut self, vm: &mut Vm, chunk: &mut Chunk) {
        self.expression(vm, chunk);
        self.consume(TokenType::SEMICOLON, "Expect ';' after expression");
        self.emit_byte(chunk, OpCode::OP_POP as u8);
    }

    fn print_statement(&mut self, vm: &mut Vm, chunk: &mut Chunk) {
        self.expression(vm, chunk);
        self.consume(TokenType::SEMICOLON, "Expect ';' after value");
        self.emit_byte(chunk, OpCode::OP_PRINT as u8);
    }

    pub fn compile(&mut self, vm: &mut Vm, source: *const u8, chunk: &mut Chunk) -> bool {
        self.scanner = Some(Scanner::new(source));

        self.parser.had_error = false;
        self.parser.panic_mode = false;

        self.advance();

        while !self.matches(TokenType::EOF) {
            self.declaration(vm, chunk);
        }

        self.end(chunk);

        !self.parser.had_error
    }
}
