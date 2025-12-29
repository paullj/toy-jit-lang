use la_arena::ArenaMap;
use miette::SourceSpan;
use std::collections::HashMap;

use crate::diagnostic::InferDiagnostic;
use crate::env::TypeEnv;
use crate::ops;
use crate::scheme::Scheme;
use crate::subst::Subst;
use crate::suggest::find_similar;
use crate::types::{StructId, Type, TypeVar};
use crate::unify::{UnifyError, unify};
use crate::{InferState, InferWithState, InferenceResult};
use hir::{BlockItem, Definition, ExprIdx, Expression, Ident, Item, Literal, TextRange};

fn to_span(range: TextRange) -> SourceSpan {
    let start: usize = range.start().into();
    let len: usize = range.len().into();
    (start, len).into()
}

/// Information about a struct definition
#[derive(Debug, Clone)]
pub(crate) struct StructInfo {
    pub name: String,
    pub fields: Vec<(String, Type)>, // (field_name, field_type)
}

pub(crate) struct InferCtx<'a> {
    hir: &'a hir::LowerResult,
    env: TypeEnv,
    subst: Subst,
    next_var: u32,
    expr_types: ArenaMap<ExprIdx, Type>,
    diagnostics: Vec<InferDiagnostic>,
    /// Expected return type for the current function (for return statement inference)
    expected_return: Option<Type>,
    /// Struct registry: maps struct name to (StructId, StructInfo)
    structs: HashMap<String, (StructId, StructInfo)>,
    /// Counter for struct IDs
    next_struct_id: u32,
}

impl<'a> InferCtx<'a> {
    pub(crate) fn new(hir: &'a hir::LowerResult) -> Self {
        Self {
            hir,
            env: TypeEnv::new(),
            subst: Subst::new(),
            next_var: 0,
            expr_types: ArenaMap::default(),
            diagnostics: Vec::new(),
            expected_return: None,
            structs: HashMap::new(),
            next_struct_id: 0,
        }
    }

    pub(crate) fn with_env(hir: &'a hir::LowerResult, env: TypeEnv, next_var: u32) -> Self {
        Self {
            hir,
            env,
            subst: Subst::new(),
            next_var,
            expr_types: ArenaMap::default(),
            diagnostics: Vec::new(),
            expected_return: None,
            structs: HashMap::new(),
            next_struct_id: 0,
        }
    }

    /// Register a struct definition and return its StructId
    fn register_struct(&mut self, name: &str) -> StructId {
        if let Some((id, _)) = self.structs.get(name) {
            return *id;
        }
        let id = StructId(self.next_struct_id);
        self.next_struct_id += 1;
        let info = StructInfo {
            name: name.to_string(),
            fields: Vec::new(), // Fields will be populated later if needed
        };
        self.structs.insert(name.to_string(), (id, info));
        id
    }

    /// Look up a struct by name
    fn lookup_struct(&self, name: &str) -> Option<(StructId, &StructInfo)> {
        self.structs.get(name).map(|(id, info)| (*id, info))
    }

    /// Look up a struct by ID
    fn lookup_struct_by_id(&self, id: StructId) -> Option<&StructInfo> {
        self.structs
            .values()
            .find(|(sid, _)| *sid == id)
            .map(|(_, info)| info)
    }

    /// Look up a field type in a struct
    fn lookup_field_type(&self, struct_id: StructId, field_name: &str) -> Option<Type> {
        self.lookup_struct_by_id(struct_id).and_then(|info| {
            info.fields
                .iter()
                .find(|(name, _)| name == field_name)
                .map(|(_, ty)| ty.clone())
        })
    }

    /// Resolve an Ident to its string representation
    fn resolve(&self, ident: Ident) -> &str {
        self.hir.resolve(ident)
    }

    fn fresh_var(&mut self) -> TypeVar {
        let v = TypeVar::new(self.next_var);
        self.next_var += 1;
        v
    }

    /// Parse a type name string into a Type
    fn parse_type_name(&self, name: &str) -> Type {
        match name.to_lowercase().as_str() {
            "int" | "integer" => Type::Integer,
            "float" => Type::Float,
            "bool" | "boolean" => Type::Boolean,
            "string" => Type::String,
            "unit" => Type::Unit,
            _ => Type::Error, // Unknown type name
        }
    }

    /// Infer the type of a function definition
    fn infer_function(
        &mut self,
        name: Option<&str>,
        params: &[hir::FunctionParam],
        body: ExprIdx,
        span: TextRange,
    ) -> Type {
        self.env.push_scope();

        // Create fresh type vars for params without annotations
        let param_types: Vec<Type> = params
            .iter()
            .map(|p| {
                let ty = match &p.ty {
                    Some(ty_name) => self.parse_type_name(self.resolve(*ty_name)),
                    None => Type::Var(self.fresh_var()),
                };

                // Check default value matches param type
                if let Some(default_idx) = p.default {
                    let (default_ty, default_span) = self.infer_expr_idx(default_idx);
                    self.unify_or_error(&ty, &default_ty, default_span);
                }

                // Bind param in scope (monomorphic)
                self.env
                    .insert(self.resolve(p.name).to_string(), Scheme::mono(ty.clone()));
                ty
            })
            .collect();

        // Create return type var
        let ret_var = Type::Var(self.fresh_var());

        // Build function type
        let fn_type = Type::Function {
            params: param_types,
            ret: Box::new(ret_var.clone()),
        };

        // Pre-bind function name for recursion
        if let Some(n) = name {
            self.env
                .insert(n.to_string(), Scheme::mono(fn_type.clone()));
        }

        // Set expected return type for return statements
        let old_expected = self.expected_return.take();
        self.expected_return = Some(ret_var.clone());

        // Infer body
        let (body_ty, _) = self.infer_expr_idx(body);

        // Restore old expected return
        self.expected_return = old_expected;

        // Unify body type with return type
        self.unify_or_error(&ret_var, &body_ty, span);

        self.env.pop_scope();

        // Apply substitution and return final type
        self.subst.apply(&fn_type)
    }

    /// Infer the type of a function call
    fn infer_call(&mut self, callee: ExprIdx, args: &[ExprIdx], _span: TextRange) -> Type {
        let (callee_ty, callee_span) = self.infer_expr_idx(callee);

        // Infer argument types
        let arg_types: Vec<(Type, TextRange)> =
            args.iter().map(|idx| self.infer_expr_idx(*idx)).collect();

        // Create expected function type with fresh return var
        let ret_var = Type::Var(self.fresh_var());
        let expected_fn = Type::Function {
            params: arg_types.iter().map(|(ty, _)| ty.clone()).collect(),
            ret: Box::new(ret_var.clone()),
        };

        // Unify callee with expected function type
        self.unify_or_error(&callee_ty, &expected_fn, callee_span);

        // Return the (now-constrained) return type
        self.subst.apply(&ret_var)
    }

    /// Infer the type of a return statement
    fn infer_return(&mut self, value: Option<ExprIdx>, span: TextRange) -> Type {
        let ret_ty = match value {
            Some(idx) => self.infer_expr_idx(idx).0,
            None => Type::Unit,
        };

        // If we're tracking expected return type, unify
        if let Some(expected) = &self.expected_return.clone() {
            self.unify_or_error(expected, &ret_ty, span);
        }

        // Return statement itself has type Unit (control flow)
        Type::Unit
    }

    pub(crate) fn infer_items(mut self) -> InferenceResult {
        for (i, item) in self.hir.items.iter().enumerate() {
            let span = self.hir.item_spans.get(i).copied().unwrap_or_default();
            self.infer_item(item, span);
        }
        self.finalize()
    }

    pub(crate) fn infer_items_with_state(mut self) -> InferWithState {
        for (i, item) in self.hir.items.iter().enumerate() {
            let span = self.hir.item_spans.get(i).copied().unwrap_or_default();
            self.infer_item(item, span);
        }
        self.finalize_with_state()
    }

    fn infer_item(&mut self, item: &Item, span: TextRange) {
        match item {
            Item::Definition(Definition::Variable { name, value }) => {
                let name_str = self.resolve(*name).to_string();
                let ty = self.infer_expr(value, span);
                // Apply current substitution before generalizing
                let ty = self.subst.apply(&ty);
                let scheme = Scheme::generalize(&self.env, &ty);
                self.env.insert(name_str, scheme);
            }
            Item::Definition(Definition::Function {
                name,
                params,
                return_type,
                body,
            }) => {
                let name_str = self.resolve(*name).to_string();
                let ret_type_str = return_type.map(|r| self.resolve(r).to_string());
                let fn_ty = self.infer_function(Some(&name_str), params, *body, span);

                // Check explicit return annotation if present
                if let Some(ret_name) = ret_type_str {
                    let annotated_ret = self.parse_type_name(&ret_name);
                    if let Type::Function { ret, .. } = &fn_ty {
                        self.unify_or_error(&annotated_ret, ret, span);
                    }
                }

                // Apply substitution before generalizing
                let fn_ty = self.subst.apply(&fn_ty);
                // Generalize and store in env (for polymorphism)
                let scheme = Scheme::generalize(&self.env, &fn_ty);
                self.env.insert(name_str, scheme);
            }
            Item::Assignment { name, value } => {
                let name_str = self.resolve(*name).to_string();
                let value_ty = self.infer_expr(value, span);
                // Clone scheme to avoid borrow conflict with fresh_var
                let scheme = self.env.lookup(&name_str).cloned();
                match scheme {
                    Some(scheme) => {
                        let var_ty = scheme.instantiate(|| self.fresh_var());
                        self.unify_or_error(&var_ty, &value_ty, span);
                    }
                    None => {
                        // Check for similar names first
                        let suggestion =
                            find_similar(&name_str, self.env.iter().map(|(n, _)| n.as_str()), 2);
                        // If no similar name found, suggest using := to define
                        let suggestion = suggestion.or_else(|| {
                            Some(format!(
                                "use `:=` to define a new variable: `{name_str} := ...`"
                            ))
                        });
                        self.diagnostics.push(InferDiagnostic::Undefined {
                            name: name_str,
                            span: to_span(span),
                            suggestion,
                        });
                    }
                }
            }
            Item::IndexAssignment {
                collection,
                index,
                value,
            } => {
                let (collection_ty, coll_span) = self.infer_expr_idx(*collection);
                let (index_ty, idx_span) = self.infer_expr_idx(*index);
                let value_ty = self.infer_expr(value, span);
                // Type check: collection should be list, index should be int
                let elem_ty = Type::Var(self.fresh_var());
                self.unify_or_error(
                    &collection_ty,
                    &Type::List(Box::new(elem_ty.clone())),
                    coll_span,
                );
                self.unify_or_error(&index_ty, &Type::Integer, idx_span);
                self.unify_or_error(&value_ty, &elem_ty, span);
            }
            Item::StructDefinition(struct_def) => {
                // Register struct definition with field types
                let name_str = self.resolve(struct_def.name).to_string();
                let struct_id = self.register_struct(&name_str);

                // Parse and store field types
                let fields: Vec<(String, Type)> = struct_def
                    .fields
                    .iter()
                    .map(|f| {
                        let field_name = self.resolve(f.name).to_string();
                        let field_type = self.parse_type_name(self.resolve(f.ty));
                        (field_name, field_type)
                    })
                    .collect();

                // Update StructInfo with field information
                if let Some((_, info)) = self.structs.get_mut(&name_str) {
                    info.fields = fields;
                }

                let ty = Type::Struct(struct_id);
                // Store a constructor-like scheme for the struct
                let scheme = Scheme::mono(ty);
                self.env.insert(name_str, scheme);
            }
            Item::FieldAssignment {
                object,
                field,
                value,
            } => {
                let (obj_ty, obj_span) = self.infer_expr_idx(*object);
                let value_ty = self.infer_expr(value, span);
                let field_name = self.resolve(*field).to_string();

                match self.subst.apply(&obj_ty) {
                    Type::Struct(struct_id) => {
                        // Look up the expected field type and unify
                        if let Some(field_ty) = self.lookup_field_type(struct_id, &field_name) {
                            self.unify_or_error(&field_ty, &value_ty, span);
                        } else {
                            self.diagnostics.push(InferDiagnostic::UndefinedField {
                                struct_name: self
                                    .lookup_struct_by_id(struct_id)
                                    .map(|i| i.name.clone())
                                    .unwrap_or_else(|| "unknown".to_string()),
                                field_name,
                                span: to_span(span),
                            });
                        }
                    }
                    other => {
                        self.diagnostics.push(InferDiagnostic::TypeMismatch {
                            expected: "struct".to_string(),
                            found: format!("{}", other),
                            span: to_span(obj_span),
                        });
                    }
                }
            }
            Item::Expression(expr) => {
                self.infer_expr(expr, span);
            }
        }
    }

    fn infer_expr(&mut self, expr: &Expression, span: TextRange) -> Type {
        match expr {
            Expression::Missing => Type::Error,
            Expression::Literal(lit) => literal_type(lit),
            Expression::VariableRef { name } => {
                let name_str = self.resolve(*name).to_string();
                self.lookup(&name_str, span)
            }
            Expression::Infix { op, lhs, rhs } => self.infer_infix(*op, *lhs, *rhs),
            Expression::Prefix { op, expr } => self.infer_prefix(*op, *expr),
            Expression::Block { items, tail } => self.infer_block(items, *tail),
            Expression::If {
                condition,
                then_branch,
                else_branch,
            } => self.infer_if(*condition, *then_branch, *else_branch),
            Expression::Function {
                params,
                return_type,
                body,
                ..
            } => {
                let fn_ty = self.infer_function(None, params, *body, span);

                // Check explicit return annotation if present
                if let Some(ret_name) = return_type {
                    let annotated_ret = self.parse_type_name(self.resolve(*ret_name));
                    if let Type::Function { ret, .. } = &fn_ty {
                        self.unify_or_error(&annotated_ret, ret, span);
                    }
                }

                fn_ty
            }
            Expression::Call { callee, args } => self.infer_call(*callee, args, span),
            Expression::Return { value } => self.infer_return(*value, span),
            Expression::Echo { value } => {
                self.infer_expr_idx(*value);
                Type::Unit
            }
            Expression::Loop { body, .. } => {
                self.infer_expr_idx(*body);
                Type::Unit // loops return unit (break value support later)
            }
            Expression::While {
                condition, body, ..
            } => {
                let (cond_ty, cond_span) = self.infer_expr_idx(*condition);
                self.unify_or_error(&cond_ty, &Type::Boolean, cond_span);
                self.infer_expr_idx(*body);
                Type::Unit
            }
            Expression::For {
                binding,
                iterable,
                body,
                ..
            } => {
                self.env.push_scope();

                // Infer iterable type
                let (iterable_ty, iterable_span) = self.infer_expr_idx(*iterable);

                // Determine binding type based on iterable
                let binding_ty = match &iterable_ty {
                    // Range produces integers
                    Type::Var(_) => {
                        // Could be range (Int) or list (elem type)
                        // Check if iterable is a Range expression
                        let iterable_expr = &self.hir.expressions[*iterable];
                        if matches!(iterable_expr, Expression::Range { .. }) {
                            Type::Integer
                        } else {
                            // Assume list, binding is element type
                            let elem_var = Type::Var(self.fresh_var());
                            self.unify_or_error(
                                &iterable_ty,
                                &Type::List(Box::new(elem_var.clone())),
                                iterable_span,
                            );
                            self.subst.apply(&elem_var)
                        }
                    }
                    Type::List(elem) => (**elem).clone(),
                    _ => {
                        // For range expressions, binding is Int
                        Type::Integer
                    }
                };

                // Bind the loop variable
                let binding_name = self.resolve(*binding).to_string();
                self.env
                    .insert(binding_name, Scheme::mono(binding_ty.clone()));

                // Infer body
                self.infer_expr_idx(*body);

                self.env.pop_scope();
                Type::Unit
            }
            Expression::Range { start, end } => {
                // Both start and end must be integers
                let (start_ty, start_span) = self.infer_expr_idx(*start);
                let (end_ty, end_span) = self.infer_expr_idx(*end);
                self.unify_or_error(&start_ty, &Type::Integer, start_span);
                self.unify_or_error(&end_ty, &Type::Integer, end_span);
                // Range produces integers (used in for loops)
                Type::Integer
            }
            Expression::Break { .. } => Type::Unit,
            Expression::Continue { .. } => Type::Unit,
            Expression::List { elements } => self.infer_list(elements),
            Expression::Index { collection, index } => self.infer_index(*collection, *index, span),
            Expression::Slice {
                collection,
                start,
                end,
            } => self.infer_slice(*collection, *start, *end, span),
            Expression::Tuple { elements } => self.infer_tuple(elements),
            Expression::TupleAccess { tuple, index } => {
                self.infer_tuple_access(*tuple, *index, span)
            }
            Expression::Struct {
                name,
                fields,
                spread,
            } => {
                let name_str = self.resolve(*name).to_string();
                // Look up or register the struct type
                let struct_id = if let Some((id, _)) = self.lookup_struct(&name_str) {
                    id
                } else {
                    self.register_struct(&name_str)
                };

                // Collect provided field names
                let mut provided_fields = std::collections::HashSet::new();

                // Infer and check types for all field values
                for (field_name_ident, field_expr) in fields {
                    let field_name = self.resolve(*field_name_ident).to_string();
                    let (field_value_ty, field_span) = self.infer_expr_idx(*field_expr);

                    provided_fields.insert(field_name.clone());

                    // Check that field value matches declared field type
                    if let Some(expected_ty) = self.lookup_field_type(struct_id, &field_name) {
                        self.unify_or_error(&expected_ty, &field_value_ty, field_span);
                    } else {
                        self.diagnostics.push(InferDiagnostic::UndefinedField {
                            struct_name: name_str.clone(),
                            field_name,
                            span: to_span(field_span),
                        });
                    }
                }

                // If there's a spread, it should be the same struct type
                if let Some(spread_expr) = spread {
                    let (spread_ty, spread_span) = self.infer_expr_idx(*spread_expr);
                    self.unify_or_error(&spread_ty, &Type::Struct(struct_id), spread_span);
                } else {
                    // No spread - check that all required fields are provided
                    let missing: Vec<String> = self
                        .lookup_struct_by_id(struct_id)
                        .map(|info| {
                            info.fields
                                .iter()
                                .filter(|(name, _)| !provided_fields.contains(name))
                                .map(|(name, _)| name.clone())
                                .collect()
                        })
                        .unwrap_or_default();

                    for field_name in missing {
                        self.diagnostics.push(InferDiagnostic::MissingField {
                            struct_name: name_str.clone(),
                            field_name,
                            span: to_span(span),
                        });
                    }
                }

                Type::Struct(struct_id)
            }
            Expression::FieldAccess { object, field } => {
                let (obj_ty, obj_span) = self.infer_expr_idx(*object);
                let field_name = self.resolve(*field).to_string();
                let resolved = self.subst.apply(&obj_ty);

                match resolved {
                    Type::Struct(struct_id) => {
                        // Look up the field type
                        if let Some(field_ty) = self.lookup_field_type(struct_id, &field_name) {
                            field_ty
                        } else {
                            self.diagnostics.push(InferDiagnostic::UndefinedField {
                                struct_name: self
                                    .lookup_struct_by_id(struct_id)
                                    .map(|i| i.name.clone())
                                    .unwrap_or_else(|| "unknown".to_string()),
                                field_name,
                                span: to_span(span),
                            });
                            Type::Error
                        }
                    }
                    Type::Var(_) => {
                        // Can't statically determine struct type yet
                        Type::Var(self.fresh_var())
                    }
                    _ => {
                        self.diagnostics.push(InferDiagnostic::TypeMismatch {
                            expected: "struct".to_string(),
                            found: resolved.to_string(),
                            span: to_span(obj_span),
                        });
                        Type::Error
                    }
                }
            }
        }
    }

    fn infer_expr_idx(&mut self, idx: ExprIdx) -> (Type, TextRange) {
        let expr = &self.hir.expressions[idx];
        let span = self.hir.expr_spans.get(idx).copied().unwrap_or_default();
        let ty = self.infer_expr(expr, span);
        self.expr_types.insert(idx, ty.clone());
        (ty, span)
    }

    fn infer_infix(&mut self, op: hir::InfixOp, lhs: ExprIdx, rhs: ExprIdx) -> Type {
        let sig = ops::infix_signature(op);
        let (lhs_ty, lhs_span) = self.infer_expr_idx(lhs);
        let (rhs_ty, rhs_span) = self.infer_expr_idx(rhs);

        // Check for wrong operator (int op on floats or float op on ints)
        let lhs_resolved = self.subst.apply(&lhs_ty);
        let rhs_resolved = self.subst.apply(&rhs_ty);

        // Only emit WrongOperator if both operands have same type but wrong for operator
        if lhs_resolved == rhs_resolved && lhs_resolved != sig.operand {
            // Check if there's a suggested operator
            let suggested = if sig.operand == Type::Integer && lhs_resolved == Type::Float {
                ops::int_to_float_op(op)
            } else if sig.operand == Type::Float && lhs_resolved == Type::Integer {
                ops::float_to_int_op(op)
            } else {
                None
            };

            if let Some(suggest_op) = suggested {
                let op_span = self.hir.expr_spans.get(lhs).copied().unwrap_or_default();
                self.diagnostics.push(InferDiagnostic::WrongOperator {
                    op: ops::op_symbol(op).to_string(),
                    expected_type: sig.operand.to_string(),
                    actual_type: lhs_resolved.to_string(),
                    span: to_span(op_span),
                    suggest_op: format!(
                        "use `{}` for {} operations",
                        ops::op_symbol(suggest_op),
                        lhs_resolved
                    ),
                });
                return sig.result;
            }
        }

        self.unify_or_error(&lhs_ty, &sig.operand, lhs_span);
        self.unify_or_error(&rhs_ty, &sig.operand, rhs_span);

        sig.result
    }

    fn infer_prefix(&mut self, op: hir::PrefixOp, expr: ExprIdx) -> Type {
        let sig = ops::prefix_signature(op);
        let (expr_ty, expr_span) = self.infer_expr_idx(expr);

        match sig {
            ops::PrefixSig::Preserve(allowed) => {
                // Check if expr_ty can unify with any allowed type
                let expr_ty_resolved = self.subst.apply(&expr_ty);
                let matches = allowed.iter().any(|t| {
                    // Try unification without committing
                    unify(&expr_ty_resolved, t).is_ok()
                });

                if !matches && !expr_ty_resolved.is_error() {
                    self.diagnostics.push(InferDiagnostic::Mismatch {
                        expected: format!("{} or {}", allowed[0], allowed[1]),
                        found: expr_ty_resolved.to_string(),
                        span: to_span(expr_span),
                    });
                }
                expr_ty
            }
            ops::PrefixSig::Fixed { operand, result } => {
                self.unify_or_error(&expr_ty, &operand, expr_span);
                result
            }
        }
    }

    fn infer_block(&mut self, items: &[BlockItem], tail: Option<ExprIdx>) -> Type {
        self.env.push_scope();

        for item in items {
            match item {
                BlockItem::Definition { name, value } => {
                    let name_str = self.resolve(*name).to_string();
                    let (ty, _) = self.infer_expr_idx(*value);
                    let ty = self.subst.apply(&ty);
                    let scheme = Scheme::generalize(&self.env, &ty);
                    self.env.insert(name_str, scheme);
                }
                BlockItem::Assignment { name, value } => {
                    let name_str = self.resolve(*name).to_string();
                    let (value_ty, span) = self.infer_expr_idx(*value);
                    let scheme = self.env.lookup(&name_str).cloned();
                    match scheme {
                        Some(scheme) => {
                            let var_ty = scheme.instantiate(|| self.fresh_var());
                            self.unify_or_error(&var_ty, &value_ty, span);
                        }
                        None => {
                            let suggestion = find_similar(
                                &name_str,
                                self.env.iter().map(|(n, _)| n.as_str()),
                                2,
                            );
                            let suggestion = suggestion.or_else(|| {
                                Some(format!(
                                    "use `:=` to define a new variable: `{name_str} := ...`"
                                ))
                            });
                            self.diagnostics.push(InferDiagnostic::Undefined {
                                name: name_str,
                                span: to_span(span),
                                suggestion,
                            });
                        }
                    }
                }
                BlockItem::Expression(idx) => {
                    self.infer_expr_idx(*idx);
                }
                BlockItem::Return { value } => {
                    let span = value
                        .map(|idx| self.hir.expr_spans.get(idx).copied().unwrap_or_default())
                        .unwrap_or_default();
                    self.infer_return(*value, span);
                }
                BlockItem::Echo { value } => {
                    self.infer_expr_idx(*value);
                }
                BlockItem::Break { .. } => {}
                BlockItem::Continue { .. } => {}
                BlockItem::IndexAssignment {
                    collection,
                    index,
                    value,
                } => {
                    let (collection_ty, coll_span) = self.infer_expr_idx(*collection);
                    let (index_ty, idx_span) = self.infer_expr_idx(*index);
                    let (value_ty, val_span) = self.infer_expr_idx(*value);
                    let elem_ty = Type::Var(self.fresh_var());
                    self.unify_or_error(
                        &collection_ty,
                        &Type::List(Box::new(elem_ty.clone())),
                        coll_span,
                    );
                    self.unify_or_error(&index_ty, &Type::Integer, idx_span);
                    self.unify_or_error(&value_ty, &elem_ty, val_span);
                }
                BlockItem::FieldAssignment {
                    object,
                    field,
                    value,
                } => {
                    let (obj_ty, obj_span) = self.infer_expr_idx(*object);
                    let (value_ty, val_span) = self.infer_expr_idx(*value);
                    let field_name = self.resolve(*field).to_string();

                    match self.subst.apply(&obj_ty) {
                        Type::Struct(struct_id) => {
                            // Look up the expected field type and unify
                            if let Some(field_ty) = self.lookup_field_type(struct_id, &field_name) {
                                self.unify_or_error(&field_ty, &value_ty, val_span);
                            } else {
                                self.diagnostics.push(InferDiagnostic::UndefinedField {
                                    struct_name: self
                                        .lookup_struct_by_id(struct_id)
                                        .map(|i| i.name.clone())
                                        .unwrap_or_else(|| "unknown".to_string()),
                                    field_name,
                                    span: to_span(val_span),
                                });
                            }
                        }
                        other => {
                            self.diagnostics.push(InferDiagnostic::TypeMismatch {
                                expected: "struct".to_string(),
                                found: format!("{}", other),
                                span: to_span(obj_span),
                            });
                        }
                    }
                }
            }
        }

        // Block type is the tail expression type, or Unit if no tail
        let result = match tail {
            Some(idx) => {
                let (ty, _) = self.infer_expr_idx(idx);
                ty
            }
            None => Type::Unit,
        };

        self.env.pop_scope();
        result
    }

    fn infer_if(&mut self, cond: ExprIdx, then_br: ExprIdx, else_br: Option<ExprIdx>) -> Type {
        let (cond_ty, cond_span) = self.infer_expr_idx(cond);
        self.unify_or_error(&cond_ty, &Type::Boolean, cond_span);

        let (then_ty, _) = self.infer_expr_idx(then_br);

        match else_br {
            Some(idx) => {
                let (else_ty, else_span) = self.infer_expr_idx(idx);
                self.unify_or_error(&then_ty, &else_ty, else_span);
                then_ty
            }
            None => Type::Unit,
        }
    }

    fn infer_list(&mut self, elements: &[ExprIdx]) -> Type {
        if elements.is_empty() {
            // Empty list: list['a] with fresh type var
            Type::List(Box::new(Type::Var(self.fresh_var())))
        } else {
            // Infer first element type, unify all others with it
            let (first_ty, _) = self.infer_expr_idx(elements[0]);
            for &elem_idx in &elements[1..] {
                let (elem_ty, elem_span) = self.infer_expr_idx(elem_idx);
                self.unify_or_error(&first_ty, &elem_ty, elem_span);
            }
            Type::List(Box::new(self.subst.apply(&first_ty)))
        }
    }

    fn infer_index(&mut self, collection: ExprIdx, index: ExprIdx, _span: TextRange) -> Type {
        let (coll_ty, coll_span) = self.infer_expr_idx(collection);
        let (idx_ty, idx_span) = self.infer_expr_idx(index);

        // Index must be int
        self.unify_or_error(&idx_ty, &Type::Integer, idx_span);

        // Collection must be list[T], return T
        let elem_var = Type::Var(self.fresh_var());
        let expected_list = Type::List(Box::new(elem_var.clone()));
        self.unify_or_error(&coll_ty, &expected_list, coll_span);

        // Also support string indexing -> string
        let resolved = self.subst.apply(&coll_ty);
        if resolved == Type::String {
            return Type::String;
        }

        self.subst.apply(&elem_var)
    }

    fn infer_slice(
        &mut self,
        collection: ExprIdx,
        start: Option<ExprIdx>,
        end: Option<ExprIdx>,
        _span: TextRange,
    ) -> Type {
        let (coll_ty, coll_span) = self.infer_expr_idx(collection);

        // Start and end must be int if present
        if let Some(start_idx) = start {
            let (start_ty, start_span) = self.infer_expr_idx(start_idx);
            self.unify_or_error(&start_ty, &Type::Integer, start_span);
        }
        if let Some(end_idx) = end {
            let (end_ty, end_span) = self.infer_expr_idx(end_idx);
            self.unify_or_error(&end_ty, &Type::Integer, end_span);
        }

        // Collection must be list[T], slice returns list[T]
        let elem_var = Type::Var(self.fresh_var());
        let expected_list = Type::List(Box::new(elem_var.clone()));
        self.unify_or_error(&coll_ty, &expected_list, coll_span);

        // Also support string slicing -> string
        let resolved = self.subst.apply(&coll_ty);
        if resolved == Type::String {
            return Type::String;
        }

        self.subst.apply(&expected_list)
    }

    fn infer_tuple(&mut self, elements: &[ExprIdx]) -> Type {
        let elem_types: Vec<Type> = elements
            .iter()
            .map(|&idx| {
                let (ty, _) = self.infer_expr_idx(idx);
                self.subst.apply(&ty)
            })
            .collect();
        Type::Tuple(elem_types)
    }

    fn infer_tuple_access(&mut self, tuple: ExprIdx, index: u32, span: TextRange) -> Type {
        let (tuple_ty, tuple_span) = self.infer_expr_idx(tuple);
        let resolved = self.subst.apply(&tuple_ty);

        match resolved {
            Type::Tuple(elems) => {
                if (index as usize) < elems.len() {
                    elems[index as usize].clone()
                } else {
                    self.diagnostics
                        .push(InferDiagnostic::TupleIndexOutOfBounds {
                            index,
                            tuple_size: elems.len(),
                            span: to_span(span),
                        });
                    Type::Error
                }
            }
            Type::Var(_) => {
                // Can't statically determine tuple type yet
                // Return a fresh type var
                Type::Var(self.fresh_var())
            }
            _ => {
                self.diagnostics.push(InferDiagnostic::TypeMismatch {
                    expected: "tuple".to_string(),
                    found: resolved.to_string(),
                    span: to_span(tuple_span),
                });
                Type::Error
            }
        }
    }

    fn lookup(&mut self, name: &str, span: TextRange) -> Type {
        // Clone scheme to avoid borrow conflict with fresh_var
        let scheme = self.env.lookup(name).cloned();
        match scheme {
            Some(scheme) => scheme.instantiate(|| self.fresh_var()),
            None => {
                let suggestion = find_similar(name, self.env.iter().map(|(n, _)| n.as_str()), 2);
                self.diagnostics.push(InferDiagnostic::Undefined {
                    name: name.to_string(),
                    span: to_span(span),
                    suggestion,
                });
                Type::Error
            }
        }
    }

    fn unify_or_error(&mut self, t1: &Type, t2: &Type, span: TextRange) {
        let t1 = self.subst.apply(t1);
        let t2 = self.subst.apply(t2);

        match unify(&t1, &t2) {
            Ok(s) => {
                self.subst = s.compose(&self.subst);
            }
            Err(e) => {
                self.report_unify_error(e, span);
            }
        }
    }

    fn report_unify_error(&mut self, error: UnifyError, span: TextRange) {
        let span = to_span(span);
        let diagnostic = match error {
            UnifyError::Mismatch { expected, found } => InferDiagnostic::Mismatch {
                expected: expected.to_string(),
                found: found.to_string(),
                span,
            },
            UnifyError::InfiniteType { var, ty } => InferDiagnostic::InfiniteType {
                var: var.to_string(),
                ty: ty.to_string(),
                span,
            },
            UnifyError::ArityMismatch { expected, found } => InferDiagnostic::ArityMismatch {
                expected,
                found,
                span,
            },
        };
        self.diagnostics.push(diagnostic);
    }

    fn finalize(self) -> InferenceResult {
        // Apply final substitution to all expression types
        let expression_types: ArenaMap<_, _> = self
            .expr_types
            .iter()
            .map(|(idx, ty)| (idx, self.subst.apply(ty)))
            .collect();

        // Extract variable types from env, applying substitution
        let variable_types: HashMap<_, _> = self
            .env
            .iter()
            .map(|(name, scheme)| {
                let ty = self.subst.apply(&scheme.ty);
                (name.clone(), ty)
            })
            .collect();

        InferenceResult {
            expression_types,
            variable_types,
            diagnostics: self.diagnostics,
        }
    }

    fn finalize_with_state(self) -> InferWithState {
        let expression_types: ArenaMap<_, _> = self
            .expr_types
            .iter()
            .map(|(idx, ty)| (idx, self.subst.apply(ty)))
            .collect();

        let variable_types: HashMap<_, _> = self
            .env
            .iter()
            .map(|(name, scheme)| {
                let ty = self.subst.apply(&scheme.ty);
                (name.clone(), ty)
            })
            .collect();

        // Apply substitution to env schemes for the returned state
        let mut updated_env = TypeEnv::new();
        for (name, scheme) in self.env.iter() {
            let applied_ty = self.subst.apply(&scheme.ty);
            // Re-generalize with the applied type
            let new_scheme = Scheme::generalize(&updated_env, &applied_ty);
            updated_env.insert(name.clone(), new_scheme);
        }

        InferWithState {
            result: InferenceResult {
                expression_types,
                variable_types,
                diagnostics: self.diagnostics,
            },
            state: InferState {
                env: updated_env,
                next_var: self.next_var,
            },
        }
    }
}

fn literal_type(lit: &Literal) -> Type {
    match lit {
        Literal::Integer(_) => Type::Integer,
        Literal::Float(_) => Type::Float,
        Literal::Boolean(_) => Type::Boolean,
        Literal::String(_) => Type::String,
    }
}
