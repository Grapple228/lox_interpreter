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
    // chars идёт сразу после структуры в памяти (flexible array member)
}

impl ObjString {
    fn chars_ptr(ptr: *const ObjString) -> *mut u8 {
        unsafe { (ptr as *mut u8).add(size_of::<ObjString>()) }
    }

    fn as_chars_ptr(&self) -> *mut u8 {
        Self::chars_ptr(self as *const Self)
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

pub fn objects_equal(a: *mut Obj, b: *mut Obj) -> bool {
    if a.is_null() || b.is_null() {
        return false;
    }

    unsafe {
        let a_type = (*a).typ;
        let b_type = (*b).typ;

        // Только строки поддерживают value equality
        if a_type != ObjType::String || b_type != ObjType::String {
            return false;
        }

        let a_str = a as *mut ObjString;
        let b_str = b as *mut ObjString;

        if (*a_str).length != (*b_str).length {
            return false;
        }

        // Сравнение содержимого строк
        let a_slice = std::slice::from_raw_parts(ObjString::chars_ptr(a_str), (*a_str).length);
        let b_slice = std::slice::from_raw_parts(ObjString::chars_ptr(b_str), (*b_str).length);

        a_slice == b_slice
    }
}

impl Vm {
    pub fn copy_string(&mut self, chars: *const u8, length: usize) -> *mut ObjString {
        let total_size = size_of::<ObjString>() + length + 1;
        let layout = Layout::from_size_align(total_size, align_of::<ObjString>()).unwrap();
        let ptr = unsafe { alloc(layout) } as *mut ObjString;

        self.bytes_allocated += total_size;

        unsafe {
            (*ptr).obj.typ = ObjType::String;
            (*ptr).obj.next = self.objects;
            self.objects = &mut (*ptr).obj;
            (*ptr).length = length;

            let chars_ptr = ObjString::chars_ptr(ptr);
            std::ptr::copy_nonoverlapping(chars, chars_ptr, length);
            *chars_ptr.add(length) = 0;

            debug!(
                "allocated object of type string and size {}, total: {} bytes",
                total_size, self.bytes_allocated
            );
        }

        ptr
    }

    pub fn concatenate_strings(
        &mut self,
        left: *mut ObjString,
        right: *mut ObjString,
    ) -> *mut ObjString {
        if left.is_null() || right.is_null() {
            panic!("Cannot concatenate null strings");
        }

        unsafe {
            let left_len = (*left).length;
            let right_len = (*right).length;
            let total_len = left_len + right_len;

            let total_size = size_of::<ObjString>() + total_len + 1;
            let layout = Layout::from_size_align(total_size, align_of::<ObjString>()).unwrap();
            let ptr = alloc(layout) as *mut ObjString;

            self.bytes_allocated += total_size;

            (*ptr).obj.typ = ObjType::String;
            (*ptr).obj.next = self.objects;
            self.objects = &mut (*ptr).obj;
            (*ptr).length = total_len;

            let chars_ptr = ObjString::chars_ptr(ptr);
            let left_chars = ObjString::chars_ptr(left);
            let right_chars = ObjString::chars_ptr(right);

            std::ptr::copy_nonoverlapping(left_chars, chars_ptr, left_len);
            std::ptr::copy_nonoverlapping(right_chars, chars_ptr.add(left_len), right_len);
            *chars_ptr.add(total_len) = 0;

            debug!(
                "allocated object of type string and size {}, total: {} bytes",
                total_size, self.bytes_allocated
            );

            ptr
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
