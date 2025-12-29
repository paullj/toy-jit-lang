mod display;
pub mod ir;
mod lower;

pub use ir::{
    Block, BlockId, CapturedVar, FuncId, Function, Inst, LocalId, Module, Operand, StructMeta,
    VReg, ValueType,
};
pub use lower::lower;
