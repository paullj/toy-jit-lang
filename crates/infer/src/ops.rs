use crate::types::Type;
use hir::{InfixOp, PrefixOp};

/// Type signature for a binary operator: (operand, operand) -> result
pub struct InfixSig {
    pub operand: Type,
    pub result: Type,
}

/// Type signature for a unary operator
pub enum PrefixSig {
    /// Operand and result have same type (must be one of the given types)
    Preserve(&'static [Type]),
    /// Fixed operand type, fixed result type
    Fixed { operand: Type, result: Type },
}

pub fn infix_signature(op: InfixOp) -> InfixSig {
    use Type::*;
    match op {
        // Integer arithmetic -> Integer
        InfixOp::Add | InfixOp::Sub | InfixOp::Mul | InfixOp::Div | InfixOp::Mod => InfixSig {
            operand: Integer,
            result: Integer,
        },
        // Float arithmetic -> Float
        InfixOp::AddFloat | InfixOp::SubFloat | InfixOp::MulFloat | InfixOp::DivFloat => InfixSig {
            operand: Float,
            result: Float,
        },
        // Integer comparison -> Boolean
        InfixOp::Eq | InfixOp::NotEq | InfixOp::Gt | InfixOp::Lt | InfixOp::Gte | InfixOp::Lte => {
            InfixSig {
                operand: Integer,
                result: Boolean,
            }
        }
        // Float comparison -> Boolean
        InfixOp::GtFloat | InfixOp::LtFloat | InfixOp::GteFloat | InfixOp::LteFloat => InfixSig {
            operand: Float,
            result: Boolean,
        },
        // Boolean logic -> Boolean
        InfixOp::And | InfixOp::Or => InfixSig {
            operand: Boolean,
            result: Boolean,
        },
    }
}

// Static arrays for Preserve signatures
static NUMERIC_TYPES: &[Type] = &[Type::Integer, Type::Float];

pub fn prefix_signature(op: PrefixOp) -> PrefixSig {
    use Type::*;
    match op {
        PrefixOp::Neg => PrefixSig::Preserve(NUMERIC_TYPES),
        PrefixOp::Not => PrefixSig::Fixed {
            operand: Boolean,
            result: Boolean,
        },
    }
}
