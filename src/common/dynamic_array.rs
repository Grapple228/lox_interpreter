use std::{
    alloc::{alloc, dealloc, realloc, Layout},
    ptr::NonNull,
};
use tracing::debug;

use crate::common::utils;

pub struct DynamicArray<T> {
    count: usize,
    capacity: usize,
    values: Option<NonNull<T>>,
    _marker: std::marker::PhantomData<T>,
}

impl<T> DynamicArray<T> {
    pub fn as_ptr(&self) -> *const T {
        match self.values {
            Some(ptr) => ptr.as_ptr(),
            None => std::ptr::null(),
        }
    }

    pub fn as_mut_ptr(&self) -> *mut T {
        match self.values {
            Some(ptr) => ptr.as_ptr(),
            None => std::ptr::null_mut(),
        }
    }

    pub fn new() -> Self {
        debug!("Dynamic array is initialized");

        Self {
            count: 0,
            capacity: 0,
            values: None,
            _marker: std::marker::PhantomData,
        }
    }

    pub fn with_capacity(capacity: usize) -> Self {
        if capacity == 0 {
            return Self::new();
        }

        let layout = Layout::array::<T>(capacity).unwrap();
        let ptr = unsafe { alloc(layout) };

        Self {
            count: 0,
            capacity,
            values: NonNull::new(ptr as *mut T),
            _marker: std::marker::PhantomData,
        }
    }

    fn grow_array(ptr: Option<NonNull<T>>, old_size: usize, new_size: usize) -> Option<NonNull<T>> {
        debug!("Grow array from {} to {}", old_size, new_size);

        if new_size == 0 {
            return None;
        }

        let new_layout = Layout::array::<T>(new_size).unwrap();

        match ptr {
            Some(old_ptr) => {
                if old_size == 0 {
                    let new_ptr = unsafe { alloc(new_layout) };
                    NonNull::new(new_ptr as *mut T)
                } else {
                    let old_layout = Layout::array::<T>(old_size).unwrap();
                    let ptr_u8 = old_ptr.as_ptr() as *mut u8;
                    let new_ptr_u8 = unsafe { realloc(ptr_u8, old_layout, new_layout.size()) };
                    if new_ptr_u8.is_null() {
                        panic!("Memory reallocation failed");
                    }
                    NonNull::new(new_ptr_u8 as *mut T)
                }
            }
            None => {
                let new_ptr = unsafe { alloc(new_layout) };
                if new_ptr.is_null() {
                    panic!("Memory allocation failed");
                }
                NonNull::new(new_ptr as *mut T)
            }
        }
    }

    pub fn write(&mut self, value: T) {
        if self.count == usize::MAX {
            panic!("Dynamic array capacity exceeded");
        }

        if self.capacity < self.count + 1 {
            let old_capacity = self.capacity;
            self.capacity = utils::grow_capacity(old_capacity);
            self.values = Self::grow_array(self.values, old_capacity, self.capacity);
        }

        let Some(values_ptr) = self.values.as_mut() else {
            panic!("values should be initialized at this point");
        };

        unsafe {
            values_ptr.as_ptr().add(self.count).write(value);
        }
        self.count += 1;
    }

    pub fn get(&self, index: usize) -> Option<&T> {
        if index < self.count {
            unsafe { self.values.unwrap().as_ptr().add(index).as_ref() }
        } else {
            None
        }
    }

    pub fn get_mut(&mut self, index: usize) -> Option<&mut T> {
        if index < self.count {
            unsafe { self.values.unwrap().as_ptr().add(index).as_mut() }
        } else {
            None
        }
    }

    pub fn count(&self) -> usize {
        self.count
    }

    pub fn is_empty(&self) -> bool {
        self.count == 0
    }

    pub fn capacity(&self) -> usize {
        self.capacity
    }

    // Добавьте метод для итерации
    pub fn iter(&self) -> impl Iterator<Item = &T> {
        (0..self.count).filter_map(move |i| self.get(i))
    }

    // Добавьте метод для преобразования в Vec
    pub fn to_vec(&self) -> Vec<T>
    where
        T: Copy,
    {
        let mut result = Vec::with_capacity(self.count);
        for i in 0..self.count {
            if let Some(&value) = self.get(i) {
                result.push(value);
            }
        }
        result
    }

    pub fn set(&mut self, offset: usize, value: T) {
        if offset >= self.count {
            panic!("Index out of bounds: {} >= {}", offset, self.count);
        }

        let Some(values_ptr) = self.values.as_mut() else {
            panic!("Dynamic array is not initialized");
        };

        unsafe {
            values_ptr.as_ptr().add(offset).write(value);
        }
    }
}

impl<T> Drop for DynamicArray<T> {
    fn drop(&mut self) {
        if let Some(ptr) = self.values {
            if self.capacity > 0 {
                // Сначала вызываем drop для каждого элемента
                for i in 0..self.count {
                    unsafe {
                        ptr.as_ptr().add(i).drop_in_place();
                    }
                }

                let layout = Layout::array::<T>(self.capacity).unwrap();
                unsafe {
                    dealloc(ptr.as_ptr() as *mut u8, layout);
                    debug!(
                        "Dynamic array memory deallocated (capacity: {})",
                        self.capacity
                    );
                }
            }
        }
    }
}

impl<T> Default for DynamicArray<T> {
    fn default() -> Self {
        Self::new()
    }
}
