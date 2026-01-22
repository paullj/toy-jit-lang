//! Typed HIR: Wrapper structures that pair HIR nodes with their inferred types.
//!
//! These structures provide a typed view of the HIR after type inference,
//! enabling type-driven code generation in later compiler phases.

use la_arena::ArenaMap;
use std::collections::HashMap;
use std::sync::Arc;

use crate::env::TypeEnv;
use crate::scheme::Scheme;
use crate::types::Type;
use toy_hir::{Definition, ExprIdx, Item, LowerItemResult, ModuleId};

/// A typed expression: pairs an expression index with its inferred type.
#[derive(Debug, Clone)]
pub struct TypedExpression {
    pub expr_idx: ExprIdx,
    pub ty: Type,
}

/// A typed item: contains the HIR item and type information for all expressions within it.
#[derive(Debug, Clone, PartialEq)]
pub struct TypedItem {
    /// The HIR item (definition, assignment, or expression)
    pub item: Item,
    /// Types for all expressions within this item
    pub expr_types: ArenaMap<ExprIdx, Type>,
    /// The item's span for diagnostics
    pub span: toy_hir::TextRange,
}

impl TypedItem {
    /// Get the type of a specific expression within this item.
    pub fn get_expr_type(&self, idx: ExprIdx) -> Option<&Type> {
        self.expr_types.get(idx)
    }

    /// For function definitions, get the function's type.
    pub fn get_function_type(&self) -> Option<Type> {
        match &self.item {
            Item::Definition(Definition::Function {
                params,
                return_type: _,
                body,
                ..
            }) => {
                // Build function type from parameters and inferred return type
                let param_types = params
                    .iter()
                    .map(|_| {
                        // For now, use type variables - will be filled by inference
                        // In practice, these should come from the expr_types
                        Type::Error // Placeholder - actual implementation will get from inference
                    })
                    .collect();

                let ret_type = self.expr_types.get(*body).cloned().unwrap_or(Type::Error);

                Some(Type::Function {
                    params: param_types,
                    ret: Box::new(ret_type),
                })
            }
            _ => None,
        }
    }
}

/// A typed module: contains all typed items and type environment for a module.
#[derive(Debug, Clone, PartialEq)]
pub struct TypedModule {
    /// Module identifier
    pub module_id: ModuleId,
    /// All typed items in this module
    pub items: Vec<TypedItem>,
    /// Expression arena containing all expressions referenced by items
    pub expressions: la_arena::Arena<toy_hir::Expression>,
    /// Type environment containing all definitions (for cross-module references)
    pub type_env: TypeEnv,
    /// Types for top-level variables and functions
    pub variable_types: HashMap<String, Type>,
    /// Whether there were type errors in this module
    pub has_errors: bool,
}

impl TypedModule {
    /// Create a new typed module.
    pub fn new(module_id: ModuleId) -> Self {
        Self {
            module_id,
            items: Vec::new(),
            expressions: la_arena::Arena::new(),
            type_env: TypeEnv::new(),
            variable_types: HashMap::new(),
            has_errors: false,
        }
    }

    /// Add a typed item to the module.
    pub fn add_item(&mut self, item: TypedItem) {
        self.items.push(item);
    }

    /// Get the type of a named variable or function.
    pub fn get_variable_type(&self, name: &str) -> Option<&Type> {
        self.variable_types.get(name)
    }

    /// Get the type scheme of a named variable or function (for polymorphism).
    pub fn get_scheme(&self, name: &str) -> Option<Scheme> {
        self.type_env.lookup(name).cloned()
    }
}

/// Result of type checking multiple modules.
#[derive(Debug)]
pub struct TypedModules {
    /// All typed modules, indexed by module ID
    pub modules: HashMap<ModuleId, Arc<TypedModule>>,
    /// Global type environment for cross-module references
    pub global_env: TypeEnv,
}

impl TypedModules {
    /// Create a new collection of typed modules.
    pub fn new() -> Self {
        Self {
            modules: HashMap::new(),
            global_env: TypeEnv::new(),
        }
    }

    /// Add a typed module to the collection.
    pub fn add_module(&mut self, module: TypedModule) {
        // Merge module's type environment into global environment
        for (name, scheme) in module.type_env.iter() {
            self.global_env.insert(name.clone(), scheme.clone());
        }
        self.modules.insert(module.module_id, Arc::new(module));
    }

    /// Get a typed module by ID.
    pub fn get_module(&self, id: ModuleId) -> Option<&Arc<TypedModule>> {
        self.modules.get(&id)
    }

    /// Check if any module had type errors.
    pub fn has_errors(&self) -> bool {
        self.modules.values().any(|m| m.has_errors)
    }
}

impl Default for TypedModules {
    fn default() -> Self {
        Self::new()
    }
}

/// Convert from the existing inference result to typed HIR.
///
/// This is a bridge function to maintain compatibility with existing code
/// while transitioning to the typed HIR representation.
pub fn from_inference_result(
    module_id: ModuleId,
    hir_items: &[Arc<LowerItemResult>],
    inference: &crate::InferenceResult,
    expression_types_per_item: &[ArenaMap<ExprIdx, Type>],
    expressions: la_arena::Arena<toy_hir::Expression>,
) -> TypedModule {
    let mut module = TypedModule::new(module_id);
    module.expressions = expressions;
    module.variable_types = inference.variable_types.clone();
    module.has_errors = inference.has_errors();

    // Convert each HIR item to a typed item
    for (item_idx, hir_item) in hir_items.iter().enumerate() {
        let expr_types = expression_types_per_item
            .get(item_idx)
            .cloned()
            .unwrap_or_else(ArenaMap::new);

        let typed_item = TypedItem {
            item: hir_item.item.clone(),
            expr_types,
            span: hir_item.item_span,
        };
        module.add_item(typed_item);
    }

    // Build type environment from variable types
    // Note: This is simplified - proper implementation would track schemes
    for (name, ty) in &module.variable_types {
        module
            .type_env
            .insert(name.clone(), Scheme::mono(ty.clone()));
    }

    module
}
