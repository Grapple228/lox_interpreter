use crate::{common::Value, object::NativeResult};

pub fn clock(_arg_count: usize, _args: *mut Value) -> NativeResult {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs_f64();

    NativeResult::Success(Value::Number(now))
}

pub fn square(arg_count: usize, args: *mut Value) -> NativeResult {
    if arg_count != 1 {
        return NativeResult::Error("square() expects 1 arguments (num).".to_string());
    }

    unsafe {
        match *args {
            Value::Number(n) => NativeResult::Success(Value::Number(n * n)),
            _ => NativeResult::Error("square() argument must be a number.".to_string()),
        }
    }
}

pub fn random(arg_count: usize, args: *mut Value) -> NativeResult {
    unsafe {
        if arg_count != 2 {
            return NativeResult::Error("random() expects 2 arguments (min, max).".to_string());
        }

        let min = match *args {
            Value::Number(n) => n,
            _ => return NativeResult::Error("random() min argument must be a number.".to_string()),
        };

        let max = match *args.add(1) {
            Value::Number(n) => n,
            _ => return NativeResult::Error("random() max argument must be a number.".to_string()),
        };

        if min > max {
            return NativeResult::Error(
                "random() min must be less than or equal to max.".to_string(),
            );
        }

        let value = rand::random_range(min..=max);

        NativeResult::Success(Value::Number(value))
    }
}
