use crate::{
    common::{Table, Value},
    object::ObjClass,
    vm::Vm,
    Obj, ObjType,
};

/*
 * TODO: Add instance[index] for access to runtime generated fields
 * var field_name = "hello";
 * instance[field_name] = "world";
 * print instance[field_name]; // "world"
*/

#[repr(C)]
pub struct ObjInstance {
    pub(super) obj: Obj,
    pub class: *mut ObjClass,
    pub fields: Table,
}

impl ObjInstance {
    pub fn allocate(vm: &mut Vm, class: *mut ObjClass) -> *mut ObjInstance {
        let ptr = vm.allocate_obj(size_of::<ObjInstance>(), ObjType::Instance) as *mut ObjInstance;

        unsafe {
            (*ptr).class = class;
            (*ptr).fields = Table::new();
        }

        ptr
    }
}

impl std::fmt::Display for ObjInstance {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let name = (unsafe { &*(*(*self).class).name }).as_str();

        write!(f, "{} instance", name)
    }
}

impl Value {
    pub fn is_instance(&self) -> bool {
        match &self {
            Value::Obj(ptr) if !ptr.is_null() => unsafe { (**ptr).typ == ObjType::Instance },
            _ => false,
        }
    }

    pub fn as_instance(&self) -> *mut ObjInstance {
        if self.is_instance() {
            self.as_obj() as *mut ObjInstance
        } else {
            std::ptr::null_mut()
        }
    }
}
