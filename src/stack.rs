use crate::common::Value;
use std::{mem::MaybeUninit, ptr::NonNull};
use tracing::debug;

const STACK_MAX: usize = 256;

pub struct Stack {
    values: Box<[MaybeUninit<Value>; STACK_MAX]>, // На куче, чтобы не двигалась
    top: NonNull<Value>,
}

impl Stack {
    pub fn new() -> Self {
        // Выделяем массив на куче - он не переместится
        let values = Box::new(std::array::from_fn(|_| MaybeUninit::uninit()));
        let ptr = values.as_ptr() as *mut Value;

        Self {
            values,
            top: NonNull::new(ptr).expect("Failed to get top of array"),
        }
    }

    pub fn push(&mut self, value: Value) {
        unsafe {
            let start = self.values.as_ptr() as *const Value;
            let current_index = self.top.as_ptr().offset_from(start) as isize;

            if current_index < 0 || current_index as usize >= STACK_MAX {
                panic!("Stack overflow at index {}", current_index);
            }

            self.top.as_ptr().write(value);
            self.top = NonNull::new_unchecked(self.top.as_ptr().add(1));
        }
    }

    pub fn pop(&mut self) -> Value {
        unsafe {
            let start = self.values.as_ptr() as *const Value;
            let current_index = self.top.as_ptr().offset_from(start) as isize;

            if current_index <= 0 {
                panic!("Stack underflow");
            }

            self.top = NonNull::new_unchecked(self.top.as_ptr().sub(1));
            self.top.as_ptr().read()
        }
    }

    pub fn len(&self) -> usize {
        unsafe {
            let start = self.values.as_ptr() as *const Value;
            let offset = self.top.as_ptr().offset_from(start);
            if offset < 0 {
                0
            } else {
                offset as usize
            }
        }
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    pub fn peek(&self, distance: usize) -> &Value {
        unsafe {
            let ptr = self.top.as_ptr().sub(distance + 1);
            &*ptr
        }
    }

    pub fn reset(&mut self) {
        unsafe {
            // Дропаем все значения
            for i in 0..self.len() {
                let ptr = self.values.as_ptr() as *mut Value;
                ptr.add(i).drop_in_place();
            }
            // Сбрасываем top на начало
            self.top = NonNull::new(self.values.as_ptr() as *mut Value).unwrap();
        }
    }

    pub fn debug_content(&self) {
        use std::fmt::Write;

        let len = self.len();
        if len == 0 {
            println!("Stack: [empty]");
            return;
        }

        let mut output = String::new();
        write!(&mut output, "Stack ({} items): [", len).unwrap();

        for i in 0..len {
            unsafe {
                let ptr = self.values.as_ptr() as *const Value;
                let value = &*ptr.add(i);

                if i > 0 {
                    write!(&mut output, ", ").unwrap();
                }
                write!(&mut output, "{}", value).unwrap();
            }
        }

        write!(&mut output, "]").unwrap();

        // Показываем top указатель для отладки
        unsafe {
            let start = self.values.as_ptr() as *const Value;
            let top_index = self.top.as_ptr().offset_from(start);
            write!(&mut output, " (top index: {})", top_index).unwrap();
        }

        println!("{}", output);
    }
}

impl Drop for Stack {
    fn drop(&mut self) {
        self.reset();
    }
}
