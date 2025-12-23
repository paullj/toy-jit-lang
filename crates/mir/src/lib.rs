mod display;
pub mod ir;
mod lower;

pub use ir::{Block, BlockId, CapturedVar, FuncId, Function, Inst, LocalId, Module, Operand, VReg};
pub use lower::lower;
