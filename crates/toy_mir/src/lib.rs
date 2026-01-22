mod display;
pub mod ir;
mod lower;

pub use ir::{
    Block, BlockId, CapturedVar, FuncId, Function, Inst, LocalId, Module, Operand, VReg, ValueType,
};
pub use lower::{lower, lower_typed_module, lower_with_interner};
