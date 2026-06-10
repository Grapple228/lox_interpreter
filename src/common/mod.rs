pub mod utils;

mod chunk;
mod dynamic_array;
mod stack;
mod table;
mod value;

pub use chunk::Chunk;
pub use dynamic_array::DynamicArray;
pub use stack::Stack;
pub use table::{Entry, Table};
pub use value::{Value, ValueResult};

use crate::define_enum_with_values;

#[derive(Debug, Clone, Copy)]
struct LineRun {
    line: usize,
    count: usize, // сколько инструкций на этой строке
}

define_enum_with_values! {
    #[allow(non_camel_case_types)]
    pub enum OpCode {
        OP_RETURN = 0 => "OP_RETURN",
        OP_CONSTANT = 1 => "OP_CONSTANT",
        OP_CONSTANT_LONG = 2 => "OP_CONSTANT_LONG",
        OP_NEGATE = 3 => "OP_NEGATE",
        OP_ADD = 4 => "OP_ADD",
        OP_SUBSTRACT = 5 => "OP_SUBSTRACT",
        OP_MULTIPLY = 6 => "OP_MULTIPLY",
        OP_DIVIDE = 7 => "OP_DIVIDE",
        OP_NIL = 8 => "OP_NIL",
        OP_TRUE = 9 => "OP_TRUE",
        OP_FALSE = 10 => "OP_FALSE",
        OP_NOT = 11 => "OP_NOT",
        OP_EQUAL = 12 => "OP_EQUAL",
        OP_GREATER = 13 => "OP_GREATER",
        OP_LESS = 14 => "OP_LESS",
        OP_PRINT = 15 => "OP_PRINT",
        OP_POP = 16 => "OP_POP",
        OP_DEFINE_GLOBAL = 17 => "OP_DEFINE_GLOBAL",
        OP_GET_GLOBAL = 18 => "OP_GET_GLOBAL",
        OP_SET_GLOBAL = 19 => "OP_SET_GLOBAL",
        OP_GET_LOCAL = 20 => "OP_GET_LOCAL",
        OP_SET_LOCAL = 21 => "OP_SET_LOCAL",
        OP_JUMP_IF_FALSE = 22 => "OP_JUMP_IF_FALSE",
        OP_JUMP = 23 => "OP_JUMP",
        OP_LOOP = 24 => "OP_LOOP",
        OP_MOD = 25 => "OP_MOD",
        OP_CALL = 26 => "OP_CALL",
        OP_CLOSURE = 27 => "OP_CLOSURE",
        OP_GET_UPVALUE = 28 => "OP_GET_UPVALUE",
        OP_SET_UPVALUE = 29 => "OP_SET_UPVALUE",
        OP_CLOSE_UPVALUE = 30 => "OP_CLOSE_UPVALUE",
        OP_CLASS = 31 => "OP_CLASS",
        OP_SET_PROPERTY = 32 => "OP_SET_PROPERTY",
        OP_GET_PROPERTY = 33 => "OP_GET_PROPERTY",
    }
}
