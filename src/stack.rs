use std::{mem::MaybeUninit, ptr::NonNull};

const STACK_MAX: usize = 256;

pub struct Stack<T> {
    values: Box<[MaybeUninit<T>; STACK_MAX]>, // На куче, чтобы не двигалась
    top: NonNull<T>,
}

impl<T> Stack<T> {
    pub fn new() -> Self {
        // Выделяем массив на куче - он не переместится
        let values = Box::new(std::array::from_fn(|_| MaybeUninit::uninit()));
        let ptr = values.as_ptr() as *mut T;

        Self {
            values,
            top: NonNull::new(ptr).expect("Failed to get top of array"),
        }
    }

    pub fn get(&self, index: usize) -> &T {
        unsafe {
            let start = self.values.as_ptr() as *const T;
            let ptr = start.add(index);
            &*ptr
        }
    }

    pub fn set(&mut self, index: usize, value: T) {
        unsafe {
            let start = self.values.as_ptr() as *mut T;
            let ptr = start.add(index);
            ptr.write(value);
        }
    }

    pub fn push(&mut self, value: T) {
        unsafe {
            let start = self.values.as_ptr() as *const T;
            let current_index = self.top.as_ptr().offset_from(start) as isize;

            if current_index < 0 || current_index as usize >= STACK_MAX {
                panic!("Stack overflow at index {}", current_index);
            }

            self.top.as_ptr().write(value);
            self.top = NonNull::new_unchecked(self.top.as_ptr().add(1));
        }
    }

    pub fn pop(&mut self) -> T {
        unsafe {
            let start = self.values.as_ptr() as *const T;
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
            let start = self.values.as_ptr() as *const T;
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

    pub fn peek(&self, distance: usize) -> &T {
        unsafe {
            let ptr = self.top.as_ptr().sub(distance + 1);
            &*ptr
        }
    }

    pub fn peek_mut(&mut self, distance: usize) -> &mut T {
        unsafe {
            let ptr = self.top.as_ptr().sub(distance + 1);
            &mut *ptr
        }
    }

    pub fn reset(&mut self) {
        unsafe {
            // Дропаем все значения
            for i in 0..self.len() {
                let ptr = self.values.as_ptr() as *mut T;
                ptr.add(i).drop_in_place();
            }
            // Сбрасываем top на начало
            self.top = NonNull::new(self.values.as_ptr() as *mut T).unwrap();
        }
    }
}

impl<T: std::fmt::Display> Stack<T> {
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
                let ptr = self.values.as_ptr() as *const T;
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
            let start = self.values.as_ptr() as *const T;
            let top_index = self.top.as_ptr().offset_from(start);
            write!(&mut output, " (top index: {})", top_index).unwrap();
        }

        println!("{}", output);
    }
}

impl<T> Drop for Stack<T> {
    fn drop(&mut self) {
        self.reset();
    }
}
