use crate::{common::Value, vm::Vm, Obj, ObjType};

#[repr(C)]
pub struct ObjUpValue {
    obj: Obj,
    pub location: *mut Value,
    pub closed: Value,
    pub next: *mut ObjUpValue,
}

impl ObjUpValue {
    pub fn new(vm: &mut Vm, slot: *mut Value) -> *mut ObjUpValue {
        let ptr = vm.allocate_obj(size_of::<ObjUpValue>(), ObjType::UpValue) as *mut ObjUpValue;

        unsafe {
            (*ptr).location = slot;
            (*ptr).closed = Value::Nil;
            (*ptr).next = std::ptr::null_mut();
        }

        ptr
    }
}

impl std::fmt::Display for ObjUpValue {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "upvalue")
    }
}

impl Value {
    pub fn is_upvalue(&self) -> bool {
        match &self {
            Value::Obj(ptr) if !ptr.is_null() => unsafe { (**ptr).typ == ObjType::UpValue },
            _ => false,
        }
    }

    pub fn as_upvalue(&self) -> *mut ObjUpValue {
        if self.is_upvalue() {
            self.as_obj() as *mut ObjUpValue
        } else {
            std::ptr::null_mut()
        }
    }
}
