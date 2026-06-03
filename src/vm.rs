use tracing::debug;

use crate::{
    common::{Chunk, OpCode, Value},
    compiler::Compiler,
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

    compiler: Compiler,
}

impl Vm {
    pub fn new() -> Self {
        Self {
            ip: std::ptr::null(),
            compiler: Compiler::new(),
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
                OpCode::OP_CONSTANT => {
                    let constant = self.read_constant(chunk);
                    stack.push(constant);
                }
                OpCode::OP_CONSTANT_LONG => {
                    let constant = self.read_constant_long(chunk);
                    stack.push(constant);
                }
                OpCode::OP_RETURN => {
                    println!("{}", stack.pop());
                    return InterpretResult::Ok;
                }
                OpCode::OP_NEGATE => {
                    let value = stack.pop();
                    stack.push(-value);
                }
                OpCode::OP_ADD => {
                    let right = stack.pop();
                    let left = stack.pop();
                    stack.push(left + right);
                }
                OpCode::OP_SUBSTRACT => {
                    let right = stack.pop();
                    let left = stack.pop();
                    stack.push(left - right);
                }
                OpCode::OP_MULTIPLY => {
                    let right = stack.pop();
                    let left = stack.pop();
                    stack.push(left * right);
                }
                OpCode::OP_DIVIDE => {
                    let right = stack.pop();
                    let left = stack.pop();
                    stack.push(left / right);
                }
            }
        }
    }

    pub fn interpret_new(&mut self, source: *const u8) -> InterpretResult {
        self.compiler.compile_old(source);

        InterpretResult::Ok
    }

    pub fn interpret(&mut self, source: *const u8) -> InterpretResult {
        let mut chunk = Chunk::new();

        if !self.compiler.compile(source, &mut chunk) {
            return InterpretResult::CompileError;
        };

        let mut stack = Stack::new();

        let result = self.run(&mut stack, &chunk);

        result
    }
}

impl Drop for Vm {
    fn drop(&mut self) {}
}
