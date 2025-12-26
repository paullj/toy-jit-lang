use crate::subst::Subst;
use crate::types::{Type, TypeVar};

#[derive(Debug, Clone, PartialEq)]
pub enum UnifyError {
    Mismatch { expected: Type, found: Type },
    InfiniteType { var: TypeVar, ty: Type },
    ArityMismatch { expected: usize, found: usize },
}

fn occurs_check(var: TypeVar, ty: &Type) -> bool {
    match ty {
        Type::Var(v) => *v == var,
        Type::Function { params, ret } => {
            params.iter().any(|p| occurs_check(var, p)) || occurs_check(var, ret)
        }
        Type::List(elem) => occurs_check(var, elem),
        _ => false,
    }
}

pub fn unify(t1: &Type, t2: &Type) -> Result<Subst, UnifyError> {
    match (t1, t2) {
        // Error propagates
        (Type::Error, _) | (_, Type::Error) => Ok(Subst::new()),

        // Same ground types
        (Type::Integer, Type::Integer)
        | (Type::Float, Type::Float)
        | (Type::Boolean, Type::Boolean)
        | (Type::String, Type::String)
        | (Type::Unit, Type::Unit) => Ok(Subst::new()),

        // Type variable bindings
        (Type::Var(v), t) | (t, Type::Var(v)) => {
            if let Type::Var(v2) = t
                && v == v2
            {
                return Ok(Subst::new());
            }
            if occurs_check(*v, t) {
                return Err(UnifyError::InfiniteType {
                    var: *v,
                    ty: t.clone(),
                });
            }
            Ok(Subst::single(*v, t.clone()))
        }

        // Function types
        (
            Type::Function {
                params: p1,
                ret: r1,
            },
            Type::Function {
                params: p2,
                ret: r2,
            },
        ) => {
            if p1.len() != p2.len() {
                return Err(UnifyError::ArityMismatch {
                    expected: p1.len(),
                    found: p2.len(),
                });
            }

            let mut s = Subst::new();
            for (a, b) in p1.iter().zip(p2.iter()) {
                let s2 = unify(&s.apply(a), &s.apply(b))?;
                s = s2.compose(&s);
            }
            let s_ret = unify(&s.apply(r1), &s.apply(r2))?;
            Ok(s_ret.compose(&s))
        }

        // List types
        (Type::List(e1), Type::List(e2)) => unify(e1, e2),

        // Mismatch
        _ => Err(UnifyError::Mismatch {
            expected: t1.clone(),
            found: t2.clone(),
        }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_unify_same() {
        assert!(unify(&Type::Integer, &Type::Integer).is_ok());
        assert!(unify(&Type::Float, &Type::Float).is_ok());
    }

    #[test]
    fn test_unify_mismatch() {
        let result = unify(&Type::Integer, &Type::Float);
        assert!(matches!(result, Err(UnifyError::Mismatch { .. })));
    }

    #[test]
    fn test_unify_var() {
        let v = Type::Var(TypeVar(0));
        let result = unify(&v, &Type::Integer).unwrap();
        assert_eq!(result.apply(&v), Type::Integer);
    }

    #[test]
    fn test_unify_var_both_sides() {
        let v1 = Type::Var(TypeVar(0));
        let v2 = Type::Var(TypeVar(1));
        let result = unify(&v1, &v2).unwrap();
        // One should map to the other
        let applied1 = result.apply(&v1);
        let applied2 = result.apply(&v2);
        assert_eq!(applied1, applied2);
    }

    #[test]
    fn test_unify_occurs_check() {
        let v = Type::Var(TypeVar(0));
        let fn_ty = Type::Function {
            params: vec![v.clone()],
            ret: Box::new(Type::Integer),
        };
        let result = unify(&v, &fn_ty);
        assert!(matches!(result, Err(UnifyError::InfiniteType { .. })));
    }

    #[test]
    fn test_unify_function() {
        let f1 = Type::Function {
            params: vec![Type::Var(TypeVar(0))],
            ret: Box::new(Type::Var(TypeVar(1))),
        };
        let f2 = Type::Function {
            params: vec![Type::Integer],
            ret: Box::new(Type::Boolean),
        };
        let result = unify(&f1, &f2).unwrap();
        assert_eq!(result.apply(&Type::Var(TypeVar(0))), Type::Integer);
        assert_eq!(result.apply(&Type::Var(TypeVar(1))), Type::Boolean);
    }

    #[test]
    fn test_unify_error_propagates() {
        assert!(unify(&Type::Error, &Type::Integer).is_ok());
        assert!(unify(&Type::Integer, &Type::Error).is_ok());
    }
}
