use std::ptr::NonNull;
use tracing::debug;

use crate::{common::utils, vm::Vm};

pub struct DynamicArray<T> {
    count: usize,
    capacity: usize,
    values: Option<NonNull<T>>,
    vm: *mut Vm,
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

    pub fn new(vm: &mut Vm) -> Self {
        debug!("Dynamic array is initialized");

        Self {
            count: 0,
            capacity: 0,
            values: None,
            vm,
        }
    }

    pub fn with_capacity(vm: &mut Vm, capacity: usize) -> Self {
        if capacity == 0 {
            return Self::new(vm);
        }

        let ptr = vm.allocate_array(capacity);

        Self {
            count: 0,
            capacity,
            values: NonNull::new(ptr),
            vm,
        }
    }

    fn grow_array(
        &mut self,
        ptr: Option<NonNull<T>>,
        old_size: usize,
        new_size: usize,
    ) -> Option<NonNull<T>> {
        debug!("Grow array from {} to {}", old_size, new_size);

        let vm = unsafe { &mut *self.vm };
        let new_ptr = vm.reallocate(
            ptr.map(|p| p.as_ptr() as *mut u8)
                .unwrap_or(std::ptr::null_mut()),
            old_size * size_of::<T>(),
            new_size * size_of::<T>(),
        );

        NonNull::new(new_ptr as *mut T)
    }

    pub fn write(&mut self, value: T) {
        if self.count == usize::MAX {
            panic!("Dynamic array capacity exceeded");
        }

        if self.capacity < self.count + 1 {
            let old_capacity = self.capacity;
            self.capacity = utils::grow_capacity(old_capacity);
            self.values = self.grow_array(self.values, old_capacity, self.capacity);
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

    pub fn remove(&mut self, index: usize) -> T {
        if index >= self.count {
            panic!("Index out of bounds: {} >= {}", index, self.count);
        }

        unsafe {
            let ptr = self.values.unwrap().as_ptr();
            let value = ptr.add(index).read();

            // Сдвигаем элементы влево
            for i in index..self.count - 1 {
                let src = ptr.add(i + 1);
                let dst = ptr.add(i);
                dst.write(src.read());
            }

            self.count -= 1;
            value
        }
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

    pub fn len(&self) -> usize {
        self.count
    }

    pub fn size(&self) -> usize {
        self.capacity * size_of::<T>()
    }

    pub fn free(&mut self) {
        let size = self.size();

        if self.capacity > 0 {
            if let Some(values_ptr) = self.values {
                unsafe {
                    // Дропаем элементы
                    for i in 0..self.count {
                        values_ptr.as_ptr().add(i).drop_in_place();
                    }
                    // Освобождаем память
                    let vm = &mut *self.vm;
                    let ptr = values_ptr.as_ptr() as *mut u8;

                    vm.reallocate(ptr, size, 0);

                    self.values = None;
                    self.capacity = 0;
                    self.count = 0;
                }
            }
        }

        debug!("DynamicArray freed {} bytes, total: {}", size, unsafe {
            (*self.vm).bytes_allocated
        });
    }
}

use std::ops::{Index, IndexMut};

impl<T> Index<usize> for DynamicArray<T> {
    type Output = T;

    fn index(&self, index: usize) -> &T {
        self.get(index).expect("Index out of bounds")
    }
}

impl<T> IndexMut<usize> for DynamicArray<T> {
    fn index_mut(&mut self, index: usize) -> &mut T {
        self.get_mut(index).expect("Index out of bounds")
    }
}
