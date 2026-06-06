use std::alloc::{alloc, dealloc, Layout};

use tracing::debug;

mod function;
mod string;

pub use function::{FunctionType, ObjFunction};
pub use string::ObjString;

use crate::{common::Value, vm::Vm};

#[repr(u8)]
#[derive(PartialEq, Clone, Copy)]
pub enum ObjType {
    String = 1,
    Function = 2,
}

impl std::fmt::Display for ObjType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::String => write!(f, "string"),
            Self::Function => write!(f, "func"),
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

impl std::fmt::Display for Obj {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self.typ {
            ObjType::String => {
                // Кастуем Obj* к ObjString*
                let obj_string = self as *const Obj as *const ObjString;
                unsafe { write!(f, "{}", &*obj_string) }
            }
            ObjType::Function => {
                let obj_func = self as *const Obj as *const ObjFunction;
                unsafe { write!(f, "{}", &*obj_func) }
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
}

impl Vm {
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
            ObjType::Function => {
                unsafe {
                    let function = obj as *mut ObjFunction;
                    (*function).chunk.free();

                    let layout =
                        Layout::from_size_align(size_of::<ObjFunction>(), align_of::<ObjString>())
                            .unwrap();
                    dealloc(function as *mut u8, layout);
                };

                todo!()
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
