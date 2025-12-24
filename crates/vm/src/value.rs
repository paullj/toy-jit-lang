//! Runtime value types.

use std::cell::RefCell;
use std::rc::Rc;

/// Closure: function + captured environment
#[derive(Debug, Clone)]
pub struct ClosureValue {
    pub func_idx: usize,
    pub env: Rc<RefCell<Vec<Value>>>,
}

impl PartialEq for ClosureValue {
    fn eq(&self, other: &Self) -> bool {
        self.func_idx == other.func_idx && Rc::ptr_eq(&self.env, &other.env)
    }
}

/// Runtime value
#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    Int(i64),
    Float(f64),
    Bool(bool),
    String(String),
    Unit,
    Closure(ClosureValue),
}

impl Value {
    pub fn type_name(&self) -> &'static str {
        match self {
            Value::Int(_) => "Int",
            Value::Float(_) => "Float",
            Value::Bool(_) => "Bool",
            Value::String(_) => "String",
            Value::Unit => "Unit",
            Value::Closure(_) => "Closure",
        }
    }
}

impl std::fmt::Display for Value {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Value::Int(n) => write!(f, "{}", n),
            Value::Float(n) => write!(f, "{}", n),
            Value::Bool(b) => write!(f, "{}", b),
            Value::String(s) => write!(f, "{}", s),
            Value::Unit => write!(f, "()"),
            Value::Closure(c) => write!(f, "<closure fn{}>", c.func_idx),
        }
    }
}
