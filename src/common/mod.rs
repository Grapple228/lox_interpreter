mod chunk;
mod dynamic_array;
mod value;

pub use chunk::Chunk;
pub use dynamic_array::DynamicArray;
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
    }
}
