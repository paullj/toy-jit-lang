//! Profiler for tracking execution statistics and hot paths.
//!
//! Currently a stub - will be expanded when functions are added.

/// Profiler for tracking execution and identifying hot paths.
#[derive(Debug, Default)]
pub struct Profiler {
    /// Total number of executions
    execution_count: u64,
    /// Threshold for considering a function "hot"
    hot_threshold: u32,
    // Future fields when functions are added:
    // function_call_counts: HashMap<FunctionId, u64>,
    // jit_compiled: HashSet<FunctionId>,
}

impl Profiler {
    /// Create a new profiler with default settings.
    pub fn new() -> Self {
        Self {
            execution_count: 0,
            hot_threshold: 100, // Default: JIT after 100 calls
        }
    }

    /// Create a new profiler with a custom hot threshold.
    pub fn with_threshold(hot_threshold: u32) -> Self {
        Self {
            execution_count: 0,
            hot_threshold,
        }
    }

    /// Record an execution (called by tiered runtime).
    pub fn record_execution(&mut self) {
        self.execution_count += 1;
    }

    /// Get the total execution count.
    pub fn execution_count(&self) -> u64 {
        self.execution_count
    }

    /// Get the hot threshold.
    pub fn hot_threshold(&self) -> u32 {
        self.hot_threshold
    }

    // Future methods when functions are added:
    //
    // pub fn record_call(&mut self, func_id: FunctionId) {
    //     *self.function_call_counts.entry(func_id).or_insert(0) += 1;
    // }
    //
    // pub fn is_hot(&self, func_id: FunctionId) -> bool {
    //     self.function_call_counts
    //         .get(&func_id)
    //         .map(|&count| count >= self.hot_threshold as u64)
    //         .unwrap_or(false)
    // }
    //
    // pub fn mark_jit_compiled(&mut self, func_id: FunctionId) {
    //     self.jit_compiled.insert(func_id);
    // }
    //
    // pub fn is_jit_compiled(&self, func_id: FunctionId) -> bool {
    //     self.jit_compiled.contains(&func_id)
    // }
}
