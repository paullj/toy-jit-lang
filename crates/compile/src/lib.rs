pub mod bytecode;
mod chunk;
mod compiler;
mod constant;
mod display;

pub use bytecode::{ConstIdx, FuncIdx, Instruction, Label, Slot};
pub use chunk::{Chunk, CompiledModule};
pub use compiler::compile;
pub use constant::{Constant, ConstantPool};

// Re-export lasso types for VM
pub use lasso::{Rodeo, Spur};
