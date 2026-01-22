//! Error types for module resolution

use crate::{ModuleId, ModulePath};
use text_size::TextRange;

#[derive(Debug)]
pub enum ResolveError {
    /// Module not found
    ModuleNotFound(ModulePath),

    /// Symbol not found in module
    SymbolNotFound { module: ModulePath, symbol: String },

    /// Private symbol accessed
    PrivateSymbol { module: ModulePath, symbol: String },

    /// Circular dependency detected
    CircularDependency(Vec<ModulePath>),

    /// Conflicting exports
    ConflictingExports {
        name: String,
        modules: Vec<ModulePath>,
    },

    /// Collection error
    Collect(CollectError),

    /// Graph building error
    Graph(GraphError),

    /// Module not found in collected data
    ModuleDataNotFound(ModuleId),

    /// Fixpoint iteration did not converge
    FixpointNotReached(Vec<ModuleId>),
}

impl From<CollectError> for ResolveError {
    fn from(e: CollectError) -> Self {
        ResolveError::Collect(e)
    }
}

impl From<GraphError> for ResolveError {
    fn from(e: GraphError) -> Self {
        ResolveError::Graph(e)
    }
}

#[derive(Debug)]
pub enum CollectError {
    /// AST not found for module
    AstNotFound(ModuleId),
}

#[derive(Debug)]
pub enum GraphError {
    /// Path resolution failed
    PathResolution(PathError),

    /// Base module path not found
    BaseNotFound(ModuleId),
}

impl From<PathError> for GraphError {
    fn from(e: PathError) -> Self {
        GraphError::PathResolution(e)
    }
}

#[derive(Debug)]
pub enum PathError {
    /// Base path not found when resolving relative import
    BaseNotFound(ModuleId),

    /// Target module not found
    ModuleNotFound(ModulePath),

    /// Too many ".." in relative path
    TooManyParents,

    /// Invalid path
    InvalidPath,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Diagnostic {
    pub level: DiagnosticLevel,
    pub message: String,
    pub span: Option<TextRange>,
    pub notes: Vec<String>,
}

impl Diagnostic {
    pub fn error(message: impl Into<String>, span: impl Into<Option<TextRange>>) -> Self {
        Self {
            level: DiagnosticLevel::Error,
            message: message.into(),
            span: span.into(),
            notes: Vec::new(),
        }
    }

    pub fn warning(message: impl Into<String>, span: impl Into<Option<TextRange>>) -> Self {
        Self {
            level: DiagnosticLevel::Warning,
            message: message.into(),
            span: span.into(),
            notes: Vec::new(),
        }
    }

    pub fn with_note(mut self, note: impl Into<String>) -> Self {
        self.notes.push(note.into());
        self
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DiagnosticLevel {
    Error,
    Warning,
    Info,
}
