use tracing::debug;

use crate::{
    common::{
        dynamic_array::DynamicArray,
        LineRun,
        OpCode::{self},
        Value,
    },
    vm::{ValueStack, Vm},
};

pub struct Chunk {
    pub code: DynamicArray<u8>,
    pub constants: DynamicArray<Value>,
    lines: DynamicArray<LineRun>,
    stack: *mut ValueStack,
}

impl Chunk {
    pub fn count(&self) -> usize {
        self.code.count()
    }

    pub fn get_code_ptr(&self) -> *const u8 {
        self.code.as_ptr()
    }

    pub fn get_code_mut_ptr(&self) -> *mut u8 {
        self.code.as_mut_ptr()
    }

    pub fn new(vm: &mut Vm) -> Self {
        debug!("Chunk is initialized");

        let res = Self {
            code: DynamicArray::new(vm),
            constants: DynamicArray::new(vm),
            lines: DynamicArray::new(vm),
            stack: &mut vm.stack,
        };

        res
    }

    pub fn with_capacity(
        vm: &mut Vm,
        code_capacity: usize,
        constants_capacity: usize,
        lines_capacity: usize,
    ) -> Self {
        Self {
            code: DynamicArray::with_capacity(vm, code_capacity),
            constants: DynamicArray::with_capacity(vm, constants_capacity),
            lines: DynamicArray::with_capacity(vm, lines_capacity),
            stack: &mut vm.stack,
        }
    }

    pub fn dump_content(&self) -> Vec<u8> {
        let mut bytes = Vec::new();

        for i in 0..self.code.count() {
            if let Some(&value) = self.code.get(i) {
                bytes.push(value);
            }
        }

        bytes
    }

    pub fn disassemble(&self, name: &str) {
        println!("== {name} ==");

        let mut offset = 0;
        while offset < self.code.count() {
            offset = self.disassemble_unstruction(offset);
        }
    }

    pub fn disassemble_unstruction(&self, offset: usize) -> usize {
        print!("{:04} ", offset);

        let current_line = self.get_line(offset);
        let prev_line = if offset > 0 {
            self.get_line(offset - 1)
        } else {
            None
        };

        if offset > 0 && current_line == prev_line {
            print!("   | ");
        } else if let Some(line) = current_line {
            print!("{:4} ", line);
        } else {
            print!("    ? ");
        }

        let Some(&instruction) = self.code.get(offset) else {
            panic!("Instruction not found");
        };

        let Some(op_code) = OpCode::from_byte(instruction) else {
            println!("Unknown opcode {}", instruction);
            return offset + 1;
        };

        match op_code {
            OpCode::OP_CONSTANT_LONG => self.constant_long_instruction(op_code, offset),
            OpCode::OP_CONSTANT => self.constant_instruction(op_code, offset),
            OpCode::OP_CLASS => self.constant_instruction(op_code, offset),
            OpCode::OP_DEFINE_GLOBAL => self.constant_instruction(op_code, offset),
            OpCode::OP_GET_GLOBAL => self.constant_instruction(op_code, offset),
            OpCode::OP_SET_GLOBAL => self.constant_instruction(op_code, offset),
            OpCode::OP_GET_PROPERTY => self.constant_instruction(op_code, offset),
            OpCode::OP_SET_PROPERTY => self.constant_instruction(op_code, offset),
            OpCode::OP_METHOD => self.constant_instruction(op_code, offset),
            OpCode::OP_GET_SUPER => self.constant_instruction(op_code, offset),

            OpCode::OP_SET_LOCAL => self.byte_instruction(op_code, offset),
            OpCode::OP_GET_LOCAL => self.byte_instruction(op_code, offset),
            OpCode::OP_SET_UPVALUE => self.byte_instruction(op_code, offset),
            OpCode::OP_GET_UPVALUE => self.byte_instruction(op_code, offset),
            OpCode::OP_CALL => self.byte_instruction(op_code, offset),

            OpCode::OP_JUMP => self.jump_instruction(op_code, 1, offset),
            OpCode::OP_JUMP_IF_FALSE => self.jump_instruction(op_code, 1, offset),
            OpCode::OP_LOOP => self.jump_instruction(op_code, -1, offset),

            OpCode::OP_INVOKE => self.invoke_instruction(op_code, offset),
            OpCode::OP_SUPER_INVOKE => self.invoke_instruction(op_code, offset),

            OpCode::OP_CLOSURE => {
                let mut offset = offset + 1;

                let constant = self.code[offset];

                offset += 1;

                println!(
                    "{:16} {:4} {}",
                    op_code.name(),
                    constant,
                    self.constants[constant as usize]
                );

                let function = self.constants[constant as usize].as_function();
                let upvalue_count = unsafe { (*function).upvalue_count };

                for _ in 0..upvalue_count {
                    let is_local = self.code[offset];
                    let index = self.code[offset + 1];

                    println!(
                        "{:4}      |                     {} {}",
                        offset,
                        if is_local == 1 { "local" } else { "upvalue" },
                        index
                    );
                    offset += 2;
                }

                offset
            }

            _ => Self::simple_instruction(op_code, offset),
        }
    }

    fn invoke_instruction(&self, op_code: OpCode, offset: usize) -> usize {
        let constant = self.code[offset + 1];
        let arg_count = self.code[offset + 2];
        println!(
            "{:16} ({} args) {} '{}'",
            op_code.name(),
            arg_count,
            constant,
            self.constants[constant as usize]
        );

        offset + 3
    }

    fn jump_instruction(&self, op_code: OpCode, sign: isize, offset: usize) -> usize {
        let jump = unsafe {
            let b0 = *self.code.as_ptr().add(offset + 1) as u16;
            let b1 = *self.code.as_ptr().add(offset + 2) as u16;
            (b0 << 8) | b1
        };

        println!(
            "{:16} {:4} -> {}",
            op_code.name(),
            offset,
            offset as isize + 3 + (sign * jump as isize)
        );

        offset + 3
    }

    fn byte_instruction(&self, op_code: OpCode, offset: usize) -> usize {
        let slot = unsafe { *self.code.as_ptr().add(offset + 1) };

        println!("{:16} {:4}", op_code.name(), slot);

        offset + 2
    }

    fn simple_instruction(op_code: OpCode, offset: usize) -> usize {
        println!("{:16}", op_code.name());

        offset + 1
    }

    fn constant_long_instruction(&self, op_code: OpCode, offset: usize) -> usize {
        // Читаем 24-битный индекс (little-endian)
        let b0 = *self.code.get(offset + 1).unwrap() as usize;
        let b1 = *self.code.get(offset + 2).unwrap() as usize;
        let b2 = *self.code.get(offset + 3).unwrap() as usize;
        let constant_idx = b0 | (b1 << 8) | (b2 << 16);

        print!("{:16} {:4} '", op_code.name(), constant_idx);

        if let Some(value) = self.constants.get(constant_idx) {
            print!("{}", value);
        }
        println!("'");

        offset + 4
    }

    fn constant_instruction(&self, op_code: OpCode, offset: usize) -> usize {
        let constant = *self.code.get(offset + 1).unwrap();
        print!("{:16} {:4} '", op_code.name(), constant);
        let value = self.constants.get(constant as usize).unwrap();
        print!("{}", value);
        println!("'");

        offset + 2
    }

    pub fn write(&mut self, value: impl Into<u8>, line: usize) {
        let byte = value.into();

        self.code.write(byte);

        // RLE компрессия для line info
        if self.lines.count() > 0 {
            let last_index = self.lines.count() - 1;

            if let Some(last_run) = self.lines.get_mut(last_index) {
                if last_run.line == line {
                    last_run.count += 1;
                    debug!(
                        "Wrote byte {:#04x} at offset {} (same line, count now {})",
                        byte,
                        self.code.count() - 1,
                        last_run.count
                    );
                    return;
                }
            }
        }

        // Новая строка - создаём новый run
        self.lines.write(LineRun { line, count: 1 });

        debug!(
            "Wrote byte {:#04x} at offset {}",
            byte,
            self.code.count() - 1
        );
    }

    pub fn write_constant(&mut self, value: Value, line: usize) -> usize {
        let index = self.add_constant(value);

        if index <= 255 {
            self.write(OpCode::OP_CONSTANT, line);
            self.write(index as u8, line);
            debug!("Added short constant");
        } else if index <= 0xFFFFFF {
            // Используем длинную инструкцию (4 байта)
            self.write(OpCode::OP_CONSTANT_LONG, line);
            self.write(index as u8, line); // Младший байт
            self.write((index >> 8) as u8, line); // Средний байт
            self.write((index >> 16) as u8, line); // Старший байт
            debug!("Added long constant");
        } else {
            panic!(
                "Constant index {} too large for 24-bit (max 16,777,215)",
                index
            );
        }

        index
    }

    pub fn add_constant(&mut self, value: Value) -> usize {
        let stack = unsafe { &mut *self.stack };

        stack.push(value);

        self.constants.write(value);

        stack.pop();

        self.constants.count() - 1
    }

    pub fn get_instruction(&self, offset: usize) -> Option<u8> {
        self.code.get(offset).copied()
    }

    pub fn get_constant(&self, index: usize) -> Option<&Value> {
        self.constants.get(index)
    }

    // Получить номер строки для инструкции по offset
    pub fn get_line(&self, offset: usize) -> Option<usize> {
        let mut remaining = offset;

        for run in 0..self.lines.count() {
            let line_run = self.lines.get(run).unwrap();
            if remaining < line_run.count {
                return Some(line_run.line);
            }
            remaining -= line_run.count;
        }

        None
    }

    pub fn size(&self) -> usize {
        self.code.size() + self.constants.size() + self.lines.size()
    }

    pub fn free(&mut self) {
        self.code.free();
        self.constants.free();
        self.lines.free();
    }
}

impl std::fmt::Debug for Chunk {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Chunk")
            .field("count", &self.code.count())
            .field("capacity", &self.code.capacity())
            .field("code", &"<raw pointer>")
            .finish()
    }
}
