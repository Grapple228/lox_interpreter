use std::{
    fmt::write,
    ops::{Add, Div, Mul, Neg, Sub},
    ptr::NonNull,
};

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Value {
    Number(f64),
    Nil,
}

impl std::fmt::Display for Value {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Value::Number(num) => write!(f, "{}", num),
            Value::Nil => write!(f, "nil"),
        }
    }
}

impl Neg for Value {
    type Output = Value;

    fn neg(self) -> Self::Output {
        match self {
            Value::Number(num) => Value::Number(-num),
            Value::Nil => panic!("Tried to negate Nil"),
        }
    }
}

impl Sub for Value {
    type Output = Value;

    fn sub(self, rhs: Self) -> Self::Output {
        match (self, rhs) {
            (Value::Number(a), Value::Number(b)) => Value::Number(a - b),
            (_, _) => panic!("Subtraction is only available for number operands"),
        }
    }
}

impl Add for Value {
    type Output = Value;

    fn add(self, rhs: Self) -> Self::Output {
        match (self, rhs) {
            (Value::Number(a), Value::Number(b)) => Value::Number(a + b),
            (_, _) => panic!("Addition are only available for number operands"),
        }
    }
}

impl Mul for Value {
    type Output = Value;

    fn mul(self, rhs: Self) -> Self::Output {
        match (self, rhs) {
            (Value::Number(a), Value::Number(b)) => Value::Number(a * b),
            (_, _) => panic!("Multiply are only available for number operands"),
        }
    }
}

impl Div for Value {
    type Output = Value;

    fn div(self, rhs: Self) -> Self::Output {
        match (self, rhs) {
            (Value::Number(_), Value::Number(0.0)) => panic!("Division by zero"),
            (Value::Number(a), Value::Number(b)) => Value::Number(a / b),
            (_, _) => panic!("Division is only available for number operands"),
        }
    }
}
