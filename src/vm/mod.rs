// ./src/vm.rs

use tracing::warn;

use crate::{
    common::{OpCode, Stack, Value},
    compiler::{CallFrame, Callee, Compiler},
    object::{FunctionType, NativeFn, NativeResult, ObjClosure, ObjNative, ObjUpValue},
    table::Table,
    Obj, ObjFunction, ObjString, ObjType,
};

mod natives;

pub const FRAMES_MAX: usize = 64;
pub const STACK_MAX: usize = FRAMES_MAX * 256;

pub type ValueStack = Stack<STACK_MAX, Value>;
pub type Frames = [CallFrame; FRAMES_MAX];

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
    pub objects: *mut Obj,
    pub bytes_allocated: usize,

    pub strings: Table,
    pub globals: Table,

    open_upvalues: *mut ObjUpValue,

    frame_count: usize,
}

impl Vm {
    pub fn new() -> Self {
        Self {
            objects: std::ptr::null_mut(),
            frame_count: 0,
            bytes_allocated: 0,
            strings: Table::new(),
            globals: Table::new(),

            open_upvalues: std::ptr::null_mut(),
        }
    }

    pub fn init(&mut self, stack: &mut ValueStack) {
        self.init_natives(stack);
    }

    fn init_natives(&mut self, stack: &mut ValueStack) {
        self.define_native(stack, "clock", 0, natives::clock);
        self.define_native(stack, "random", 2, natives::random);
        self.define_native(stack, "square", 1, natives::square);
    }

    fn runtime_error(&self, frames: &Frames, stack: &mut ValueStack, message: &str) {
        eprintln!("{}", message);

        for i in (0..self.frame_count).rev() {
            let frame = &frames[i];
            unsafe {
                let function_ptr = frame.function();
                if function_ptr.is_null() {
                    eprintln!("[unknown location]");
                    continue;
                }

                let function = &*frame.function();

                let chunk = function.chunk();
                let instruction = frame.ip.offset_from((*chunk).get_code_ptr()) as usize - 1;
                let line = (*chunk).get_line(instruction).unwrap_or(0);

                eprint!("[line {}] in ", line);
                let name = function.name();
                if name.is_null() {
                    eprintln!("script");
                } else {
                    eprintln!("{}()", (*name).as_str());
                }
            }
        }

        stack.reset();
    }

    fn read_byte(frame: &mut CallFrame) -> u8 {
        unsafe {
            let byte = *frame.ip;
            frame.ip = frame.ip.add(1);
            byte
        }
    }

    fn read_u16(frame: &mut CallFrame) -> u16 {
        unsafe {
            let b0 = *frame.ip as u16;
            let b1 = *frame.ip.add(1) as u16;
            frame.ip = frame.ip.add(2);
            (b0 << 8) | b1
        }
    }

    fn read_constant(frame: &mut CallFrame) -> Value {
        let idx = Self::read_byte(frame) as usize;
        unsafe {
            let function = frame.function();
            let chunk = (*function).chunk();
            *(*chunk).constants.get(idx).expect("Constant not found")
        }
    }

    fn read_constant_long(frame: &mut CallFrame) -> Value {
        let b0 = Self::read_byte(frame) as usize;
        let b1 = Self::read_byte(frame) as usize;
        let b2 = Self::read_byte(frame) as usize;
        let idx = b0 | (b1 << 8) | (b2 << 16);
        unsafe {
            let function = frame.function();
            let chunk = (*function).chunk();
            *(*chunk).constants.get(idx).expect("Constant not found")
        }
    }

    fn read_string(&mut self, frame: &mut CallFrame) -> Option<*mut ObjString> {
        let name = Self::read_constant(frame);

        match name {
            Value::Obj(ptr) if unsafe { (*ptr).typ() == ObjType::String } => {
                Some(ptr as *mut ObjString)
            }
            _ => None,
        }
    }

    fn add_strings(&mut self, a: *mut Obj, b: *mut Obj) -> Option<Value> {
        unsafe {
            if a.is_null() || b.is_null() {
                return None;
            }
            if (*a).typ() != ObjType::String || (*b).typ() != ObjType::String {
                return None;
            }
            let a_str = a as *mut ObjString;
            let b_str = b as *mut ObjString;
            let result = ObjString::concatenate(self, a_str, b_str);
            Some(Value::Obj(result as *mut Obj))
        }
    }

    fn call_function(
        &mut self,
        frames: &mut Frames,
        stack: &mut ValueStack,
        function: *mut ObjFunction,
        arg_count: usize,
    ) -> bool {
        unsafe {
            if self.frame_count == FRAMES_MAX {
                self.runtime_error(frames, stack, "Stack overflow.");
                return false;
            }

            let arity = (*function).arity();
            if arg_count != arity {
                self.runtime_error(
                    frames,
                    stack,
                    &format!("Expected {} arguments but got {}.", arity, arg_count),
                );
                return false;
            }

            let frame = &mut frames[self.frame_count];
            self.frame_count += 1;

            frame.callee = Callee::Function(function);
            frame.ip = (*(*function).chunk()).get_code_mut_ptr();

            let stack_top = stack.len();
            frame.slots = stack.get_ptr(stack_top - arg_count - 1) as *mut Value;

            true
        }
    }

    fn call_closure(
        &mut self,
        frames: &mut Frames,
        stack: &mut ValueStack,
        closure: *mut ObjClosure,
        arg_count: usize,
    ) -> bool {
        if !self.call_function(frames, stack, unsafe { (*closure).function }, arg_count) {
            return false;
        }
        let frame = &mut frames[self.frame_count - 1];
        frame.callee = Callee::Closure(closure);
        true
    }

    fn call_value(
        &mut self,
        frames: &mut Frames,
        stack: &mut ValueStack,
        callee: Value,
        arg_count: usize,
    ) -> bool {
        if callee.is_obj() {
            match unsafe { (*callee.as_obj()).typ() } {
                ObjType::Closure => {
                    return self.call_closure(frames, stack, callee.as_closure(), arg_count);
                }
                ObjType::Function => {
                    return self.call_function(frames, stack, callee.as_function(), arg_count);
                }
                ObjType::Native => {
                    let native = unsafe { &*(callee.as_obj() as *mut ObjNative) };

                    if arg_count != native.arity() {
                        self.runtime_error(
                            frames,
                            stack,
                            &format!(
                                "Expected {} arguments but got {}.",
                                native.arity(),
                                arg_count
                            ),
                        );
                        return false;
                    }

                    let result = native.function()(
                        arg_count,
                        stack.get_ptr(stack.len() - arg_count) as *mut Value,
                    );

                    // Убираем аргументы и функцию со стека
                    for _ in 0..arg_count + 1 {
                        stack.pop();
                    }

                    match result {
                        NativeResult::Success(value) => {
                            stack.push(value);
                        }
                        NativeResult::Error(e) => {
                            self.runtime_error(frames, stack, &e);
                            return false;
                        }
                    }

                    return true;
                }
                _ => {}
            }
        }

        self.runtime_error(frames, stack, "Can only call functions as classes.");
        false
    }

    fn define_native(
        &mut self,
        stack: &mut ValueStack,
        name: &'static str,
        arity: usize,
        function: NativeFn,
    ) {
        let name_obj = ObjString::copy(self, name.as_ptr(), name.len());
        stack.push(Value::Obj(name_obj as *mut Obj));

        let native = ObjNative::allocate(self, arity, function);
        stack.push(Value::Obj(native as *mut Obj));

        let key = stack.get(0).as_string();
        let value = *stack.get(1);
        self.globals.set(key, value);

        stack.pop();
        stack.pop();
    }

    fn capture_upvalue(&mut self, local: *mut Value) -> *mut ObjUpValue {
        let mut prev_upvalue = std::ptr::null_mut();
        let mut upvalue = self.open_upvalues;

        unsafe {
            while !upvalue.is_null() && (*upvalue).location > local {
                prev_upvalue = upvalue;
                upvalue = (*upvalue).next;
            }

            if !upvalue.is_null() && (*upvalue).location == local {
                return upvalue;
            }
        }

        let created_upvalue = ObjUpValue::allocate(self, local);

        if prev_upvalue.is_null() {
            self.open_upvalues = created_upvalue;
        } else {
            unsafe {
                (*prev_upvalue).next = created_upvalue;
            }
        }

        created_upvalue
    }

    fn close_upvalues(&mut self, last: *mut Value) {
        unsafe {
            while !self.open_upvalues.is_null() && (*self.open_upvalues).location >= last {
                let upvalue = self.open_upvalues;
                (*upvalue).closed = *(*upvalue).location;
                (*upvalue).location = &mut (*upvalue).closed;
                self.open_upvalues = (*upvalue).next;
            }
        }
    }

    fn run(&mut self, frames: &mut Frames, stack: &mut ValueStack) -> InterpretResult {
        let mut frame_index = self.frame_count - 1;

        loop {
            if cfg!(debug_assertions) {
                stack.debug_content();
                unsafe {
                    let frame = &mut frames[frame_index];

                    let function_ptr = frame.function();
                    if !function_ptr.is_null() {
                        let chunk = &*(*function_ptr).chunk();
                        let offset = frame.ip.offset_from(chunk.get_code_ptr()) as usize;
                        chunk.disassemble_unstruction(offset);
                    }
                }
            }

            let Some(op) = OpCode::from_byte(Self::read_byte(&mut frames[frame_index])) else {
                panic!("Invalid op code");
            };

            match op {
                OpCode::OP_JUMP_IF_FALSE => {
                    let frame = &mut frames[frame_index];

                    let offset = Self::read_u16(frame);
                    if stack.peek(0).is_falsey() {
                        frame.ip = unsafe { frame.ip.add(offset as usize) };
                    }
                }
                OpCode::OP_JUMP => {
                    let frame = &mut frames[frame_index];

                    let offset = Self::read_u16(frame);
                    frame.ip = unsafe { frame.ip.add(offset as usize) };
                }

                OpCode::OP_LOOP => {
                    let frame = &mut frames[frame_index];

                    let offset = Self::read_u16(frame);
                    frame.ip = unsafe { frame.ip.sub(offset as usize) };
                }

                OpCode::OP_POP => {
                    _ = stack.pop();
                }

                OpCode::OP_GET_UPVALUE => {
                    let frame = &mut frames[frame_index];

                    let slot = Self::read_byte(frame) as usize;
                    unsafe {
                        let closure = frame.closure();
                        let value = (**(*closure).upvalues.add(slot)).location;
                        stack.push(*value);
                    }
                }

                OpCode::OP_SET_UPVALUE => {
                    let frame = &mut frames[frame_index];

                    let slot = Self::read_byte(frame) as usize;
                    unsafe {
                        let closure = frame.closure();
                        let upvalue = *(*closure).upvalues.add(slot);
                        *(*upvalue).location = *stack.peek(0);
                    }
                }

                OpCode::OP_CLOSE_UPVALUE => {
                    self.close_upvalues(stack.get_ptr(stack.len() - 1) as *mut Value);
                    stack.pop();
                }

                OpCode::OP_GET_LOCAL => {
                    let frame = &mut frames[frame_index];

                    let slot = Self::read_byte(frame) as usize;
                    unsafe {
                        let value = *frame.slots.add(slot);
                        stack.push(value);
                    }
                }

                OpCode::OP_SET_LOCAL => {
                    let frame = &mut frames[frame_index];

                    let slot = Self::read_byte(frame) as usize;
                    let value = *stack.peek(0);
                    unsafe {
                        *frame.slots.add(slot) = value;
                    }
                }

                OpCode::OP_SET_GLOBAL => {
                    let frame = &mut frames[frame_index];

                    let Some(name_str) = self.read_string(frame) else {
                        self.runtime_error(frames, stack, "Global variable name must be a string.");
                        return InterpretResult::RuntimeError;
                    };

                    let value = *stack.peek(0);
                    self.globals.set(name_str, value);
                }

                OpCode::OP_GET_GLOBAL => {
                    let frame = &mut frames[frame_index];

                    let Some(name_str) = self.read_string(frame) else {
                        self.runtime_error(frames, stack, "Global variable name must be a string.");
                        return InterpretResult::RuntimeError;
                    };

                    let mut value = Value::Nil;
                    if !self.globals.get(name_str, &mut value) {
                        self.runtime_error(frames, stack, "Undefined variable.");
                        return InterpretResult::RuntimeError;
                    }

                    stack.push(value);
                }

                OpCode::OP_DEFINE_GLOBAL => {
                    let frame = &mut frames[frame_index];

                    let Some(name_str) = self.read_string(frame) else {
                        self.runtime_error(frames, stack, "Global variable name must be a string.");
                        return InterpretResult::RuntimeError;
                    };

                    let value = *stack.peek(0);
                    self.globals.set(name_str, value);
                    stack.pop();
                }

                OpCode::OP_NIL => stack.push(Value::Nil),
                OpCode::OP_TRUE => stack.push(Value::Bool(true)),
                OpCode::OP_FALSE => stack.push(Value::Bool(false)),

                OpCode::OP_PRINT => {
                    println!("{}", stack.pop());
                }

                OpCode::OP_RETURN => {
                    let frame = &mut frames[frame_index];

                    let result = stack.pop();

                    self.close_upvalues(frame.slots);

                    self.frame_count -= 1;

                    if self.frame_count == 0 {
                        return InterpretResult::Ok;
                    }

                    stack.set_top(frame.slots as *mut Value);
                    stack.push(result);

                    frame_index = self.frame_count - 1;
                }

                OpCode::OP_CLOSURE => {
                    let function = Self::read_constant(&mut frames[frame_index]).as_function();
                    let closure = ObjClosure::allocate(self, function);
                    stack.push(Value::Obj(closure as *mut Obj));

                    unsafe {
                        for i in 0..(*closure).upvalue_count {
                            let (is_local, index) = {
                                let frame = &mut frames[frame_index];

                                let is_local = Self::read_byte(frame);
                                let index = Self::read_byte(frame);

                                (is_local, index)
                            };

                            if is_local == 1 {
                                let frame = &mut frames[frame_index];

                                let upvalue = self.capture_upvalue(frame.slots.add(index as usize));
                                *(*closure).upvalues.add(i) = upvalue;
                            } else {
                                let enclosing_frame = &frames[self.frame_count - 2];
                                let upvalue = enclosing_frame.enclosing_upvalue(index as usize);
                                *(*closure).upvalues.add(i) = upvalue;
                            }
                        }
                    }
                }

                OpCode::OP_CONSTANT => {
                    let frame = &mut frames[frame_index];

                    let constant = Self::read_constant(frame);
                    stack.push(constant);
                }

                OpCode::OP_CONSTANT_LONG => {
                    let frame = &mut frames[frame_index];

                    let constant = Self::read_constant_long(frame);
                    stack.push(constant);
                }

                OpCode::OP_NOT => {
                    let value = stack.pop();
                    stack.push(Value::Bool(value.is_falsey()));
                }
                OpCode::OP_NEGATE => {
                    let value = stack.pop();
                    match value {
                        Value::Number(n) => stack.push(Value::Number(-n)),
                        _ => {
                            self.runtime_error(frames, stack, "Operand must be a number.");
                            return InterpretResult::RuntimeError;
                        }
                    }
                }

                OpCode::OP_ADD => {
                    let right = stack.pop();
                    let left = stack.pop();

                    match (left, right) {
                        (Value::Number(a), Value::Number(b)) => stack.push(Value::Number(a + b)),
                        (Value::Obj(a), Value::Obj(b)) => match self.add_strings(a, b) {
                            Some(v) => stack.push(v),
                            None => {
                                self.runtime_error(
                                    frames,
                                    stack,
                                    "Operands must be numbers or strings.",
                                );
                                return InterpretResult::RuntimeError;
                            }
                        },
                        _ => {
                            self.runtime_error(
                                frames,
                                stack,
                                "Operands must be numbers or strings.",
                            );
                            return InterpretResult::RuntimeError;
                        }
                    }
                }
                OpCode::OP_SUBSTRACT => {
                    let right = stack.pop();
                    let left = stack.pop();
                    match (left, right) {
                        (Value::Number(a), Value::Number(b)) => stack.push(Value::Number(a - b)),
                        _ => {
                            self.runtime_error(frames, stack, "Operands must be numbers.");
                            return InterpretResult::RuntimeError;
                        }
                    }
                }
                OpCode::OP_MOD => {
                    let right = stack.pop();
                    let left = stack.pop();
                    match (left, right) {
                        (Value::Number(a), Value::Number(b)) => stack.push(Value::Number(a % b)),
                        _ => {
                            self.runtime_error(frames, stack, "Operands must be numbers.");
                            return InterpretResult::RuntimeError;
                        }
                    }
                }
                OpCode::OP_MULTIPLY => {
                    let right = stack.pop();
                    let left = stack.pop();
                    match (left, right) {
                        (Value::Number(a), Value::Number(b)) => stack.push(Value::Number(a * b)),
                        _ => {
                            self.runtime_error(frames, stack, "Operands must be numbers.");
                            return InterpretResult::RuntimeError;
                        }
                    }
                }
                OpCode::OP_DIVIDE => {
                    let right = stack.pop();
                    let left = stack.pop();
                    match (left, right) {
                        (Value::Number(a), Value::Number(b)) => stack.push(Value::Number(a / b)),
                        _ => {
                            self.runtime_error(frames, stack, "Operands must be numbers.");
                            return InterpretResult::RuntimeError;
                        }
                    }
                }
                OpCode::OP_EQUAL => {
                    let right = stack.pop();
                    let left = stack.pop();
                    stack.push(Value::Bool(left == right));
                }
                OpCode::OP_GREATER => {
                    let right = stack.pop();
                    let left = stack.pop();
                    match (left, right) {
                        (Value::Number(a), Value::Number(b)) => stack.push(Value::Bool(a > b)),
                        _ => {
                            self.runtime_error(frames, stack, "Operands must be numbers.");
                            return InterpretResult::RuntimeError;
                        }
                    }
                }
                OpCode::OP_LESS => {
                    let right = stack.pop();
                    let left = stack.pop();
                    match (left, right) {
                        (Value::Number(a), Value::Number(b)) => stack.push(Value::Bool(a < b)),
                        _ => {
                            self.runtime_error(frames, stack, "Operands must be numbers.");
                            return InterpretResult::RuntimeError;
                        }
                    }
                }
                OpCode::OP_CALL => {
                    let frame = &mut frames[frame_index];

                    let arg_count = Self::read_byte(frame) as usize;
                    let callee = *stack.peek(arg_count);

                    if !self.call_value(frames, stack, callee, arg_count) {
                        return InterpretResult::RuntimeError;
                    }

                    frame_index = self.frame_count - 1;
                }
            }
        }
    }

    pub fn interpret(&mut self, stack: &mut ValueStack, source: *const u8) -> InterpretResult {
        let mut compiler =
            Compiler::new(self as *mut Vm, FunctionType::Script, std::ptr::null_mut());

        let function = compiler.compile(source);
        if function.is_null() {
            return InterpretResult::CompileError;
        };

        let mut frames = std::array::from_fn(|_| CallFrame::null());
        self.frame_count = 0;

        stack.push(Value::Obj(function as *mut Obj));

        self.call_function(&mut frames, stack, function, 0);

        self.run(&mut frames, stack)
    }
}

impl Drop for Vm {
    fn drop(&mut self) {
        self.free_objects();
    }
}
