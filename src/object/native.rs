use crate::{common::Value, vm::Vm, Obj, ObjType};

pub type NativeFn = fn(arg_count: usize, args: *mut Value) -> NativeResult;

pub enum NativeResult {
    Success(Value),
    Error(String),
}

#[repr(C)]
pub struct ObjNative {
    obj: Obj,
    arity: usize,
    function: NativeFn,
}

impl ObjNative {
    pub fn new(vm: &mut Vm, arity: usize, function: NativeFn) -> *mut ObjNative {
        let ptr = vm.allocate_obj(size_of::<ObjNative>(), ObjType::Native) as *mut ObjNative;

        unsafe {
            (*ptr).arity = arity;
            ((*ptr).function) = function;
        }

        ptr
    }

    #[inline(always)]
    pub fn function(&self) -> NativeFn {
        self.function
    }

    #[inline(always)]
    pub fn arity(&self) -> usize {
        self.arity
    }
}

impl std::fmt::Display for ObjNative {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "<native fn>")
    }
}

impl Value {
    pub fn is_native(&self) -> bool {
        match &self {
            Value::Obj(ptr) if !ptr.is_null() => unsafe { (**ptr).typ == ObjType::Native },
            _ => false,
        }
    }
}
