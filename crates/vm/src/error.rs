//! Runtime error types.

/// Runtime error
#[derive(Debug, Clone, PartialEq)]
pub enum RuntimeError {
    DivisionByZero,
    StackOverflow,
    InvalidRegister(u32),
    InvalidLocal(u32),
    InvalidConstant(u32),
    InvalidFunction(u32),
    TypeMismatch {
        expected: &'static str,
        got: &'static str,
    },
    NotAClosure,
    NoClosure,
    CaptureOutOfBounds(u8),
}

impl std::fmt::Display for RuntimeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RuntimeError::DivisionByZero => write!(f, "division by zero"),
            RuntimeError::StackOverflow => write!(f, "stack overflow"),
            RuntimeError::InvalidRegister(r) => write!(f, "invalid register r{}", r),
            RuntimeError::InvalidLocal(l) => write!(f, "invalid local slot{}", l),
            RuntimeError::InvalidConstant(c) => write!(f, "invalid constant c{}", c),
            RuntimeError::InvalidFunction(idx) => write!(f, "invalid function fn{}", idx),
            RuntimeError::TypeMismatch { expected, got } => {
                write!(f, "type mismatch: expected {}, got {}", expected, got)
            }
            RuntimeError::NotAClosure => write!(f, "expected closure value"),
            RuntimeError::NoClosure => write!(f, "no closure environment in current frame"),
            RuntimeError::CaptureOutOfBounds(idx) => {
                write!(f, "capture index {} out of bounds", idx)
            }
        }
    }
}

impl std::error::Error for RuntimeError {}
