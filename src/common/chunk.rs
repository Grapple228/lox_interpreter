use std::{
    alloc::{alloc, dealloc, realloc, Layout},
    ptr::NonNull,
};
use tracing::debug;

use crate::common::{
    dynamic_array::DynamicArray,
    LineRun,
    OpCode::{self, OP_RETURN},
    Value,
};

pub struct Chunk {
    pub code: DynamicArray<u8>,
    constants: DynamicArray<Value>,
    lines: DynamicArray<LineRun>,
}

impl Chunk {
    pub fn get_code_ptr(&self) -> *const u8 {
        self.code.as_ptr()
    }

    pub fn get_code_mut_ptr(&self) -> *mut u8 {
        self.code.as_mut_ptr()
    }

    pub fn new() -> Self {
        debug!("Chunk is initialized");

        Self {
            code: DynamicArray::new(),
            constants: DynamicArray::new(),
            lines: DynamicArray::new(),
        }
    }

    pub fn with_capacity(
        code_capacity: usize,
        constants_capacity: usize,
        lines_capacity: usize,
    ) -> Self {
        Self {
            code: DynamicArray::with_capacity(code_capacity),
            constants: DynamicArray::with_capacity(constants_capacity),
            lines: DynamicArray::with_capacity(lines_capacity),
        }
    }

    pub fn dump_content(&self) -> Vec<u8> {
        let mut bytes = Vec::new();

        for i in 0..self.code.len() {
            if let Some(&value) = self.code.get(i) {
                unsafe {
                    bytes.push(value);
                }
            }
        }

        bytes
    }

    pub fn disassemble(&self, name: &str) {
        println!("== {name} ==");

        let mut offset = 0;
        while offset < self.code.len() {
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
            _ => Self::simple_instruction(op_code, offset),
        }
    }

    fn simple_instruction(op_code: OpCode, offset: usize) -> usize {
        println!("{}", op_code.name());

        offset + 1
    }

    fn constant_long_instruction(&self, op_code: OpCode, offset: usize) -> usize {
        // Читаем 24-битный индекс (little-endian)
        let b0 = *self.code.get(offset + 1).unwrap() as usize;
        let b1 = *self.code.get(offset + 2).unwrap() as usize;
        let b2 = *self.code.get(offset + 3).unwrap() as usize;
        let constant_idx = b0 | (b1 << 8) | (b2 << 16);

        print!("{:16} {:4} '", "OP_CONSTANT_LONG", constant_idx);

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
        if self.lines.len() > 0 {
            let last_index = self.lines.len() - 1;

            if let Some(last_run) = self.lines.get_mut(last_index) {
                if last_run.line == line {
                    last_run.count += 1;
                    debug!(
                        "Wrote byte {:#04x} at offset {} (same line, count now {})",
                        byte,
                        self.code.len() - 1,
                        last_run.count
                    );
                    return;
                }
            }
        }

        // Новая строка - создаём новый run
        self.lines.write(LineRun { line, count: 1 });

        debug!("Wrote byte {:#04x} at offset {}", byte, self.code.len() - 1);
    }

    pub fn write_constant(&mut self, value: Value, line: usize) {
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
    }

    pub fn add_constant(&mut self, value: Value) -> usize {
        self.constants.write(value);
        self.constants.len() - 1
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

        for run in 0..self.lines.len() {
            let line_run = self.lines.get(run).unwrap();
            if remaining < line_run.count {
                return Some(line_run.line);
            }
            remaining -= line_run.count;
        }

        None
    }
}

impl std::fmt::Debug for Chunk {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Chunk")
            .field("count", &self.code.len())
            .field("capacity", &self.code.capacity())
            .field("code", &"<raw pointer>")
            .finish()
    }
}

impl Default for Chunk {
    fn default() -> Self {
        Self::new()
    }
}
