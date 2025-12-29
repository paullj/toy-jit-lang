//! Runtime helpers for JIT-compiled code.
//!
//! These functions are called from JIT-compiled code via FFI to perform
//! operations that require heap access (lists, dynamic strings, etc.).

use lasso::Rodeo;
use vm::{Heap, Value};

/// Runtime context passed to JIT-compiled main function.
/// Contains heap and interner for dynamic allocations.
#[repr(C)]
pub struct RuntimeContext {
    pub heap: Heap,
    pub interner: Rodeo,
}

impl RuntimeContext {
    pub fn new(interner: Rodeo) -> Self {
        Self {
            heap: Heap::new(),
            interner,
        }
    }

    /// Register struct metadata for display purposes
    pub fn register_struct_meta(&mut self, struct_id: u32, name: String, field_names: Vec<String>) {
        self.heap.register_struct_meta(struct_id, name, field_names);
    }

    pub fn take_heap(self) -> Heap {
        self.heap
    }
}

/// Allocate a new list. Returns NaN-boxed list Value as i64.
#[unsafe(no_mangle)]
pub extern "C" fn rt_list_new(ctx: *mut RuntimeContext, capacity: i64) -> i64 {
    let ctx = unsafe { &mut *ctx };
    let idx = ctx.heap.alloc_list(capacity as usize);
    Value::list(idx).to_bits()
}

/// Set list element. list_val and elem_val are NaN-boxed i64.
#[unsafe(no_mangle)]
pub extern "C" fn rt_list_set(ctx: *mut RuntimeContext, list_val: i64, index: i64, elem_val: i64) {
    let ctx = unsafe { &mut *ctx };
    let list_idx = Value::from_bits(list_val)
        .as_list_idx()
        .expect("rt_list_set: not a list");
    let list = ctx.heap.get_list_mut(list_idx);

    let len = list.elements.len() as i64;
    let idx = if index < 0 { len + index } else { index };

    if idx < 0 || idx >= len {
        panic!("list index out of bounds: {} (len {})", index, len);
    }

    list.elements[idx as usize] = Value::from_bits(elem_val);
}

/// Get list element. Returns NaN-boxed Value as i64.
#[unsafe(no_mangle)]
pub extern "C" fn rt_list_get(ctx: *mut RuntimeContext, list_val: i64, index: i64) -> i64 {
    let ctx = unsafe { &mut *ctx };
    let list_idx = Value::from_bits(list_val)
        .as_list_idx()
        .expect("rt_list_get: not a list");
    let list = ctx.heap.get_list(list_idx);

    let len = list.elements.len() as i64;
    let idx = if index < 0 { len + index } else { index };

    if idx < 0 || idx >= len {
        panic!("list index out of bounds: {} (len {})", index, len);
    }

    list.elements[idx as usize].to_bits()
}

/// Slice list. Returns NaN-boxed list Value as i64.
/// start/end: i64::MIN = None (use default)
#[unsafe(no_mangle)]
pub extern "C" fn rt_list_slice(
    ctx: *mut RuntimeContext,
    list_val: i64,
    start: i64,
    end: i64,
) -> i64 {
    let ctx = unsafe { &mut *ctx };
    let list_idx = Value::from_bits(list_val)
        .as_list_idx()
        .expect("rt_list_slice: not a list");
    let list = ctx.heap.get_list(list_idx);
    let len = list.elements.len();

    let start_idx = if start == i64::MIN {
        0
    } else if start < 0 {
        (len as i64 + start).max(0) as usize
    } else {
        (start as usize).min(len)
    };

    let end_idx = if end == i64::MIN {
        len
    } else if end < 0 {
        (len as i64 + end).max(0) as usize
    } else {
        (end as usize).min(len)
    };

    let slice: Vec<Value> = if start_idx < end_idx {
        list.elements[start_idx..end_idx].to_vec()
    } else {
        Vec::new()
    };

    let new_idx = ctx.heap.alloc_list(slice.len());
    ctx.heap.get_list_mut(new_idx).elements = slice;
    Value::list(new_idx).to_bits()
}

/// Get list length. Returns length as i64.
#[unsafe(no_mangle)]
pub extern "C" fn rt_list_len(ctx: *mut RuntimeContext, list_val: i64) -> i64 {
    let ctx = unsafe { &mut *ctx };
    let list_idx = Value::from_bits(list_val)
        .as_list_idx()
        .expect("rt_list_len: not a list");
    let list = ctx.heap.get_list(list_idx);
    list.elements.len() as i64
}

/// Echo value to stdout. val is NaN-boxed i64.
#[unsafe(no_mangle)]
pub extern "C" fn rt_echo(ctx: *mut RuntimeContext, val: i64) {
    let ctx = unsafe { &*ctx };
    let value = Value::from_bits(val);
    println!("{}", value.display_with_interner(&ctx.heap, &ctx.interner));
}

/// Allocate a new tuple from an array of NaN-boxed elements.
/// elements_ptr: pointer to array of i64 (NaN-boxed values)
/// count: number of elements
/// Returns NaN-boxed tuple Value as i64.
#[unsafe(no_mangle)]
pub extern "C" fn rt_tuple_new(
    ctx: *mut RuntimeContext,
    elements_ptr: *const i64,
    count: i64,
) -> i64 {
    let ctx = unsafe { &mut *ctx };
    let elements: Vec<Value> = (0..count as usize)
        .map(|i| {
            let bits = unsafe { *elements_ptr.add(i) };
            Value::from_bits(bits)
        })
        .collect();

    let idx = ctx.heap.alloc_tuple(elements);
    Value::tuple(idx).to_bits()
}

/// Get tuple element by index. Returns NaN-boxed Value as i64.
#[unsafe(no_mangle)]
pub extern "C" fn rt_tuple_get(ctx: *mut RuntimeContext, tuple_val: i64, index: i64) -> i64 {
    let ctx = unsafe { &*ctx };
    let tuple_idx = Value::from_bits(tuple_val)
        .as_tuple_idx()
        .expect("rt_tuple_get: not a tuple");
    let tuple = ctx.heap.get_tuple(tuple_idx);

    if index < 0 || index >= tuple.elements.len() as i64 {
        panic!(
            "tuple index out of bounds: {} (len {})",
            index,
            tuple.elements.len()
        );
    }

    tuple.elements[index as usize].to_bits()
}

/// Allocate a new struct from an array of NaN-boxed fields.
/// struct_id: identifier for the struct type
/// fields_ptr: pointer to array of i64 (NaN-boxed values)
/// count: number of fields
/// Returns NaN-boxed struct Value as i64.
#[unsafe(no_mangle)]
pub extern "C" fn rt_struct_new(
    ctx: *mut RuntimeContext,
    struct_id: i64,
    fields_ptr: *const i64,
    count: i64,
) -> i64 {
    let ctx = unsafe { &mut *ctx };
    let fields: Vec<Value> = (0..count as usize)
        .map(|i| {
            let bits = unsafe { *fields_ptr.add(i) };
            Value::from_bits(bits)
        })
        .collect();

    let idx = ctx.heap.alloc_struct(struct_id as u32, fields);
    Value::struct_obj(idx).to_bits()
}

/// Get struct field by index. Returns NaN-boxed Value as i64.
#[unsafe(no_mangle)]
pub extern "C" fn rt_struct_get(
    ctx: *mut RuntimeContext,
    struct_val: i64,
    field_index: i64,
) -> i64 {
    let ctx = unsafe { &*ctx };
    let struct_idx = Value::from_bits(struct_val)
        .as_struct_idx()
        .expect("rt_struct_get: not a struct");
    let struct_data = ctx.heap.get_struct(struct_idx);

    if field_index < 0 || field_index >= struct_data.fields.len() as i64 {
        panic!(
            "struct field index out of bounds: {} (len {})",
            field_index,
            struct_data.fields.len()
        );
    }

    struct_data.fields[field_index as usize].to_bits()
}

/// Set struct field by index. struct_val, value are NaN-boxed i64.
#[unsafe(no_mangle)]
pub extern "C" fn rt_struct_set(
    ctx: *mut RuntimeContext,
    struct_val: i64,
    field_index: i64,
    value: i64,
) {
    let ctx = unsafe { &mut *ctx };
    let struct_idx = Value::from_bits(struct_val)
        .as_struct_idx()
        .expect("rt_struct_set: not a struct");
    let struct_data = ctx.heap.get_struct_mut(struct_idx);

    if field_index < 0 || field_index >= struct_data.fields.len() as i64 {
        panic!(
            "struct field index out of bounds: {} (len {})",
            field_index,
            struct_data.fields.len()
        );
    }

    struct_data.fields[field_index as usize] = Value::from_bits(value);
}
