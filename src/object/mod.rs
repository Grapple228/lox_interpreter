use std::alloc::{alloc, dealloc, realloc, Layout};

use tracing::debug;

mod class;
mod closure;
mod function;
mod instance;
mod native;
mod string;
mod upvalue;

pub use class::ObjClass;
pub use closure::ObjClosure;
pub use function::{FunctionType, ObjFunction};
pub use instance::ObjInstance;
pub use native::{NativeFn, NativeResult, ObjNative};
pub use string::ObjString;
pub use upvalue::ObjUpValue;

use crate::{
    common::Value,
    vm::{Gc, Vm},
};

#[repr(u8)]
#[derive(PartialEq, Clone, Copy)]
pub enum ObjType {
    String = 1,
    Function = 2,
    Native = 3,
    Closure = 4,
    UpValue = 5,
    Class = 6,
    Instance = 7,
}

impl std::fmt::Display for ObjType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::String => write!(f, "string"),
            Self::Function => write!(f, "func"),
            Self::Native => write!(f, "native"),
            Self::Closure => write!(f, "closure"),
            Self::UpValue => write!(f, "upvalue"),
            Self::Class => write!(f, "class"),
            Self::Instance => write!(f, "instance"),
        }
    }
}

#[repr(C)]
pub struct Obj {
    typ: ObjType,
    pub is_marked: bool,
    pub next: *mut Obj,
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
            ObjType::Native => {
                let obj_native = self as *const Obj as *const ObjNative;
                unsafe { write!(f, "{}", &*obj_native) }
            }
            ObjType::Closure => {
                let obj_closure = self as *const Obj as *const ObjClosure;
                unsafe { write!(f, "{}", &*obj_closure) }
            }
            ObjType::UpValue => {
                let obj_upvalue = self as *const Obj as *const ObjUpValue;
                unsafe { write!(f, "{}", &*obj_upvalue) }
            }
            ObjType::Class => {
                let obj_class = self as *const Obj as *const ObjClass;
                unsafe { write!(f, "{}", &*obj_class) }
            }
            ObjType::Instance => {
                let instance = self as *const Obj as *const ObjInstance;
                unsafe { write!(f, "{}", &*instance) }
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
    pub fn reallocate(&mut self, ptr: *mut u8, old_size: usize, new_size: usize) -> *mut u8 {
        self.bytes_allocated = self.bytes_allocated + new_size - old_size;

        if new_size > old_size && !self.is_compiling {
            if cfg!(debug_assertions) {
                Gc::collect_garbage(self);
            }
            if self.bytes_allocated > self.next_gc {
                Gc::collect_garbage(self);
            }
        }

        if new_size == 0 {
            if !ptr.is_null() {
                unsafe {
                    let layout = Layout::from_size_align(old_size, 1).unwrap();
                    dealloc(ptr, layout);
                }
            }
            return std::ptr::null_mut();
        }

        let new_ptr = if ptr.is_null() {
            unsafe {
                let layout = Layout::from_size_align(new_size, 1).unwrap();
                alloc(layout)
            }
        } else {
            unsafe {
                let new_layout = Layout::from_size_align(new_size, 1).unwrap();
                realloc(ptr, new_layout, new_size)
            }
        };

        new_ptr
    }

    fn allocate(&mut self, size: usize) -> *mut u8 {
        self.reallocate(std::ptr::null_mut(), 0, size)
    }

    pub fn allocate_array<T>(&mut self, count: usize) -> *mut T {
        let size = size_of::<T>() * count;
        self.allocate(size) as *mut T
    }

    fn allocate_obj(&mut self, size: usize, typ: ObjType) -> *mut Obj {
        unsafe {
            let obj = self.allocate(size) as *mut Obj;
            (*obj).typ = typ;
            (*obj).is_marked = false;
            (*obj).next = self.objects;
            self.objects = obj;

            debug!(
                "{:p} allocate {} for {}, total: {}",
                obj, size, typ, self.bytes_allocated
            );

            obj
        }
    }

    pub fn deallocate_object(obj: *mut Obj) -> usize {
        unsafe {
            debug!("{:?} deallocate type {}", obj, (*obj).typ);
        }

        match unsafe { (*obj).typ } {
            ObjType::String => ObjString::deallocate(obj as *mut ObjString),
            ObjType::Function => ObjFunction::deallocate(obj as *mut ObjFunction),
            ObjType::Closure => ObjClosure::deallocate(obj as *mut ObjClosure),
            ObjType::UpValue => ObjUpValue::deallocate(obj as *mut ObjUpValue),
            ObjType::Native => ObjNative::deallocate(obj as *mut ObjNative),
            ObjType::Class => ObjClass::deallocate(obj as *mut ObjClass),
            ObjType::Instance => ObjInstance::deallocate(obj as *mut ObjInstance),
        }
    }

    pub fn free_objects(&mut self) {
        let mut obj = self.objects;

        while !obj.is_null() {
            unsafe {
                let typ = (*obj).typ;
                let next = (*obj).next;
                let size = Self::deallocate_object(obj);

                debug!(
                    "Freeing of {} object with size {}, total: {}",
                    typ, size, self.bytes_allocated
                );

                obj = next;
            }
        }

        if self.gray_capacity > 0 && !self.gray_stack.is_null() {
            self.reallocate(
                self.gray_stack as *mut u8,
                self.gray_capacity * size_of::<*mut Obj>(),
                0,
            );
        }
        self.gray_stack = std::ptr::null_mut();
        self.gray_capacity = 0;
        self.gray_count = 0;
    }
}

pub trait Object: Sized {
    #[inline(always)]
    fn layout() -> Layout {
        Layout::from_size_align(size_of::<Self>(), align_of::<Self>()).unwrap()
    }

    fn deallocate(obj: *mut Self) -> usize {
        let extra = Self::free_extra(obj);
        let layout = Self::layout();
        unsafe {
            dealloc(obj as *mut u8, layout);
        }
        extra + layout.size()
    }

    // Возвращает размер дополнительно выделенной памяти
    fn free_extra(_obj: *mut Self) -> usize {
        0
    }
}

impl Object for Obj {}

impl Object for ObjString {
    fn deallocate(obj: *mut Self) -> usize {
        unsafe {
            let length = (*obj).length;
            let total_size = size_of::<ObjString>() + length + 1;
            let layout = Layout::from_size_align(total_size, align_of::<ObjString>()).unwrap();
            dealloc(obj as *mut u8, layout);
            total_size
        }
    }
}

impl Object for ObjClosure {
    fn free_extra(obj: *mut Self) -> usize {
        unsafe {
            if (*obj).upvalue_count > 0 {
                let array_size = size_of::<*mut ObjUpValue>() * (*obj).upvalue_count;
                let array_layout =
                    Layout::from_size_align(array_size, align_of::<*mut ObjUpValue>()).unwrap();
                dealloc((*obj).upvalues as *mut u8, array_layout);
                array_size
            } else {
                0
            }
        }
    }
}

impl Object for ObjInstance {
    fn free_extra(obj: *mut Self) -> usize {
        unsafe {
            (*obj).fields.free();
        }
        0
    }
}

impl Object for ObjFunction {
    fn free_extra(obj: *mut Self) -> usize {
        unsafe {
            (*obj).chunk.free();
        }
        0
    }
}

impl Object for ObjUpValue {}
impl Object for ObjNative {}
impl Object for ObjClass {}
