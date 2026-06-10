use crate::{
    common::{Table, Value},
    vm::Vm,
    Obj, ObjString, ObjType,
};

#[repr(C)]
pub struct ObjClass {
    pub(super) obj: Obj,
    pub name: *const ObjString,
    pub methods: Table,
}

impl ObjClass {
    #[inline(always)]
    pub fn name(&self) -> *const ObjString {
        self.name
    }

    pub fn allocate(vm: &mut Vm, name: *mut ObjString) -> *mut ObjClass {
        let ptr = vm.allocate_obj(size_of::<ObjClass>(), ObjType::Class) as *mut ObjClass;

        unsafe {
            (*ptr).name = name;
            (*ptr).methods = Table::new();
        }

        ptr
    }
}

impl std::fmt::Display for ObjClass {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let name = (unsafe { &*self.name }).as_str();

        write!(f, "{}", name)
    }
}

impl Value {
    pub fn is_class(&self) -> bool {
        match &self {
            Value::Obj(ptr) if !ptr.is_null() => unsafe { (**ptr).typ == ObjType::Class },
            _ => false,
        }
    }

    pub fn as_class(&self) -> *mut ObjClass {
        if self.is_class() {
            self.as_obj() as *mut ObjClass
        } else {
            std::ptr::null_mut()
        }
    }
}
