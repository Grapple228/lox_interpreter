use crate::{common::Value, object::ObjClosure, vm::Vm, Obj, ObjType};

#[repr(C)]
pub struct ObjBoundMethod {
    pub(super) obj: Obj,
    pub receiver: Value,
    pub method: *mut ObjClosure,
}

impl ObjBoundMethod {
    pub fn allocate(vm: &mut Vm, receiver: Value, method: *mut ObjClosure) -> *mut ObjBoundMethod {
        let ptr = vm.allocate_obj(size_of::<ObjBoundMethod>(), ObjType::BoundMethod)
            as *mut ObjBoundMethod;

        unsafe {
            (*ptr).receiver = receiver;
            (*ptr).method = method;
        }

        ptr
    }
}

impl std::fmt::Display for ObjBoundMethod {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let name = unsafe { &*self.method };
        write!(f, "{}", name)
    }
}

impl Value {
    pub fn is_bound_method(&self) -> bool {
        match &self {
            Value::Obj(ptr) if !ptr.is_null() => unsafe { (**ptr).typ == ObjType::BoundMethod },
            _ => false,
        }
    }

    pub fn as_bound_method(&self) -> *mut ObjBoundMethod {
        if self.is_bound_method() {
            self.as_obj() as *mut ObjBoundMethod
        } else {
            std::ptr::null_mut()
        }
    }
}
