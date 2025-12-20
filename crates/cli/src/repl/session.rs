use infer::InferState;

/// Entry in REPL history.
#[derive(Debug, Clone)]
pub struct HistoryEntry {
    pub input: String,
    pub output: Option<String>,
    pub is_error: bool,
}

/// Persistent state for REPL session.
#[derive(Debug, Default)]
pub struct ReplSession {
    /// Type inference state (TypeEnv + next_var).
    pub infer_state: InferState,
    /// Command history.
    pub history: Vec<HistoryEntry>,
}

impl ReplSession {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add_history(&mut self, input: String, output: Option<String>, is_error: bool) {
        self.history.push(HistoryEntry {
            input,
            output,
            is_error,
        });
    }
}
