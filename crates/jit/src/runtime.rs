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
