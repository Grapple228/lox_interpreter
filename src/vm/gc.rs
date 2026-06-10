use crate::{
    common::{utils, DynamicArray, Table, Value},
    object::{ObjClass, ObjClosure, ObjInstance, ObjUpValue},
    vm::Vm,
    Obj, ObjFunction,
};
use std::alloc::Layout;
use tracing::debug;

pub const GC_HEAP_GROW_FACTOR: usize = 2;

pub struct Gc;

impl Gc {
    pub fn collect_garbage(vm: &mut Vm) {
        debug!("-- gc begin");

        let before = vm.bytes_allocated;

        Self::mark_roots(vm);
        Self::trace_references(vm);
        Self::table_remove_white(vm);
        Self::sweep(vm);

        vm.next_gc = vm.bytes_allocated * GC_HEAP_GROW_FACTOR;

        debug!("-- gc end");
        debug!(
            "    collected {} bytes (from {} to {}) next at {}",
            before - vm.bytes_allocated,
            before,
            vm.bytes_allocated,
            vm.next_gc
        )
    }

    fn table_remove_white(vm: &mut Vm) {
        for i in 0..vm.strings.capacity() {
            let entry = vm.strings.entry(i);
            unsafe {
                if !entry.key().is_null() && !(*entry.key()).obj.is_marked {
                    vm.strings.delete(entry.key());
                }
            }
        }
    }

    fn sweep(vm: &mut Vm) {
        let mut previous = std::ptr::null_mut();
        let mut object = vm.objects;

        let mut deallocated = 0;

        unsafe {
            while !object.is_null() {
                if (*object).is_marked {
                    (*object).is_marked = false;

                    previous = object;
                    object = (*object).next;
                } else {
                    let unreached = object;
                    object = (*object).next;

                    if !previous.is_null() {
                        (*previous).next = object;
                    } else {
                        vm.objects = object;
                    }

                    deallocated += Vm::deallocate_object(unreached);
                }
            }
        }

        vm.bytes_allocated -= deallocated;
    }

    fn mark_roots(vm: &mut Vm) {
        for i in 0..vm.stack.len() {
            let value = *vm.stack.get(i);
            Self::mark_value(vm, value);
        }

        for i in 0..vm.frame_count() {
            Self::mark_obj(vm, vm.frames[i].callee_obj());
        }

        let mut upvalue = vm.open_upvalues;
        while !upvalue.is_null() {
            Self::mark_obj(vm, upvalue as *mut Obj);
            upvalue = unsafe { (*upvalue).next };
        }

        let globals = &mut vm.globals as *mut Table;
        Self::mark_table(vm, globals);
        Self::mark_compiler_roots(vm);
    }

    fn mark_compiler_roots(vm: &mut Vm) {
        let mut current = vm.current;
        while !current.is_null() {
            unsafe {
                Self::mark_obj(vm, (&*current).function as *mut Obj);
                current = (*current).enclosing;
            }
        }
    }

    fn mark_table(vm: &mut Vm, table: *mut Table) {
        let table = unsafe { &mut *table };

        for i in 0..table.capacity() {
            let entry = table.entry(i);
            Self::mark_obj(vm, entry.key() as *mut Obj);
            Self::mark_value(vm, entry.value());
        }
    }

    fn mark_value(vm: &mut Vm, value: Value) {
        if value.is_obj() {
            Self::mark_obj(vm, value.as_obj());
        }
    }

    fn trace_references(vm: &mut Vm) {
        while vm.gray_count > 0 {
            vm.gray_count -= 1;
            let object = unsafe { *vm.gray_stack.add(vm.gray_count) };
            Self::blacken_object(vm, object);
        }
    }

    fn blacken_object(vm: &mut Vm, object: *mut Obj) {
        let value = Value::Obj(object);
        debug!("{:p} blacken {}", object, value);

        unsafe {
            match (*object).typ() {
                crate::ObjType::Native | crate::ObjType::String => {}
                crate::ObjType::UpValue => {
                    let upvalue = object as *mut ObjUpValue;
                    Self::mark_value(vm, (*upvalue).closed);
                }
                crate::ObjType::Function => {
                    let function = object as *mut ObjFunction;
                    Self::mark_obj(vm, (*function).name as *mut Obj);
                    Self::mark_array(vm, &(*(*function).chunk()).constants)
                }
                crate::ObjType::Closure => {
                    let closure = object as *mut ObjClosure;
                    Self::mark_obj(vm, (*closure).function as *mut Obj);

                    for i in 0..(*closure).upvalue_count {
                        Self::mark_obj(vm, (*closure).upvalues.add(i) as *mut Obj);
                    }
                }
                crate::ObjType::Class => {
                    let class = object as *mut ObjClass;
                    Self::mark_obj(vm, (*class).name as *mut Obj);
                }
                crate::ObjType::Instance => {
                    let instance = object as *mut ObjInstance;
                    Self::mark_obj(vm, (*instance).class as *mut Obj);
                    Self::mark_table(vm, &mut (*instance).fields);
                }
            }
        }
    }

    fn mark_array(vm: &mut Vm, array: &DynamicArray<Value>) {
        for i in 0..array.count() {
            Self::mark_value(vm, array[i]);
        }
    }

    fn mark_obj(vm: &mut Vm, obj: *mut Obj) {
        if obj.is_null() {
            return;
        }

        if unsafe { (*obj).is_marked } {
            return;
        }

        let value = Value::Obj(obj);
        debug!("{:p} mark {}", obj, value);

        unsafe { (*obj).is_marked = true };

        if vm.gray_capacity < vm.gray_count + 1 {
            vm.gray_capacity = utils::grow_capacity(vm.gray_capacity);

            unsafe {
                let size = size_of::<*mut Obj>() * vm.gray_capacity;
                let layout = Layout::from_size_align(size, 1).unwrap();

                // Проверяем, первый ли это allocation
                let new_ptr = if vm.gray_stack.is_null() {
                    // Первое выделение - используем alloc
                    std::alloc::alloc(layout) as *mut *mut Obj
                } else {
                    // Последующие - используем realloc
                    let old_size = size_of::<*mut Obj>() * (vm.gray_capacity / 2);
                    let old_layout = Layout::from_size_align(old_size, 1).unwrap();
                    std::alloc::realloc(vm.gray_stack as *mut u8, old_layout, size) as *mut *mut Obj
                };

                vm.gray_stack = new_ptr;
            }

            if vm.gray_stack.is_null() {
                std::process::exit(1);
            }
        }

        unsafe {
            *vm.gray_stack.add(vm.gray_count) = obj;
        }
        vm.gray_count += 1;
    }
}
