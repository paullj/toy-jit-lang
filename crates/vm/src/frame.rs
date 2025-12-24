//! Call frame for function invocation.

use crate::value::Value;
use compile::Reg;
use std::cell::RefCell;
use std::rc::Rc;

/// Call frame for function invocation
#[derive(Debug, Clone)]
pub struct CallFrame {
    pub return_pc: usize,
    pub return_chunk: usize,
    pub base_reg: usize,
    pub base_local: usize,
    pub result_reg: Option<Reg>,
    pub closure_env: Option<Rc<RefCell<Vec<Value>>>>,
}
