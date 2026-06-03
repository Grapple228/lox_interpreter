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

pub struct Vm<'a> {
    chunk: Option<&'a Chunk>,
    ip: *const u8,

    compiler: Compiler,
}

impl<'a> Vm<'a> {
    pub fn new() -> Self {
        Self {
            chunk: None,
            ip: std::ptr::null(),
            compiler: Compiler,
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

    fn read_constant(&mut self) -> Value {
        let chunk = self.chunk.expect("Chunk should exist at this point");
        let idx = self.read_byte() as usize;
        *chunk.get_constant(idx).expect("Constant not found")
    }

    fn read_constant_long(&mut self) -> Value {
        let chunk = self.chunk.expect("Chunk should exist");
        let b0 = self.read_byte() as usize;
        let b1 = self.read_byte() as usize;
        let b2 = self.read_byte() as usize;
        let idx = b0 | (b1 << 8) | (b2 << 16);
        *chunk.get_constant(idx).expect("Constant not found")
    }

    fn current_offset(&self) -> usize {
        let chunk = self.chunk.expect("Chunk should exist");

        let start = chunk.get_code_ptr();
        if start.is_null() || self.ip.is_null() {
            return 0;
        }
        // Разница между текущим IP и началом чанка = смещение в байтах
        unsafe { self.ip.offset_from(start) as usize }
    }

    fn run(&mut self, stack: &mut Stack) -> InterpretResult {
        loop {
            if cfg!(debug_assertions) {
                stack.debug_content();

                let chunk = self.chunk.expect("Chunk should exist");
                chunk.disassemble_unstruction(self.current_offset());
            }

            let Some(op) = OpCode::from_byte(self.read_byte()) else {
                panic!("Invalid op code");
            };

            match op {
                OpCode::OP_CONSTANT => {
                    let constant = self.read_constant();
                    stack.push(constant);
                }
                OpCode::OP_CONSTANT_LONG => {
                    let constant = self.read_constant_long();
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
        self.compiler.compile(source);

        InterpretResult::Ok
    }

    pub fn interpret(&mut self, chunk: &'a Chunk) -> InterpretResult {
        self.chunk = Some(chunk);
        self.ip = chunk.get_code_ptr();

        let mut stack = Stack::new();

        self.run(&mut stack)
    }
}

impl<'a> Drop for Vm<'a> {
    fn drop(&mut self) {}
}
