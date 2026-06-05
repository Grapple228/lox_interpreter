use std::alloc::{alloc, dealloc, Layout};

use tracing::debug;

use crate::{common::Value, vm::Vm};

#[repr(u8)]
#[derive(PartialEq, Clone, Copy)]
pub enum ObjType {
    String = 1,
}

impl std::fmt::Display for ObjType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::String => write!(f, "string"),
        }
    }
}

#[repr(C)]
pub struct Obj {
    typ: ObjType,
    next: *mut Obj,
}

impl Obj {
    pub fn typ(&self) -> ObjType {
        self.typ
    }
}

#[repr(C)]
pub struct ObjString {
    obj: Obj,
    length: usize,
    hash: u32,
    // chars идёт сразу после структуры в памяти (flexible array member)
}

impl ObjString {
    pub fn chars_ptr(ptr: *const ObjString) -> *mut u8 {
        unsafe { (ptr as *mut u8).add(size_of::<ObjString>()) }
    }

    fn as_chars_ptr(&self) -> *mut u8 {
        Self::chars_ptr(self as *const Self)
    }

    pub fn length(&self) -> usize {
        self.length
    }

    pub fn hash(&self) -> u32 {
        self.hash
    }

    pub fn as_str(&self) -> &str {
        unsafe {
            let chars_ptr = self.as_chars_ptr();
            let slice = std::slice::from_raw_parts(chars_ptr, self.length);
            std::str::from_utf8_unchecked(slice)
        }
    }
}

impl std::fmt::Display for ObjString {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "\"{}\"", self.as_str())
    }
}

impl std::fmt::Display for Obj {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self.typ {
            ObjType::String => {
                // Кастуем Obj* к ObjString*
                let obj_string = self as *const Obj as *const ObjString;
                unsafe { write!(f, "{}", &*obj_string) }
            }
        }
    }
}

impl Value {
    pub fn is_obj(&self) -> bool {
        matches!(self, Value::Obj(_))
    }

    pub fn as_obj(&self) -> *mut Obj {
        match self {
            Value::Obj(ptr) => *ptr,
            _ => std::ptr::null_mut(),
        }
    }

    pub fn as_index(&self) -> Option<usize> {
        if let Value::Index(num) = self {
            Some(*num)
        } else {
            None
        }
    }

    pub fn as_number(&self) -> Option<f64> {
        if let Value::Number(num) = self {
            Some(*num)
        } else {
            None
        }
    }

    pub fn is_string(&self) -> bool {
        match &self {
            Value::Obj(ptr) if !ptr.is_null() => unsafe { (**ptr).typ == ObjType::String },
            _ => false,
        }
    }

    pub fn as_string(&self) -> *mut ObjString {
        if self.is_string() {
            self.as_obj() as *mut ObjString
        } else {
            std::ptr::null_mut()
        }
    }
}

impl Vm {
    fn allocate_string(&mut self, length: usize) -> *mut ObjString {
        let total_size = size_of::<ObjString>() + length + 1;
        let layout = Layout::from_size_align(total_size, align_of::<ObjString>()).unwrap();
        let ptr = unsafe { alloc(layout) } as *mut ObjString;

        self.bytes_allocated += total_size;

        unsafe {
            (*ptr).obj.typ = ObjType::String;
            (*ptr).obj.next = self.objects;
            self.objects = &mut (*ptr).obj;
            (*ptr).length = length;
            (*ptr).hash = 0;
        };

        ptr
    }

    fn hash_string(key: *const u8, length: usize) -> u32 {
        let mut hash: u32 = 2166136261; // FNV offset basis

        for i in 0..length {
            unsafe {
                hash ^= *key.add(i) as u32;
                hash = hash.wrapping_mul(16777619); // FNV prime
            }
        }

        hash
    }

    pub fn take_string(&mut self, chars: *mut u8, length: usize) -> *mut ObjString {
        let hash = Self::hash_string(chars, length);

        // Проверяем, нет ли уже такой строки
        let existing = self.strings.find_string(chars, length, hash);
        if !existing.is_null() {
            // Освобождаем буфер, возвращаем существующую
            unsafe {
                std::alloc::dealloc(
                    chars,
                    std::alloc::Layout::from_size_align(length + 1, 1).unwrap(),
                );
            }
            return existing;
        }

        // Создаём новую строку
        let total_size = size_of::<ObjString>() + length + 1;
        let layout = Layout::from_size_align(total_size, align_of::<ObjString>()).unwrap();
        let ptr = unsafe { alloc(layout) } as *mut ObjString;

        self.bytes_allocated += total_size;

        unsafe {
            (*ptr).obj.typ = ObjType::String;
            (*ptr).obj.next = self.objects;
            self.objects = &mut (*ptr).obj;
            (*ptr).length = length;
            (*ptr).hash = hash;

            let dest = ObjString::chars_ptr(ptr);
            std::ptr::copy_nonoverlapping(chars, dest, length);
            *dest.add(length) = 0;
        }

        // Добавляем в таблицу интернирования
        self.strings.set(ptr, Value::Nil);

        ptr
    }

    pub fn copy_string(&mut self, chars: *const u8, length: usize) -> *mut ObjString {
        let heap_chars = unsafe {
            let ptr =
                std::alloc::alloc(std::alloc::Layout::from_size_align(length + 1, 1).unwrap());
            std::ptr::copy_nonoverlapping(chars, ptr, length);
            *ptr.add(length) = 0;
            ptr
        };

        self.take_string(heap_chars, length)
    }

    pub fn concatenate_strings(
        &mut self,
        left: *mut ObjString,
        right: *mut ObjString,
    ) -> *mut ObjString {
        unsafe {
            let left_len = (*left).length;
            let right_len = (*right).length;
            let total_len = left_len + right_len;

            let chars =
                std::alloc::alloc(std::alloc::Layout::from_size_align(total_len + 1, 1).unwrap());

            let left_chars = ObjString::chars_ptr(left);
            let right_chars = ObjString::chars_ptr(right);

            std::ptr::copy_nonoverlapping(left_chars, chars, left_len);
            std::ptr::copy_nonoverlapping(right_chars, chars.add(left_len), right_len);
            *chars.add(total_len) = 0;

            self.take_string(chars, total_len)
        }
    }

    fn allocate(&mut self, size: usize) -> *mut u8 {
        let layout = Layout::from_size_align(size, 1).unwrap();
        let ptr = unsafe { alloc(layout) };

        self.bytes_allocated += layout.size();

        ptr
    }

    fn allocate_obj(&mut self, size: usize, typ: ObjType) -> *mut Obj {
        unsafe {
            let obj = self.allocate(size) as *mut Obj;
            (*obj).typ = typ;
            (*obj).next = self.objects;

            self.objects = obj;

            debug!(
                "allocated object of type {} and size {}, total: {} bytes",
                typ, size, self.bytes_allocated
            );

            obj
        }
    }

    pub fn deallocate_object(obj: *mut Obj) -> usize {
        let typ = unsafe { (*obj).typ };

        match typ {
            ObjType::String => {
                let string = obj as *mut ObjString;
                unsafe {
                    let length = (*string).length;
                    let total_size = size_of::<ObjString>() + length + 1;
                    let layout =
                        Layout::from_size_align(total_size, align_of::<ObjString>()).unwrap();
                    dealloc(string as *mut u8, layout);
                    total_size
                }
            }
        }
    }

    pub fn free_objects(&mut self) {
        self.strings.free();

        let mut obj = self.objects;

        while !obj.is_null() {
            unsafe {
                let typ = (*obj).typ;
                let next = (*obj).next;
                let size = Self::deallocate_object(obj);
                self.bytes_allocated -= size;

                debug!(
                    "Freeing of {} object with size {}, total: {}",
                    typ, size, self.bytes_allocated
                );

                obj = next;
            }
        }
    }
}
