//! Call frame for function invocation.

use crate::value::Value;
use std::cell::RefCell;
use std::rc::Rc;

/// Call frame for function invocation
#[derive(Debug, Clone)]
pub struct CallFrame {
    pub return_pc: usize,
    pub return_chunk: usize,
    /// Base index into the unified stack for this frame
    pub stack_base: usize,
    /// Result slot relative to caller's stack_base (if any)
    pub result_slot: Option<u32>,
    pub closure_env: Option<Rc<RefCell<Vec<Value>>>>,
}
