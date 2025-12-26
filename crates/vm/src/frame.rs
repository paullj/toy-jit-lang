//! Call frame for function invocation.

/// Call frame for function invocation
#[derive(Debug, Clone, Copy, Default)]
pub struct CallFrame {
    pub return_pc: usize,
    pub return_chunk: usize,
    /// Base index into the unified stack for this frame
    pub stack_base: usize,
    /// Result slot relative to caller's stack_base (if any)
    pub result_slot: Option<u8>,
    /// Closure index in heap (if called via closure)
    pub closure_idx: Option<u32>,
}
