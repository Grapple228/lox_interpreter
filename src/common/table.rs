use std::alloc::{alloc, dealloc, Layout};

use tracing::debug;

use crate::{
    common::{utils, Value},
    ObjString,
};

#[repr(C)]
pub struct Table {
    count: usize,
    capacity: usize,
    entries: *mut Entry,
}

#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct Entry {
    key: *mut ObjString,
    value: Value,
}

impl Entry {
    pub fn key(&self) -> *mut ObjString {
        self.key
    }

    pub fn value(&self) -> Value {
        self.value
    }
}

impl Table {
    const MAX_LOAD: f64 = 0.75;

    pub fn new() -> Self {
        let mut table = Self {
            count: 0,
            capacity: 0,
            entries: std::ptr::null_mut(),
        };

        table.init();

        table
    }

    pub fn init(&mut self) {
        self.count = 0;
        self.capacity = 0;

        if !self.entries.is_null() {
            unsafe {
                let layout = Layout::array::<Entry>(self.capacity).unwrap();
                dealloc(self.entries as *mut u8, layout);
            }
            self.entries = std::ptr::null_mut();
        }
    }

    pub fn find_string(&self, chars: *const u8, length: usize, hash: u32) -> *mut ObjString {
        if self.count == 0 {
            return std::ptr::null_mut();
        }

        let mut index = hash as usize % self.capacity;

        loop {
            unsafe {
                let entry = self.entries.add(index);

                if entry.is_null() {
                    panic!("find_string: entry is null at index {}", index);
                }

                if (*entry).key.is_null() {
                    // Stop if we find an empty non-tombstone entry.
                    if (*entry).value.is_nil() {
                        return std::ptr::null_mut();
                    }
                    // Otherwise it's a tombstone, continue probing.
                } else {
                    let key = (*entry).key;
                    if (*key).length() == length && (*key).hash() == hash {
                        // Compare actual characters
                        let chars_ptr = ObjString::chars_ptr(key);
                        let key_slice = std::slice::from_raw_parts(chars_ptr, length);
                        let search_slice = std::slice::from_raw_parts(chars, length);

                        if key_slice == search_slice {
                            return key;
                        }
                    }
                }

                index = (index + 1) % self.capacity;
            }
        }
    }

    fn find(entries: *mut Entry, capacity: usize, key: *mut ObjString) -> *mut Entry {
        if capacity == 0 {
            return std::ptr::null_mut();
        }

        if entries.is_null() {
            panic!("find: entries is null");
        }

        let mut index: usize = (unsafe { &*key }).hash() as usize % capacity;
        let mut tombstone: *mut Entry = std::ptr::null_mut();

        let mut steps = 0;

        loop {
            steps += 1;

            unsafe {
                let entry = entries.add(index);

                // Проверка что entry не null
                if entry.is_null() {
                    panic!("find: entry is null at index {}", index);
                }

                if cfg!(debug_assertions) && steps > capacity {
                    panic!("Infinite loop in find() - table may be full");
                }

                if (&*entry).key.is_null() {
                    if (&*entry).value.is_nil() {
                        // empty entry
                        return if tombstone.is_null() {
                            entry
                        } else {
                            tombstone
                        };
                    } else {
                        // tombstone
                        if tombstone.is_null() {
                            tombstone = entry;
                        }
                    }
                } else if (&*entry).key == key {
                    // found key
                    return entry;
                }

                index = (index + 1) % capacity;
            }
        }
    }

    fn adjust_capacity(&mut self, capacity: usize) {
        debug!("adjust_capacity: {} -> {}", self.capacity, capacity);

        let layout = Layout::array::<Entry>(capacity).unwrap();
        let entries = unsafe { alloc(layout) as *mut Entry };

        // Инициализируем новую таблицу
        for i in 0..capacity {
            unsafe {
                let entry = &mut *entries.add(i);
                entry.key = std::ptr::null_mut();
                entry.value = Value::Nil;
            }
        }

        // Сохраняем старые значения
        let old_capacity = self.capacity;
        let old_entries = self.entries;
        let old_count = self.count;

        // Сбрасываем счётчик
        self.count = 0;

        // Переносим старые записи в новую таблицу
        if old_capacity > 0 && !old_entries.is_null() {
            for i in 0..old_capacity {
                unsafe {
                    let entry = &mut *old_entries.add(i);
                    if entry.key.is_null() {
                        continue;
                    }
                    let dest = Self::find(entries, capacity, entry.key);
                    (*dest).key = entry.key;
                    (*dest).value = entry.value;
                    self.count += 1;
                }
            }

            // Освобождаем старую память
            let old_layout = Layout::array::<Entry>(old_capacity).unwrap();
            unsafe { dealloc(old_entries as *mut u8, old_layout) };
        }

        self.entries = entries;
        self.capacity = capacity;

        debug!(
            "adjust_capacity: migrated {} entries, new count={}",
            old_count, self.count
        );
    }

    pub fn add_all(&mut self, from: &Table) {
        for i in 0..from.capacity {
            unsafe {
                let entry = &*from.entries.add(i);
                if !entry.key.is_null() {
                    self.set(entry.key, entry.value);
                }
            }
        }
    }

    pub fn delete(&mut self, key: *mut ObjString) -> bool {
        if self.count == 0 {
            return false;
        }

        // find entry
        let entry = unsafe { &mut *Self::find(self.entries, self.capacity, key) };
        if entry.key.is_null() {
            return false;
        }

        // place tombstone
        entry.key = std::ptr::null_mut();
        entry.value = Value::Bool(true);

        true
    }

    pub fn entry(&self, index: usize) -> Entry {
        unsafe { *self.entries.add(index) }
    }

    pub fn get(&self, key: *mut ObjString, value: *mut Value) -> bool {
        if self.count == 0 {
            return false;
        }

        let entry = unsafe { &*Self::find(self.entries, self.capacity, key) };
        if entry.key.is_null() {
            return false;
        }

        unsafe { *value = entry.value };

        true
    }

    pub fn set(&mut self, key: *mut ObjString, value: Value) -> bool {
        if (self.count + 1) as f64 > self.capacity as f64 * Self::MAX_LOAD {
            let capacity = utils::grow_capacity(self.capacity);
            self.adjust_capacity(capacity);
        }

        let entry = unsafe { &mut *Self::find(self.entries, self.capacity, key) };

        let is_new_key = entry.key.is_null();

        if is_new_key && entry.value.is_nil() {
            self.count += 1;
        };

        entry.key = key;
        entry.value = value;

        is_new_key
    }

    #[allow(unused)]
    pub fn dump(&self) {
        if self.capacity == 0 {
            println!("Table: empty");
            return;
        }

        println!(
            "Table: count={}, capacity={}, load={:.2}",
            self.count,
            self.capacity,
            self.count as f64 / self.capacity as f64
        );

        for i in 0..self.capacity {
            unsafe {
                let entry = &*self.entries.add(i);
                if entry.key.is_null() {
                    if entry.value.is_nil() {
                        println!("  [{}]: empty", i);
                    } else {
                        println!("  [{}]: TOMBSTONE", i);
                    }
                } else {
                    println!(
                        "  [{}]: key={:?} (hash={}) addr={:p}",
                        i,
                        unsafe { (&*entry.key).as_str() },
                        unsafe { (*entry.key).hash() },
                        unsafe { entry.key }
                    );
                }
            }
        }
    }

    pub fn free(&mut self) {
        if !self.entries.is_null() {
            unsafe {
                let layout = Layout::array::<Entry>(self.capacity).unwrap();
                dealloc(self.entries as *mut u8, layout);
            }
        }

        self.count = 0;
        self.capacity = 0;
        self.entries = std::ptr::null_mut();
    }

    pub fn capacity(&self) -> usize {
        self.capacity
    }
}
