use crate::{common::Value, vm::Vm, Obj, ObjFunction, ObjType};

#[repr(C)]
pub struct ObjClosure {
    obj: Obj,
    pub function: *mut ObjFunction,
}

impl ObjClosure {
    pub fn new(vm: &mut Vm, function: *mut ObjFunction) -> *mut ObjClosure {
        let ptr = vm.allocate_obj(size_of::<ObjClosure>(), ObjType::Closure) as *mut ObjClosure;

        unsafe {
            (*ptr).function = function;
        }

        ptr
    }
}

impl std::fmt::Display for ObjClosure {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", unsafe { (&*self.function) })
    }
}

impl Value {
    pub fn is_closure(&self) -> bool {
        match &self {
            Value::Obj(ptr) if !ptr.is_null() => unsafe { (**ptr).typ == ObjType::Closure },
            _ => false,
        }
    }

    pub fn as_closure(&self) -> *mut ObjClosure {
        if self.is_closure() {
            self.as_obj() as *mut ObjClosure
        } else {
            std::ptr::null_mut()
        }
    }
}
