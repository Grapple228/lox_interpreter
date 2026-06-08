use crate::{common::Value, object::ObjUpValue, vm::Vm, Obj, ObjFunction, ObjType};

#[repr(C)]
pub struct ObjClosure {
    obj: Obj,
    pub function: *mut ObjFunction,
    pub upvalues: *mut *mut ObjUpValue,
    pub upvalue_count: usize,
}

impl ObjClosure {
    pub fn allocate(vm: &mut Vm, function: *mut ObjFunction) -> *mut ObjClosure {
        unsafe {
            let upvalue_count = (*function).upvalue_count;

            // Выделяем память под массив указателей на upvalues
            let upvalues = if upvalue_count > 0 {
                vm.allocate_array::<*mut ObjUpValue>(upvalue_count)
            } else {
                std::ptr::null_mut()
            };

            let ptr = vm.allocate_obj(size_of::<ObjClosure>(), ObjType::Closure) as *mut ObjClosure;

            (*ptr).function = function;
            (*ptr).upvalues = upvalues;
            (*ptr).upvalue_count = upvalue_count;

            ptr
        }
    }
}

impl std::fmt::Display for ObjClosure {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", unsafe { &*self.function })
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
