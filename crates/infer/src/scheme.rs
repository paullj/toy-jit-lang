use std::collections::HashSet;

use crate::env::TypeEnv;
use crate::subst::Subst;
use crate::types::{Type, TypeVar};

/// A type scheme: forall quantified. ty
/// e.g., forall 'a. 'a -> 'a
#[derive(Debug, Clone, PartialEq)]
pub struct Scheme {
    pub quantified: HashSet<TypeVar>,
    pub ty: Type,
}

impl Scheme {
    /// Create a monomorphic scheme (no quantified vars)
    pub fn mono(ty: Type) -> Self {
        Self {
            quantified: HashSet::new(),
            ty,
        }
    }

    /// Generalize a type over free vars not in the environment
    pub fn generalize(env: &TypeEnv, ty: &Type) -> Self {
        let env_free = env.free_vars();
        let ty_free = ty.free_vars();
        let quantified: HashSet<_> = ty_free.difference(&env_free).copied().collect();
        Self {
            quantified,
            ty: ty.clone(),
        }
    }

    /// Instantiate a scheme with fresh type variables
    pub fn instantiate<F>(&self, mut fresh: F) -> Type
    where
        F: FnMut() -> TypeVar,
    {
        if self.quantified.is_empty() {
            return self.ty.clone();
        }

        let mut subst = Subst::new();
        for &var in &self.quantified {
            subst.insert(var, Type::Var(fresh()));
        }
        subst.apply(&self.ty)
    }

    pub fn free_vars(&self) -> HashSet<TypeVar> {
        let ty_free = self.ty.free_vars();
        ty_free.difference(&self.quantified).copied().collect()
    }
}

impl std::fmt::Display for Scheme {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.quantified.is_empty() {
            write!(f, "{}", self.ty)
        } else {
            write!(f, "forall ")?;
            let mut vars: Vec<_> = self.quantified.iter().collect();
            vars.sort_by_key(|v| v.0);
            for (i, var) in vars.iter().enumerate() {
                if i > 0 {
                    write!(f, " ")?;
                }
                write!(f, "{}", var)?;
            }
            write!(f, ". {}", self.ty)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mono() {
        let s = Scheme::mono(Type::Integer);
        assert!(s.quantified.is_empty());
        assert_eq!(s.ty, Type::Integer);
    }

    #[test]
    fn test_generalize_no_env() {
        let env = TypeEnv::new();
        let ty = Type::Function {
            params: vec![Type::Var(TypeVar(0))],
            ret: Box::new(Type::Var(TypeVar(0))),
        };
        let scheme = Scheme::generalize(&env, &ty);
        assert!(scheme.quantified.contains(&TypeVar(0)));
    }

    #[test]
    fn test_generalize_with_env() {
        let mut env = TypeEnv::new();
        env.insert("x".to_string(), Scheme::mono(Type::Var(TypeVar(0))));

        let ty = Type::Function {
            params: vec![Type::Var(TypeVar(0))],
            ret: Box::new(Type::Var(TypeVar(1))),
        };
        let scheme = Scheme::generalize(&env, &ty);

        // 'a is in env, so not quantified; 'b is quantified
        assert!(!scheme.quantified.contains(&TypeVar(0)));
        assert!(scheme.quantified.contains(&TypeVar(1)));
    }

    #[test]
    fn test_instantiate() {
        let scheme = Scheme {
            quantified: [TypeVar(0)].into_iter().collect(),
            ty: Type::Function {
                params: vec![Type::Var(TypeVar(0))],
                ret: Box::new(Type::Var(TypeVar(0))),
            },
        };

        let mut counter = 10u32;
        let ty = scheme.instantiate(|| {
            let v = TypeVar(counter);
            counter += 1;
            v
        });

        // Should have fresh var, not TypeVar(0)
        if let Type::Function { params, ret } = ty {
            assert_eq!(params[0], Type::Var(TypeVar(10)));
            assert_eq!(*ret, Type::Var(TypeVar(10)));
        } else {
            panic!("expected function type");
        }
    }
}
