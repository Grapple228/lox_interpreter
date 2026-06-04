use crate::common::object::{objects_equal, Obj};
use std::ops::{Div, Mul, Neg, Not, Sub};

#[derive(Debug, Clone, Copy)]
pub enum Value {
    Bool(bool),
    Number(f64),
    Nil,
    Obj(*mut Obj),
}

impl Value {
    pub const fn is_truthy(&self) -> bool {
        match self {
            Value::Nil => false,
            Value::Bool(b) => *b,
            // Number (включая 0) -> true
            // String (включая "") -> true
            _ => true,
        }
    }

    pub const fn is_falsey(&self) -> bool {
        !self.is_truthy()
    }
}

impl std::fmt::Display for Value {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Value::Bool(value) => write!(f, "{}", value),
            Value::Number(num) => write!(f, "{}", num),
            Value::Nil => write!(f, "nil"),
            Value::Obj(obj) => {
                if obj.is_null() {
                    write!(f, "null")
                } else {
                    write!(f, "{}", unsafe { &**obj })
                }
            }
        }
    }
}

pub enum ValueResult {
    Success(Value),
    Error(&'static str),
}

impl ValueResult {
    pub const fn is_error(&self) -> bool {
        matches!(self, ValueResult::Error(_))
    }
}

impl From<Value> for ValueResult {
    fn from(value: Value) -> Self {
        Self::Success(value)
    }
}

impl PartialEq for Value {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Nil, Self::Nil) => true,
            (Self::Bool(b1), Self::Bool(b2)) => b1 == b2,
            (Self::Number(n1), Self::Number(n2)) => n1 == n2,
            (Self::Obj(a), Self::Obj(b)) => objects_equal(*a, *b),
            _ => false,
        }
    }
}

impl Not for Value {
    type Output = ValueResult;

    fn not(self) -> Self::Output {
        Value::Bool(self.is_falsey()).into()
    }
}

impl Neg for Value {
    type Output = ValueResult;

    fn neg(self) -> Self::Output {
        match self {
            Value::Number(value) => Value::Number(-value).into(),
            _ => ValueResult::Error("Operand must be a number."),
        }
    }
}

impl Sub for Value {
    type Output = ValueResult;

    fn sub(self, rhs: Self) -> Self::Output {
        match (self, rhs) {
            (Value::Number(a), Value::Number(b)) => Value::Number(a - b).into(),
            (_, _) => ValueResult::Error("Operands must be a numbers."),
        }
    }
}

impl Mul for Value {
    type Output = ValueResult;

    fn mul(self, rhs: Self) -> Self::Output {
        match (self, rhs) {
            (Value::Number(a), Value::Number(b)) => Value::Number(a * b).into(),
            (_, _) => ValueResult::Error("Operands must be a numbers."),
        }
    }
}

impl Div for Value {
    type Output = ValueResult;

    fn div(self, rhs: Self) -> Self::Output {
        match (self, rhs) {
            (Value::Number(_), Value::Number(0.0)) => ValueResult::Error("Division by zero"),
            (Value::Number(a), Value::Number(b)) => Value::Number(a / b).into(),
            (_, _) => ValueResult::Error("Operands must be a numbers."),
        }
    }
}
