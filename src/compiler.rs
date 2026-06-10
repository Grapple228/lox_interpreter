use crate::{
    common::{Chunk, DynamicArray, OpCode, Stack, Table, Value},
    object::{FunctionType, Obj, ObjClosure, ObjUpValue},
    parser::get_parser,
    precedence::Precedence,
    scanner::{init_scanner, scan_token, scanner_line},
    token::{
        Token,
        TokenType::{self},
    },
    vm::Vm,
    ObjString,
};

use crate::ObjFunction;

type ParseFn = fn(&mut Compiler, bool);

pub struct ParseRule {
    prefix: Option<ParseFn>,
    infix: Option<ParseFn>,
    precedence: Precedence,
}

const RULES: [ParseRule; 43] = [
    // TokenType индексы должны соответствовать порядку в enum
    ParseRule {
        prefix: Some(Compiler::grouping),
        infix: Some(Compiler::call),
        precedence: Precedence::CALL,
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
        infix: Some(Compiler::dot),
        precedence: Precedence::CALL,
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
        prefix: None,
        infix: Some(Compiler::binary),
        precedence: Precedence::FACTOR,
    }, // PERCENT
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
        infix: Some(Compiler::and),
        precedence: Precedence::AND,
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
        infix: Some(Compiler::or),
        precedence: Precedence::OR,
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
    ParseRule {
        prefix: None,
        infix: None,
        precedence: Precedence::NONE,
    }, // BREAK
    ParseRule {
        prefix: None,
        infix: None,
        precedence: Precedence::NONE,
    }, // CONTINUE
];

fn get_rule(typ: TokenType) -> &'static ParseRule {
    &RULES[typ as usize]
}

const U8_COUNT: usize = u8::MAX as usize + 1;

#[derive(Debug, Clone, Copy)]
struct Local {
    name: Token,
    depth: Option<usize>,
    is_captured: bool,
}

impl Local {
    fn empty() -> Self {
        Self {
            name: Token::empty(),
            depth: None,
            is_captured: false,
        }
    }
}

struct LoopScope {
    start: usize,
    exit_jump: Option<usize>,
    scope_depth: usize,
    has_increment: bool,
}

pub enum Callee {
    Closure(*mut ObjClosure),
    Function(*mut ObjFunction),
}

pub struct CallFrame {
    pub callee: Callee,
    pub ip: *mut u8,
    pub slots: *mut Value,
}

impl CallFrame {
    pub fn null() -> Self {
        Self {
            callee: Callee::Function(std::ptr::null_mut()),

            ip: std::ptr::null_mut(),
            slots: std::ptr::null_mut(),
        }
    }

    pub fn callee_obj(&self) -> *mut Obj {
        match self.callee {
            Callee::Closure(c) => c as *mut Obj,
            Callee::Function(f) => f as *mut Obj,
        }
    }

    pub fn function(&self) -> *mut ObjFunction {
        match self.callee {
            Callee::Closure(ptr) => unsafe { (*ptr).function },
            Callee::Function(ptr) => ptr,
        }
    }

    pub fn is_closure(&self) -> bool {
        matches!(self.callee, Callee::Closure(_))
    }

    pub fn closure(&self) -> *mut ObjClosure {
        match self.callee {
            Callee::Closure(ptr) => ptr,
            _ => std::ptr::null_mut(),
        }
    }

    pub fn enclosing_upvalue(&self, index: usize) -> *mut ObjUpValue {
        match self.callee {
            Callee::Closure(ptr) => unsafe { *(*ptr).upvalues.add(index) },
            Callee::Function(_) => {
                panic!("Cannot capture upvalue from a plain function frame");
            }
        }
    }
}

#[derive(Debug, Clone, Copy)]
struct UpValue {
    index: usize,
    is_local: bool,
}

impl UpValue {
    pub fn new() -> Self {
        Self {
            index: 0,
            is_local: false,
        }
    }
}

pub struct Compiler {
    vm: *mut Vm,
    vars_cache: Table,

    locals: [Local; U8_COUNT],
    upvalues: [UpValue; U8_COUNT],
    local_count: usize,
    scope_depth: usize,

    loop_scopes: Stack<256, LoopScope>,        // стек циклов
    break_jumps: DynamicArray<(usize, usize)>, // (offset_jump, scope_index)

    pub(crate) function: *mut ObjFunction,
    typ: FunctionType,

    pub(crate) enclosing: *mut Compiler,
}

impl Compiler {
    pub fn new(vm: *mut Vm, typ: FunctionType, enclosing: *mut Compiler) -> Box<Self> {
        let mut compiler = Box::new(Self {
            vars_cache: Table::new(),

            local_count: 0,
            scope_depth: 0,
            locals: [Local::empty(); U8_COUNT],
            upvalues: [UpValue::new(); U8_COUNT],
            loop_scopes: Stack::new(),
            break_jumps: DynamicArray::new(unsafe { &mut *vm }),

            function: ObjFunction::allocate(unsafe { &mut *vm }),
            typ,
            vm,

            enclosing,
        });

        if typ != FunctionType::Script {
            let parser = get_parser();
            unsafe {
                (*compiler.function).name =
                    ObjString::copy(&mut *vm, parser.previous.start, parser.previous.length)
            };
        }

        compiler.locals[0].depth = Some(0);
        compiler.locals[0].name.start = "".as_ptr();
        compiler.locals[0].name.length = 0;
        compiler.local_count = 1;

        compiler
    }

    fn error_at(&mut self, token: Token, message: &'static str) {
        let parser = get_parser();
        if parser.panic_mode {
            return;
        }
        parser.panic_mode = true;

        eprint!("[line {}] Error", token.line);
        match token.typ {
            TokenType::EOF => eprint!(" at end"),
            TokenType::ERROR => {}
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
        parser.had_error = true;
    }

    #[inline(always)]
    fn current_chunk(&self) -> &mut Chunk {
        unsafe { &mut *(*self.function).chunk_mut() }
    }

    fn error(&mut self, message: &'static str) {
        self.error_at(get_parser().previous, message);
    }

    fn error_at_current(&mut self, message: &'static str) {
        self.error_at(get_parser().current, message);
    }

    fn consume(&mut self, typ: TokenType, message: &'static str) {
        if get_parser().current.typ == typ {
            self.advance();
            return;
        }

        self.error_at_current(message);
    }

    pub fn advance(&mut self) {
        let parser = get_parser();

        parser.previous = parser.current;

        loop {
            parser.current = scan_token();

            if parser.current.typ != TokenType::ERROR {
                break;
            }

            let message = unsafe {
                std::str::from_utf8_unchecked(std::slice::from_raw_parts(
                    parser.current.start,
                    parser.current.length,
                ))
            };

            self.error_at_current(message);
        }
    }

    fn emit_constant(&mut self, value: Value) -> usize {
        let line = scanner_line();
        self.current_chunk().write_constant(value, line)
    }

    fn emit_byte(&mut self, byte: u8) {
        self.current_chunk().write(byte, get_parser().previous.line);
    }

    fn emit_bytes(&mut self, byte1: u8, byte2: u8) {
        self.emit_byte(byte1);
        self.emit_byte(byte2);
    }

    fn emit_return(&mut self) {
        self.emit_byte(OpCode::OP_NIL as u8);
        self.emit_byte(OpCode::OP_RETURN as u8);
    }

    fn emit_jump(&mut self, instruction: u8) -> usize {
        self.emit_byte(instruction);
        self.emit_byte(0xff);
        self.emit_byte(0xff);

        self.current_chunk().count() - 2
    }

    fn emit_loop(&mut self, loop_start: usize) {
        self.emit_byte(OpCode::OP_LOOP as u8);

        let offset = self.current_chunk().count() - loop_start + 2;
        if offset > u16::MAX as usize {
            self.error("Loop body too large.");
        }

        self.emit_byte((offset >> 8) as u8 & 0xff);
        self.emit_byte(offset as u8 & 0xff);
    }

    fn end(&mut self) -> *mut ObjFunction {
        self.emit_return();

        let function = self.function;

        if cfg!(debug_assertions) && !get_parser().had_error {
            let name = unsafe {
                if !(*function).name().is_null() {
                    (*(*function).name()).as_str()
                } else {
                    "<script>"
                }
            };
            self.current_chunk().disassemble(name);
        }

        function
    }

    fn parse_precedence(&mut self, precedence: Precedence) {
        let parser = get_parser();

        self.advance();

        let Some(prefix_rule) = get_rule(parser.previous.typ).prefix else {
            self.error("Expect expression.");
            return;
        };

        let can_assign = precedence <= Precedence::ASSIGNMENT;
        prefix_rule(self, can_assign);

        while precedence <= get_rule(parser.current.typ).precedence {
            self.advance();

            let Some(infix_rule) = get_rule(parser.previous.typ).infix else {
                self.error("Expect expression.");
                return;
            };

            infix_rule(self, can_assign);
        }

        if can_assign && self.matches(TokenType::EQUAL) {
            self.error("Invalid assignment target.");
        }
    }

    fn expression(&mut self) {
        self.parse_precedence(Precedence::ASSIGNMENT);
    }

    fn dot(&mut self, can_assign: bool) {
        self.consume(TokenType::IDENTIFIER, "Expect property name after '.'.");
        let name = self.identifier_constant(get_parser().previous);

        if can_assign && self.matches(TokenType::EQUAL) {
            self.expression();
            self.emit_bytes(OpCode::OP_SET_PROPERTY as u8, name as u8);
        } else {
            self.emit_bytes(OpCode::OP_GET_PROPERTY as u8, name as u8);
        }
    }

    fn binary(&mut self, _can_assign: bool) {
        let operator_type = get_parser().previous.typ;
        let rule = get_rule(operator_type);

        self.parse_precedence(rule.precedence.next());

        match operator_type {
            // BASIC OPS
            TokenType::PLUS => self.emit_byte(OpCode::OP_ADD as u8),
            TokenType::MINUS => self.emit_byte(OpCode::OP_SUBSTRACT as u8),
            TokenType::STAR => self.emit_byte(OpCode::OP_MULTIPLY as u8),
            TokenType::SLASH => self.emit_byte(OpCode::OP_DIVIDE as u8),
            TokenType::PERCENT => self.emit_byte(OpCode::OP_MOD as u8),

            // EQUALITY
            TokenType::BANG_EQUAL => self.emit_bytes(OpCode::OP_EQUAL as u8, OpCode::OP_NOT as u8),
            TokenType::EQUAL_EQUAL => self.emit_byte(OpCode::OP_EQUAL as u8),
            TokenType::GREATER => self.emit_byte(OpCode::OP_GREATER as u8),
            TokenType::GREATER_EQUAL => {
                self.emit_bytes(OpCode::OP_LESS as u8, OpCode::OP_NOT as u8)
            }
            TokenType::LESS => self.emit_byte(OpCode::OP_LESS as u8),
            TokenType::LESS_EQUAL => {
                self.emit_bytes(OpCode::OP_GREATER as u8, OpCode::OP_NOT as u8)
            }
            _ => unreachable!(),
        }
    }

    fn unary(&mut self, _can_assign: bool) {
        let operator_type = get_parser().previous.typ;

        self.parse_precedence(Precedence::UNARY);

        match operator_type {
            TokenType::MINUS => self.emit_byte(OpCode::OP_NEGATE as u8),
            TokenType::BANG => self.emit_byte(OpCode::OP_NOT as u8),
            _ => unreachable!(),
        }
    }

    fn grouping(&mut self, _can_assign: bool) {
        self.expression();
        self.consume(TokenType::RIGHT_PAREN, "Expect ')' after expression");
    }

    fn literal(&mut self, _can_assign: bool) {
        match get_parser().previous.typ {
            TokenType::FALSE => self.emit_byte(OpCode::OP_FALSE as u8),
            TokenType::TRUE => self.emit_byte(OpCode::OP_TRUE as u8),
            TokenType::NIL => self.emit_byte(OpCode::OP_NIL as u8),
            _ => unreachable!(),
        }
    }

    fn string(&mut self, _can_assign: bool) {
        let parser = get_parser();

        let chars = unsafe { parser.previous.start.add(1) };
        let length = parser.previous.length - 2;

        let vm = unsafe { &mut *self.vm };
        let obj_string = ObjString::copy(vm, chars, length);
        self.emit_constant(Value::Obj(obj_string as *mut Obj));
    }

    fn number(&mut self, _can_assign: bool) {
        let parser = get_parser();
        let num = parser
            .previous
            .as_f64()
            .unwrap_or_else(|| panic!("Invalid number at line {}", parser.previous.line));
        self.emit_constant(Value::Number(num));
    }

    fn or(&mut self, _can_assign: bool) {
        let else_jump = self.emit_jump(OpCode::OP_JUMP_IF_FALSE as u8);
        let end_jump = self.emit_jump(OpCode::OP_JUMP as u8);

        self.patch_jump(else_jump);
        self.emit_byte(OpCode::OP_POP as u8);

        self.parse_precedence(Precedence::OR);
        self.patch_jump(end_jump);
    }

    fn and(&mut self, _can_assign: bool) {
        let end_jump = self.emit_jump(OpCode::OP_JUMP_IF_FALSE as u8);

        self.emit_byte(OpCode::OP_POP as u8);
        self.parse_precedence(Precedence::AND);

        self.patch_jump(end_jump);
    }

    fn variable(&mut self, can_assign: bool) {
        self.named_variable(get_parser().previous, can_assign);
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

    fn add_upvalue(&mut self, index: usize, is_local: bool) -> usize {
        let upvalue_count = unsafe { (*self.function).upvalue_count };

        for i in 0..upvalue_count {
            let upvalue = self.upvalues[i];
            if upvalue.index == index && upvalue.is_local == is_local {
                return i;
            }
        }

        if upvalue_count == U8_COUNT {
            self.error("Too many variables in function.");
            return 0;
        }

        self.upvalues[upvalue_count].is_local = is_local;
        self.upvalues[upvalue_count].index = index;

        unsafe {
            (*self.function).upvalue_count = upvalue_count + 1;
        }

        upvalue_count
    }

    fn resolve_upvalue(&mut self, name: Token) -> Option<usize> {
        if self.enclosing.is_null() {
            return None;
        }

        let compiler = unsafe { &mut *self.enclosing };

        if let Some(local) = compiler.resolve_local(name) {
            compiler.locals[local].is_captured = true;

            return Some(self.add_upvalue(local, true));
        }

        if let Some(upvalue) = compiler.resolve_upvalue(name) {
            return Some(self.add_upvalue(upvalue, false));
        }
        None
    }

    fn get_variable_ops(&mut self, name: Token) -> (OpCode, OpCode, usize) {
        if let Some(arg) = self.resolve_local(name) {
            return (OpCode::OP_GET_LOCAL, OpCode::OP_SET_LOCAL, arg);
        }

        if let Some(arg) = self.resolve_upvalue(name) {
            return (OpCode::OP_GET_UPVALUE, OpCode::OP_SET_UPVALUE, arg);
        }

        let arg = self.identifier_constant(name);
        (OpCode::OP_GET_GLOBAL, OpCode::OP_SET_GLOBAL, arg)
    }

    fn named_variable(&mut self, name: Token, can_assign: bool) {
        let (get_op, set_op, arg) = self.get_variable_ops(name);

        if can_assign && self.matches(TokenType::EQUAL) {
            self.expression();
            self.emit_bytes(set_op as u8, arg as u8);
        } else {
            self.emit_bytes(get_op as u8, arg as u8);
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
        get_parser().current.typ == typ
    }

    fn declaration(&mut self) {
        if self.matches(TokenType::CLASS) {
            self.class_declaration();
        } else if self.matches(TokenType::FUN) {
            self.fun_declaration();
        } else if self.matches(TokenType::VAR) {
            self.var_declaration();
        } else {
            self.statement();
        }

        if get_parser().panic_mode {
            self.synchronize();
        }
    }

    fn class_declaration(&mut self) {
        let class_name = self.parse_variable("Expect class name.");

        self.emit_bytes(OpCode::OP_CLASS as u8, class_name as u8);

        self.define_variable(class_name);

        self.consume(TokenType::LEFT_BRACE, "Expect '{' before class body.");
        self.consume(TokenType::RIGHT_BRACE, "Expect '}' after class body.");
    }

    fn fun_declaration(&mut self) {
        let global = self.parse_variable("Expect function name");
        self.mark_initialized();
        self.function(FunctionType::Function);
        self.define_variable(global);
    }

    fn call(&mut self, _can_assign: bool) {
        let arg_count = self.argument_list();
        self.emit_bytes(OpCode::OP_CALL as u8, arg_count as u8);
    }

    fn argument_list(&mut self) -> usize {
        let mut arg_count = 0;

        if !self.check(TokenType::RIGHT_PAREN) {
            loop {
                self.expression();

                if arg_count == 255 {
                    self.error("Can't have more than 255 arguments.");
                }

                arg_count += 1;

                if !self.matches(TokenType::COMMA) {
                    break;
                }
            }
        }

        self.consume(TokenType::RIGHT_PAREN, "Expect ')' after arguments.");

        arg_count
    }

    fn function(&mut self, typ: FunctionType) {
        let mut compiler = Compiler::new(self.vm, typ, &mut *self as *mut Compiler);
        compiler.begin_scope();

        compiler.consume(TokenType::LEFT_PAREN, "Expect '(' after function name.");

        if !compiler.check(TokenType::RIGHT_PAREN) {
            loop {
                unsafe {
                    (*compiler.function).arity += 1;
                }
                if unsafe { (*compiler.function).arity } > 255 {
                    compiler.error_at_current("Can't have more than 255 parameters.");
                }
                let constant = compiler.parse_variable("Expect parameter name.");
                compiler.define_variable(constant);
                if !compiler.matches(TokenType::COMMA) {
                    break;
                }
            }
        }

        compiler.consume(TokenType::RIGHT_PAREN, "Expect ')' after parameters.");
        compiler.consume(TokenType::LEFT_BRACE, "Expect '{' before function body.");

        compiler.block();

        let function = compiler.end();

        let constant = self
            .current_chunk()
            .add_constant(Value::Obj(function as *mut Obj));
        let upvalue_count = unsafe { (*function).upvalue_count };

        // Определяем closure или обычная функция
        if upvalue_count > 0 {
            self.emit_bytes(OpCode::OP_CLOSURE as u8, constant as u8);
            for i in 0..upvalue_count {
                self.emit_byte(if compiler.upvalues[i].is_local { 1 } else { 0 });
                self.emit_byte(compiler.upvalues[i].index as u8);
            }
        } else {
            self.emit_bytes(OpCode::OP_CONSTANT as u8, constant as u8);
        }
    }

    fn var_declaration(&mut self) {
        let global = self.parse_variable("Expect variable name.");

        if self.matches(TokenType::EQUAL) {
            self.expression();
        } else {
            self.emit_byte(OpCode::OP_NIL as u8);
        }

        self.consume(
            TokenType::SEMICOLON,
            "Expect ';' after variable declaration.",
        );

        self.define_variable(global);
    }

    fn mark_initialized(&mut self) {
        if self.scope_depth == 0 {
            return;
        }

        self.locals[self.local_count - 1].depth = Some(self.scope_depth);
    }

    fn define_variable(&mut self, global: usize) {
        if self.scope_depth > 0 {
            self.mark_initialized();
            return;
        }

        self.emit_bytes(OpCode::OP_DEFINE_GLOBAL as u8, global as u8);
    }

    fn parse_variable(&mut self, error_message: &'static str) -> usize {
        self.consume(TokenType::IDENTIFIER, error_message);

        self.declare_variable();
        if self.scope_depth > 0 {
            return 0;
        }

        self.identifier_constant(get_parser().previous)
    }

    fn declare_variable(&mut self) {
        if self.scope_depth == 0 {
            return;
        }

        let name = get_parser().previous;

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

        self.locals[self.local_count] = Local {
            name,
            depth: None,
            is_captured: false,
        };
        self.local_count += 1;
    }

    fn identifier_constant(&mut self, name: Token) -> usize {
        let vm = unsafe { &mut *self.vm };

        let obj_string = ObjString::copy(vm, name.start, name.length);

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
        let idx = self.current_chunk().add_constant(value);

        // Сохраняем в кэш
        self.vars_cache.set(obj_string, Value::Index(idx));

        idx
    }

    fn synchronize(&mut self) {
        let parser = get_parser();

        parser.panic_mode = false;

        while parser.current.typ != TokenType::EOF {
            if parser.previous.typ == TokenType::SEMICOLON {
                return;
            }

            match parser.current.typ {
                TokenType::CLASS
                | TokenType::FUN
                | TokenType::VAR
                | TokenType::FOR
                | TokenType::IF
                | TokenType::WHILE
                | TokenType::PRINT
                | TokenType::RETURN => return,
                _ => {}
            }

            self.advance();
        }
    }

    fn statement(&mut self) {
        if self.matches(TokenType::PRINT) {
            self.print_statement();
        } else if self.matches(TokenType::CONTINUE) {
            self.consume(TokenType::SEMICOLON, "Expect ';' after 'continue'.");
            self.continue_statement();
        } else if self.matches(TokenType::BREAK) {
            self.consume(TokenType::SEMICOLON, "Expect ';' after 'break'.");
            self.break_statement();
        } else if self.matches(TokenType::FOR) {
            self.for_statement();
        } else if self.matches(TokenType::IF) {
            self.if_statement();
        } else if self.matches(TokenType::RETURN) {
            self.return_statement();
        } else if self.matches(TokenType::WHILE) {
            self.while_statement();
        } else if self.matches(TokenType::LEFT_BRACE) {
            self.begin_scope();
            self.block();
            self.end_scope();
        } else {
            self.expression_statement();
        }
    }

    fn return_statement(&mut self) {
        if self.typ == FunctionType::Script {
            self.error("Can't return from top-level code.");
        }

        if self.matches(TokenType::SEMICOLON) {
            self.emit_return();
        } else {
            self.expression();
            self.consume(TokenType::SEMICOLON, "Expect ';' after return value.");
            self.emit_byte(OpCode::OP_RETURN as u8);
        }
    }

    fn patch_breaks(&mut self, scope_idx: usize, exit_offset: usize) {
        let mut i = 0;
        while i < self.break_jumps.len() {
            let (jump_offset, saved_scope_idx) = self.break_jumps[i];
            if saved_scope_idx == scope_idx {
                let jump = exit_offset - jump_offset - 2;
                self.current_chunk()
                    .code
                    .set(jump_offset, ((jump >> 8) & 0xff) as u8);
                self.current_chunk()
                    .code
                    .set(jump_offset + 1, (jump & 0xff) as u8);
                self.break_jumps.remove(i);
            } else {
                i += 1;
            }
        }
    }

    fn break_statement(&mut self) {
        if self.loop_scopes.is_empty() {
            self.error("'break' must be inside a loop.");
        }

        let scope_depth = self.loop_scopes.peek(0).scope_depth;
        while self.local_count > 0 {
            let local = self.locals[self.local_count - 1];
            if local.depth.unwrap_or(0) <= scope_depth {
                break;
            }
            self.emit_byte(OpCode::OP_POP as u8);
            self.local_count -= 1;
        }

        // Прыгаем на выход (пока placeholder)
        let jump = self.emit_jump(OpCode::OP_JUMP as u8);

        // Сохраняем для патчинга
        self.break_jumps.write((jump, self.loop_scopes.len() - 1));
    }

    fn continue_statement(&mut self) {
        if self.loop_scopes.is_empty() {
            self.error("'continue' must be inside a loop.");
        }

        let (scope_depth, scope_start) = {
            let scope = self.loop_scopes.peek(0);
            (scope.scope_depth, scope.start)
        };

        // Очищаем локальные переменные, объявленные в теле цикла
        while self.local_count > 0 {
            let local = self.locals[self.local_count - 1];
            if local.depth.unwrap_or(0) <= scope_depth {
                break;
            }
            self.emit_byte(OpCode::OP_POP as u8);
            self.local_count -= 1;
        }

        // Прыгаем на начало цикла (или на инкремент для for)
        self.emit_loop(scope_start);
    }

    fn for_statement(&mut self) {
        self.begin_scope();

        let scope_idx = self.loop_scopes.len();
        self.loop_scopes.push(LoopScope {
            start: 0,
            exit_jump: None,
            scope_depth: self.scope_depth,
            has_increment: false,
        });

        self.consume(TokenType::LEFT_PAREN, "Expect '(' after 'for'.");

        if self.matches(TokenType::SEMICOLON) {
            // no initializer
        } else if self.matches(TokenType::VAR) {
            self.var_declaration();
        } else {
            self.expression_statement();
        }

        let mut loop_start = self.current_chunk().count();
        let mut exit_jump = None;

        if !self.matches(TokenType::SEMICOLON) {
            self.expression();
            self.consume(TokenType::SEMICOLON, "Expect ';' after loop condition.");

            // Jump out of the loop if the condition is false.
            exit_jump = Some(self.emit_jump(OpCode::OP_JUMP_IF_FALSE as u8));
            self.emit_byte(OpCode::OP_POP as u8);
        }

        if !self.matches(TokenType::RIGHT_PAREN) {
            let body_jump = self.emit_jump(OpCode::OP_JUMP as u8);
            let increment_start = self.current_chunk().count();

            self.expression();
            self.emit_byte(OpCode::OP_POP as u8);

            self.consume(TokenType::RIGHT_PAREN, "Expect ')' after for clauses.");

            self.emit_loop(loop_start);
            loop_start = increment_start;
            self.loop_scopes.peek_mut(0).start = increment_start;
            self.loop_scopes.peek_mut(0).has_increment = true;
            self.patch_jump(body_jump);
        } else {
            self.loop_scopes.peek_mut(0).start = loop_start;
        }

        self.statement();
        self.emit_loop(loop_start);

        if let Some(exit_jump) = exit_jump {
            self.patch_jump(exit_jump);
            self.emit_byte(OpCode::OP_POP as u8);
        }

        let count = self.current_chunk().count();
        self.loop_scopes.peek_mut(0).exit_jump = Some(count);
        self.patch_breaks(scope_idx, count);
        self.loop_scopes.pop();

        self.end_scope();
    }

    fn while_statement(&mut self) {
        let loop_start = self.current_chunk().count();

        // сохраняем информацию о цикле
        let scope_idx = self.loop_scopes.len();
        self.loop_scopes.push(LoopScope {
            start: loop_start,
            exit_jump: None,
            scope_depth: self.scope_depth,
            has_increment: false,
        });

        self.consume(TokenType::LEFT_PAREN, "Expect '(' after 'while'.");
        self.expression();
        self.consume(TokenType::RIGHT_PAREN, "Expect ')' after condition.");

        let exit_jump = self.emit_jump(OpCode::OP_JUMP_IF_FALSE as u8);
        self.emit_byte(OpCode::OP_POP as u8);

        self.statement();

        self.emit_loop(loop_start);

        self.patch_jump(exit_jump);
        self.emit_byte(OpCode::OP_POP as u8);

        // Обновляем exit_jump для break
        let count = self.current_chunk().count();
        self.loop_scopes.peek_mut(0).exit_jump = Some(count);
        self.loop_scopes.pop();
        self.patch_breaks(scope_idx, count);
    }

    fn if_statement(&mut self) {
        self.consume(TokenType::LEFT_PAREN, "Expect '(' after 'if'.");
        self.expression();
        self.consume(TokenType::RIGHT_PAREN, "Expect ')' after condition.");

        let then_jump = self.emit_jump(OpCode::OP_JUMP_IF_FALSE as u8);
        self.emit_byte(OpCode::OP_POP as u8);
        self.statement();

        let else_jump = self.emit_jump(OpCode::OP_JUMP as u8);

        self.patch_jump(then_jump);
        self.emit_byte(OpCode::OP_POP as u8);

        if self.matches(TokenType::ELSE) {
            self.statement();
        }

        self.patch_jump(else_jump);
    }

    fn patch_jump(&mut self, offset: usize) {
        // -2 to adjust for the bytecode for the jump offset itself.
        let jump = self.current_chunk().count() - offset - 2;

        if jump > u16::MAX as usize {
            self.error("Too much code to jump over");
        }
        self.current_chunk()
            .code
            .set(offset, ((jump >> 8) & 0xff) as u8);
        self.current_chunk()
            .code
            .set(offset + 1, (jump & 0xff) as u8);
    }

    fn begin_scope(&mut self) {
        self.scope_depth += 1;
    }

    fn end_scope(&mut self) {
        self.scope_depth -= 1;

        while self.local_count > 0 {
            let local = self.locals[self.local_count - 1];

            if local.depth.unwrap_or(0) > self.scope_depth {
                if local.is_captured {
                    self.emit_byte(OpCode::OP_CLOSE_UPVALUE as u8);
                } else {
                    self.emit_byte(OpCode::OP_POP as u8);
                }

                self.local_count -= 1;
            } else {
                break;
            }
        }
    }

    fn block(&mut self) {
        while !self.check(TokenType::RIGHT_BRACE) && !self.check(TokenType::EOF) {
            self.declaration();
        }

        self.consume(TokenType::RIGHT_BRACE, "Expect '}' after block.");
    }

    fn expression_statement(&mut self) {
        self.expression();
        self.consume(TokenType::SEMICOLON, "Expect ';' after expression");
        self.emit_byte(OpCode::OP_POP as u8);
    }

    fn print_statement(&mut self) {
        self.expression();
        self.consume(TokenType::SEMICOLON, "Expect ';' after value");
        self.emit_byte(OpCode::OP_PRINT as u8);
    }

    pub fn compile(&mut self, source: *const u8) -> *mut ObjFunction {
        let parser = get_parser();

        init_scanner(source);

        parser.had_error = false;
        parser.panic_mode = false;

        self.advance();

        while !self.matches(TokenType::EOF) {
            self.declaration();
        }

        let function = self.end();

        if parser.had_error {
            std::ptr::null_mut()
        } else {
            function
        }
    }
}
