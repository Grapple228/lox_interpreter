use crate::{
    common::{Chunk, Value},
    vm::Vm,
    Obj, ObjString, ObjType,
};

#[repr(u8)]
#[derive(PartialEq, Clone, Copy)]
pub enum FunctionType {
    Function = 0,
    Script = 1,
}

#[repr(C)]
pub struct ObjFunction {
    pub(super) obj: Obj,
    pub arity: usize,
    pub(super) chunk: Chunk,
    pub name: *const ObjString,
}

impl ObjFunction {
    pub fn arity(&self) -> usize {
        self.arity
    }

    #[inline(always)]
    pub fn name(&self) -> *const ObjString {
        self.name
    }

    pub fn new(vm: &mut Vm) -> *mut ObjFunction {
        let ptr = vm.allocate_obj(size_of::<ObjFunction>(), ObjType::Function) as *mut ObjFunction;

        unsafe {
            (*ptr).arity = 0;
            (*ptr).name = std::ptr::null();
            (*ptr).chunk = Chunk::new();
        }

        ptr
    }

    pub fn chunk(&mut self) -> *mut Chunk {
        &mut self.chunk as *mut Chunk
    }
}

impl std::fmt::Display for ObjFunction {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.name.is_null() {
            return write!(f, "<script>");
        }

        let name = (unsafe { &*self.name }).as_str();

        write!(f, "<fn {}>", name)
    }
}

impl Value {
    pub fn is_function(&self) -> bool {
        match &self {
            Value::Obj(ptr) if !ptr.is_null() => unsafe { (**ptr).typ == ObjType::Function },
            _ => false,
        }
    }

    pub fn as_function(&self) -> *mut ObjFunction {
        if self.is_function() {
            self.as_obj() as *mut ObjFunction
        } else {
            std::ptr::null_mut()
        }
    }
}
