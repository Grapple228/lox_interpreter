use crate::define_enum_with_values;

define_enum_with_values! {
    #[allow(non_camel_case_types)]
    #[derive(PartialOrd, Ord)]
    pub enum Precedence {
        NONE = 0 => "NONE",
        ASSIGNMENT = 1 => "ASSIGNMENT", // =
        OR = 2 => "OR",                 // or
        AND = 3 => "AND",               // and
        EQUALITY = 4 => "EQUALITY",     // == !=
        COMPARISON = 5 => "COMPARISON", // < > <= >=
        TERM = 6 => "TERM",             // + -
        FACTOR = 7 => "FACTOR",         // * /
        UNARY = 8 => "UNARY",           // ! -
        CALL = 9 => "CALL",             // . ()
        PRIMARY = 10 => "PRIMARY",
    }
}

impl Precedence {
    pub fn next(self) -> Self {
        match self {
            Precedence::NONE => Precedence::ASSIGNMENT,
            Precedence::ASSIGNMENT => Precedence::OR,
            Precedence::OR => Precedence::AND,
            Precedence::AND => Precedence::EQUALITY,
            Precedence::EQUALITY => Precedence::COMPARISON,
            Precedence::COMPARISON => Precedence::TERM,
            Precedence::TERM => Precedence::FACTOR,
            Precedence::FACTOR => Precedence::UNARY,
            Precedence::UNARY => Precedence::CALL,
            Precedence::CALL => Precedence::PRIMARY,
            Precedence::PRIMARY => Precedence::PRIMARY,
        }
    }
}
