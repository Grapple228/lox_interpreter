use tracing::debug;

use crate::{
    common::{
        object::{Obj, ObjString, ObjType},
        Chunk, OpCode, Value, ValueResult,
    },
    compiler::Compiler,
    table::Table,
    Stack,
};

pub enum InterpretResult {
    Ok,
    CompileError,
    RuntimeError,
}

impl InterpretResult {
    pub const fn as_str(&self) -> &'static str {
        match self {
            InterpretResult::Ok => "INTERPRET_OK",
            InterpretResult::CompileError => "INTERPRET_COMPILE_ERROR",
            InterpretResult::RuntimeError => "INTERPRET_RUNTIME_ERROR",
        }
    }
}

pub struct Vm {
    ip: *const u8,

    pub objects: *mut Obj,
    pub bytes_allocated: usize,

    pub strings: Table,
    pub globals: Table,
}

impl Vm {
    pub fn new() -> Self {
        Self {
            ip: std::ptr::null(),
            objects: std::ptr::null_mut(),
            bytes_allocated: 0,

            strings: Table::new(),
            globals: Table::new(),
        }
    }

    fn read_byte(&mut self) -> u8 {
        unsafe {
            if self.ip.is_null() {
                panic!("IP is null");
            }

            let byte = *self.ip;
            self.ip = self.ip.add(1);
            byte
        }
    }

    fn read_u16(&mut self) -> u16 {
        unsafe {
            if self.ip.is_null() {
                panic!("IP is null");
            }

            let b0 = *self.ip as u16;
            let b1 = *self.ip.add(1) as u16;
            self.ip = self.ip.add(2);
            (b0 << 8) | b1
        }
    }

    fn read_constant(&mut self, chunk: &Chunk) -> Value {
        let idx = self.read_byte() as usize;
        *chunk.get_constant(idx).expect("Constant not found")
    }

    fn read_constant_long(&mut self, chunk: &Chunk) -> Value {
        let b0 = self.read_byte() as usize;
        let b1 = self.read_byte() as usize;
        let b2 = self.read_byte() as usize;
        let idx = b0 | (b1 << 8) | (b2 << 16);
        *chunk.get_constant(idx).expect("Constant not found")
    }

    fn current_offset(&self, chunk: &Chunk) -> usize {
        let start = chunk.get_code_ptr();
        if start.is_null() || self.ip.is_null() {
            return 0;
        }
        // Разница между текущим IP и началом чанка = смещение в байтах
        unsafe { self.ip.offset_from(start) as usize }
    }

    fn runtime_error(&mut self, stack: &mut Stack, chunk: &Chunk, message: &'static str) {
        eprintln!("{}", message);

        let instruction = self.current_offset(chunk) - 1;
        let line = chunk.get_line(instruction).unwrap_or(0);
        eprintln!("[line {}] in script", line);

        stack.reset();
        self.ip = std::ptr::null_mut();
    }

    fn unary_op<F>(&mut self, stack: &mut Stack, chunk: &Chunk, f: F) -> bool
    where
        F: FnOnce(Value) -> ValueResult,
    {
        let value = stack.pop();

        match f(value) {
            ValueResult::Success(value) => {
                stack.push(value);
                false
            }
            ValueResult::Error(message) => {
                self.runtime_error(stack, chunk, message);
                true
            }
        }
    }

    fn binary_op<F>(&mut self, stack: &mut Stack, chunk: &Chunk, f: F) -> bool
    where
        F: FnOnce(Value, Value) -> ValueResult,
    {
        let right = stack.pop();
        let left = stack.pop();

        match f(left, right) {
            ValueResult::Success(value) => {
                stack.push(value);
                false
            }
            ValueResult::Error(message) => {
                self.runtime_error(stack, chunk, message);
                true
            }
        }
    }

    fn comparison_op<F>(&mut self, stack: &mut Stack, chunk: &Chunk, f: F) -> bool
    where
        F: FnOnce(f64, f64) -> bool,
    {
        let right = stack.pop();
        let left = stack.pop();

        match (left, right) {
            (Value::Number(a), Value::Number(b)) => {
                stack.push(Value::Bool(f(a, b)));
                false
            }
            _ => {
                self.runtime_error(stack, chunk, "Operands must be numbers.");
                true
            }
        }
    }

    fn add_strings(&mut self, a: *mut Obj, b: *mut Obj) -> Option<Value> {
        unsafe {
            if a.is_null() || b.is_null() {
                return None;
            }
            if (*a).typ() != ObjType::String || (*b).typ() != ObjType::String {
                return None;
            }
            let a_str = a as *mut ObjString;
            let b_str = b as *mut ObjString;
            let result = self.concatenate_strings(a_str, b_str);
            Some(Value::Obj(result as *mut Obj))
        }
    }

    fn read_string(&mut self, stack: &mut Stack, chunk: &Chunk) -> Option<*mut ObjString> {
        // 1. Читаем имя переменной из констант
        let constant_idx = self.read_byte() as usize;
        let name = chunk.get_constant(constant_idx).unwrap();

        // 2. Имя должно быть строкой
        let name_str = match *name {
            Value::Obj(ptr) if unsafe { (*ptr).typ() == ObjType::String } => ptr as *mut ObjString,
            _ => {
                self.runtime_error(stack, chunk, "Global variable name must be a string.");
                return None;
            }
        };

        Some(name_str)
    }

    fn run(&mut self, stack: &mut Stack, chunk: &Chunk) -> InterpretResult {
        self.ip = chunk.get_code_ptr();

        loop {
            if cfg!(debug_assertions) {
                stack.debug_content();
                chunk.disassemble_unstruction(self.current_offset(chunk));
            }

            let Some(op) = OpCode::from_byte(self.read_byte()) else {
                panic!("Invalid op code");
            };

            match op {
                OpCode::OP_JUMP_IF_FALSE => {
                    let offset = self.read_u16();
                    if stack.peek(0).is_falsey() {
                        self.ip = unsafe { self.ip.add(offset as usize) };
                    }
                }
                OpCode::OP_JUMP => {
                    let offset = self.read_u16();
                    self.ip = unsafe { self.ip.add(offset as usize) };
                }

                OpCode::OP_LOOP => {
                    let offset = self.read_u16();
                    self.ip = unsafe { self.ip.sub(offset as usize) };
                }

                OpCode::OP_POP => {
                    _ = stack.pop();
                }

                OpCode::OP_GET_LOCAL => {
                    let slot = self.read_byte() as usize;
                    let value = stack.get(slot);
                    stack.push(value);
                }

                OpCode::OP_SET_LOCAL => {
                    let slot = self.read_byte() as usize;
                    let value = stack.peek(0);
                    stack.set(slot, *value);
                }

                OpCode::OP_SET_GLOBAL => {
                    let Some(name_str) = self.read_string(stack, chunk) else {
                        return InterpretResult::RuntimeError;
                    };

                    let value = stack.peek(0);

                    if self.globals.set(name_str, *value) {
                        self.globals.delete(name_str);
                        self.runtime_error(stack, chunk, "Undefined variable");
                        return InterpretResult::RuntimeError;
                    }
                }

                OpCode::OP_GET_GLOBAL => {
                    let Some(name_str) = self.read_string(stack, chunk) else {
                        return InterpretResult::RuntimeError;
                    };

                    let mut value = Value::Nil;

                    if !self.globals.get(name_str, &mut value) {
                        self.runtime_error(stack, chunk, "Undefined variable");
                        return InterpretResult::RuntimeError;
                    }

                    stack.push(value);
                }

                OpCode::OP_DEFINE_GLOBAL => {
                    let Some(name_str) = self.read_string(stack, chunk) else {
                        return InterpretResult::RuntimeError;
                    };

                    let value = stack.peek(0);

                    self.globals.set(name_str, *value);

                    stack.pop();
                }

                OpCode::OP_NIL => {
                    stack.push(Value::Nil);
                }
                OpCode::OP_TRUE => {
                    stack.push(Value::Bool(true));
                }
                OpCode::OP_FALSE => {
                    stack.push(Value::Bool(false));
                }

                OpCode::OP_PRINT => {
                    println!("{}", stack.pop());
                }

                OpCode::OP_RETURN => {
                    return InterpretResult::Ok;
                }

                OpCode::OP_CONSTANT => {
                    let constant = self.read_constant(chunk);
                    stack.push(constant);
                }
                OpCode::OP_CONSTANT_LONG => {
                    let constant = self.read_constant_long(chunk);
                    stack.push(constant);
                }

                OpCode::OP_NOT => {
                    if self.unary_op(stack, chunk, |a| !a) {
                        return InterpretResult::RuntimeError;
                    }
                }
                OpCode::OP_NEGATE => {
                    if self.unary_op(stack, chunk, |a| -a) {
                        return InterpretResult::RuntimeError;
                    }
                }
                OpCode::OP_ADD => {
                    const ERR_MSG: &str = "Operands must be numbers or strings.";
                    let right = stack.pop();
                    let left = stack.pop();

                    match (left, right) {
                        (Value::Number(a), Value::Number(b)) => stack.push(Value::Number(a + b)),
                        (Value::Obj(a), Value::Obj(b)) => match self.add_strings(a, b) {
                            Some(v) => stack.push(v),
                            None => {
                                self.runtime_error(stack, chunk, ERR_MSG);
                                return InterpretResult::RuntimeError;
                            }
                        },
                        _ => {
                            self.runtime_error(stack, chunk, ERR_MSG);
                            return InterpretResult::RuntimeError;
                        }
                    }
                }
                OpCode::OP_SUBSTRACT => {
                    if self.binary_op(stack, chunk, |a, b| a - b) {
                        return InterpretResult::RuntimeError;
                    }
                }
                OpCode::OP_MULTIPLY => {
                    if self.binary_op(stack, chunk, |a, b| a * b) {
                        return InterpretResult::RuntimeError;
                    }
                }
                OpCode::OP_DIVIDE => {
                    if self.binary_op(stack, chunk, |a, b| a / b) {
                        return InterpretResult::RuntimeError;
                    }
                }
                OpCode::OP_EQUAL => {
                    let right = stack.pop();
                    let left = stack.pop();

                    stack.push(Value::Bool(left == right));
                }
                OpCode::OP_GREATER => {
                    if self.comparison_op(stack, chunk, |a, b| a > b) {
                        return InterpretResult::RuntimeError;
                    }
                }
                OpCode::OP_LESS => {
                    if self.comparison_op(stack, chunk, |a, b| a < b) {
                        return InterpretResult::RuntimeError;
                    }
                }
            }
        }
    }

    pub fn interpret(&mut self, source: *const u8) -> InterpretResult {
        let mut chunk = Chunk::new();
        let mut compiler = Compiler::new();

        if !compiler.compile(self, source, &mut chunk) {
            return InterpretResult::CompileError;
        };

        let mut stack = Stack::new();

        let result = self.run(&mut stack, &chunk);

        result
    }
}

impl Drop for Vm {
    fn drop(&mut self) {
        self.free_objects();

        if !self.ip.is_null() {
            self.ip = std::ptr::null_mut();
        }
    }
}
