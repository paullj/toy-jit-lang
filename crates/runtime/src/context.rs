//! Shared runtime context between VM and JIT execution.

use lasso::Rodeo;
use vm::Heap;

/// Shared context between VM and JIT execution.
/// Foundation for tiered compilation - both execution modes share the same heap.
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

    /// Consume the context and return the heap.
    pub fn take_heap(self) -> Heap {
        self.heap
    }
}
