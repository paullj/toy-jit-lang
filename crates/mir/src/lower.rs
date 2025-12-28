use std::collections::HashMap;

use hir::{BlockItem, Definition, ExprIdx, Expression, Ident, InfixOp, Item, Literal, PrefixOp};
use infer::{InferenceResult, Type};

use crate::ir::{
    Block, BlockId, CapturedVar, FuncId, Function, Inst, LocalId, Module, Operand, VReg, ValueType,
};

pub fn lower(hir: &hir::LowerResult, types: &InferenceResult) -> Module {
    let mut ctx = LowerCtx::new(hir, types);
    ctx.lower_module();
    ctx.finish()
}

/// Tracks loop context for break/continue targets
#[derive(Clone)]
struct LoopContext {
    label: Option<String>,
    continue_bb: BlockId,
    break_bb: BlockId,
}

/// Saved context for nested function lowering
struct SavedContext {
    blocks: Vec<Block>,
    current_block: BlockId,
    next_vreg: u32,
    next_local: u32,
    next_block: u32,
    local_scopes: Vec<HashMap<String, LocalId>>,
    capture_map: HashMap<String, u32>,
    loop_stack: Vec<LoopContext>,
}

struct LowerCtx<'a> {
    hir: &'a hir::LowerResult,
    types: &'a InferenceResult,

    // Current function being lowered
    current_func_id: FuncId,
    blocks: Vec<Block>,
    current_block: Block,
    next_vreg: u32,
    next_local: u32,
    next_block: u32,

    /// Stack of scopes, each scope maps names to local IDs
    local_scopes: Vec<HashMap<String, LocalId>>,

    // All functions in module
    functions: Vec<Function>,
    next_func_id: u32,

    // Function name -> FuncId mapping for direct calls
    func_names: HashMap<String, FuncId>,

    // Closure context
    capture_map: HashMap<String, u32>, // name -> capture index (for inside closure body)

    // Loop context stack for break/continue
    loop_stack: Vec<LoopContext>,
}

impl<'a> LowerCtx<'a> {
    fn new(hir: &'a hir::LowerResult, types: &'a InferenceResult) -> Self {
        Self {
            hir,
            types,
            current_func_id: FuncId(0),
            blocks: Vec::new(),
            current_block: Block::new(BlockId(0)),
            next_vreg: 0,
            next_local: 0,
            next_block: 1,
            local_scopes: vec![HashMap::new()],
            functions: Vec::new(),
            next_func_id: 0,
            func_names: HashMap::new(),
            capture_map: HashMap::new(),
            loop_stack: Vec::new(),
        }
    }

    /// Resolve an Ident to its string representation
    fn resolve(&self, ident: Ident) -> &str {
        self.hir.resolve(ident)
    }

    /// Get ValueType for an expression index (for echo)
    fn get_value_type(&self, expr_idx: ExprIdx) -> ValueType {
        match self.types.expression_types.get(expr_idx) {
            Some(Type::Integer) => ValueType::Int,
            Some(Type::Float) => ValueType::Float,
            Some(Type::Boolean) => ValueType::Bool,
            // Lists, closures, strings, etc. are already NaN-boxed
            _ => ValueType::Boxed,
        }
    }

    fn save_context(&mut self) -> SavedContext {
        SavedContext {
            blocks: std::mem::take(&mut self.blocks),
            current_block: self.current_block.id,
            next_vreg: self.next_vreg,
            next_local: self.next_local,
            next_block: self.next_block,
            local_scopes: std::mem::take(&mut self.local_scopes),
            capture_map: std::mem::take(&mut self.capture_map),
            loop_stack: std::mem::take(&mut self.loop_stack),
        }
    }

    fn restore_context(&mut self, saved: SavedContext) {
        self.blocks = saved.blocks;
        self.current_block = Block::new(saved.current_block);
        self.next_vreg = saved.next_vreg;
        self.next_local = saved.next_local;
        self.next_block = saved.next_block;
        self.local_scopes = saved.local_scopes;
        self.capture_map = saved.capture_map;
        self.loop_stack = saved.loop_stack;
    }

    fn find_loop(&self, label: Option<&str>) -> Option<&LoopContext> {
        match label {
            None => self.loop_stack.last(),
            Some(target) => self
                .loop_stack
                .iter()
                .rev()
                .find(|ctx| ctx.label.as_deref() == Some(target)),
        }
    }

    fn alloc_func_id(&mut self) -> FuncId {
        let id = FuncId(self.next_func_id);
        self.next_func_id += 1;
        id
    }

    fn create_block(&mut self) -> BlockId {
        let id = BlockId(self.next_block);
        self.next_block += 1;
        id
    }

    fn switch_to_block(&mut self, id: BlockId) {
        if !self.current_block.insts.is_empty() || self.current_block.id.0 != id.0 {
            let old_block = std::mem::replace(&mut self.current_block, Block::new(id));
            self.blocks.push(old_block);
        } else {
            self.current_block = Block::new(id);
        }
    }

    fn push_scope(&mut self) {
        self.local_scopes.push(HashMap::new());
    }

    fn pop_scope(&mut self) {
        if self.local_scopes.len() > 1 {
            self.local_scopes.pop();
        }
    }

    fn fresh_vreg(&mut self) -> VReg {
        let v = VReg(self.next_vreg);
        self.next_vreg += 1;
        v
    }

    fn alloc_local(&mut self, name: &str) -> LocalId {
        let id = LocalId(self.next_local);
        self.next_local += 1;
        if let Some(scope) = self.local_scopes.last_mut() {
            scope.insert(name.to_string(), id);
        }
        id
    }

    fn get_local(&self, name: &str) -> Option<LocalId> {
        for scope in self.local_scopes.iter().rev() {
            if let Some(&id) = scope.get(name) {
                return Some(id);
            }
        }
        None
    }

    fn emit(&mut self, inst: Inst) {
        // Don't emit anything after a terminator (block is already sealed)
        if self.current_block.is_terminated() {
            return;
        }
        self.current_block.push(inst);
    }

    fn lower_module(&mut self) {
        // First pass: register all top-level functions so we know their FuncIds
        for item in &self.hir.items {
            if let Item::Definition(Definition::Function { name, .. }) = item {
                let func_id = self.alloc_func_id();
                let name_str = self.resolve(*name).to_string();
                self.func_names.insert(name_str, func_id);
            }
        }

        // Main function gets the next ID
        let main_id = self.alloc_func_id();
        self.current_func_id = main_id;

        // Second pass: lower all function definitions first
        for item in self.hir.items.clone() {
            if let Item::Definition(Definition::Function {
                name, params, body, ..
            }) = &item
            {
                let name_str = self.resolve(*name).to_string();
                let func_id = self.func_names[&name_str];
                self.lower_function_def(func_id, Some(name_str), params, *body);
            }
        }

        // Third pass: lower main body (non-function items)
        self.current_func_id = main_id;
        self.blocks = Vec::new();
        self.current_block = Block::new(BlockId(0));
        self.next_vreg = 0;
        self.next_local = 0;
        self.next_block = 1;
        self.local_scopes = vec![HashMap::new()];

        let mut last_expr_value: Option<Operand> = None;

        for item in &self.hir.items {
            match item {
                Item::Definition(Definition::Function { name, .. }) => {
                    // Allocate local for function reference (for indirect calls)
                    let name_str = self.resolve(*name).to_string();
                    let func_id = self.func_names[&name_str];
                    let local = self.alloc_local(&name_str);
                    let dst = self.fresh_vreg();
                    // Store function reference as a closure with no captures
                    self.emit(Inst::MakeClosure {
                        dst,
                        func: func_id,
                        captures: vec![],
                    });
                    self.emit(Inst::StoreLocal {
                        local,
                        src: Operand::VReg(dst),
                    });
                    last_expr_value = None;
                }
                Item::Definition(Definition::Variable { name, value }) => {
                    let name_str = self.resolve(*name).to_string();
                    let operand = self.lower_expr(value);
                    let local = self.alloc_local(&name_str);
                    self.emit(Inst::StoreLocal {
                        local,
                        src: operand,
                    });
                    last_expr_value = None;
                }
                Item::Assignment { name, value } => {
                    let name_str = self.resolve(*name).to_string();
                    let operand = self.lower_expr(value);
                    if let Some(local) = self.get_local(&name_str) {
                        self.emit(Inst::StoreLocal {
                            local,
                            src: operand,
                        });
                    }
                    last_expr_value = None;
                }
                Item::IndexAssignment {
                    collection,
                    index,
                    value,
                } => {
                    let list = self.lower_expr_idx(*collection);
                    let idx = self.lower_expr_idx(*index);
                    let val = self.lower_expr(value);
                    self.emit(Inst::ListSet {
                        list,
                        index: idx,
                        value: val,
                    });
                    last_expr_value = None;
                }
                Item::Expression(expr) => {
                    last_expr_value = Some(self.lower_expr(expr));
                }
            }
        }

        self.emit(Inst::Return {
            value: last_expr_value,
        });

        // Build main function
        self.blocks.push(std::mem::replace(
            &mut self.current_block,
            Block::new(BlockId(0)),
        ));
        let main_func = Function {
            id: main_id,
            name: Some("main".to_string()),
            params: Vec::new(),
            param_count: 0,
            blocks: std::mem::take(&mut self.blocks),
            local_count: self.next_local,
            vreg_count: self.next_vreg,
            captures: Vec::new(),
            is_closure: false,
        };
        self.functions.push(main_func);
    }

    fn lower_function_def(
        &mut self,
        func_id: FuncId,
        name: Option<String>,
        params: &[hir::FunctionParam],
        body: ExprIdx,
    ) {
        // Save current context
        let saved = self.save_context();
        let saved_func_id = self.current_func_id;

        // Initialize for new function
        self.current_func_id = func_id;
        self.blocks = Vec::new();
        self.current_block = Block::new(BlockId(0));
        self.next_vreg = 0;
        self.next_local = 0;
        self.next_block = 1;
        self.local_scopes = vec![HashMap::new()];
        self.capture_map.clear();

        // Allocate params as first locals
        let param_locals: Vec<LocalId> = params
            .iter()
            .map(|p| {
                let local = LocalId(self.next_local);
                self.next_local += 1;
                let name_str = self.resolve(p.name).to_string();
                self.local_scopes
                    .last_mut()
                    .unwrap()
                    .insert(name_str, local);
                local
            })
            .collect();

        // Lower body
        let result = self.lower_expr_idx(body);

        // Emit return with body result
        self.emit(Inst::Return {
            value: Some(result),
        });

        // Build function
        self.blocks.push(std::mem::replace(
            &mut self.current_block,
            Block::new(BlockId(0)),
        ));
        let func = Function {
            id: func_id,
            name,
            params: param_locals.clone(),
            param_count: params.len() as u32,
            blocks: std::mem::take(&mut self.blocks),
            local_count: self.next_local,
            vreg_count: self.next_vreg,
            captures: Vec::new(),
            is_closure: false,
        };
        self.functions.push(func);

        // Restore context
        self.restore_context(saved);
        self.current_func_id = saved_func_id;
    }

    fn lower_lambda(
        &mut self,
        params: &[hir::FunctionParam],
        body: ExprIdx,
        captures: &[Ident],
    ) -> Operand {
        let func_id = self.alloc_func_id();

        // Collect capture operands from current scope BEFORE switching context
        let capture_ops: Vec<Operand> = captures
            .iter()
            .filter_map(|name| {
                let name_str = self.resolve(*name);
                if let Some(local) = self.get_local(name_str) {
                    let dst = self.fresh_vreg();
                    self.emit(Inst::LoadLocal { dst, local });
                    Some(Operand::VReg(dst))
                } else {
                    None
                }
            })
            .collect();

        let capture_vars: Vec<CapturedVar> = captures
            .iter()
            .filter_map(|name| {
                let name_str = self.resolve(*name);
                self.get_local(name_str).map(|local| CapturedVar {
                    name: name_str.to_string(),
                    outer_local: local,
                })
            })
            .collect();

        // Save current context
        let saved = self.save_context();
        let saved_func_id = self.current_func_id;

        // Initialize for closure
        self.current_func_id = func_id;
        self.blocks = Vec::new();
        self.current_block = Block::new(BlockId(0));
        self.next_vreg = 0;
        self.next_local = 0;
        self.next_block = 1;
        self.local_scopes = vec![HashMap::new()];

        // Setup capture map for LoadCapture in closure body
        self.capture_map.clear();
        for (i, cap) in capture_vars.iter().enumerate() {
            self.capture_map.insert(cap.name.clone(), i as u32);
        }

        // Allocate params as first locals
        let param_locals: Vec<LocalId> = params
            .iter()
            .map(|p| {
                let local = LocalId(self.next_local);
                self.next_local += 1;
                let name_str = self.resolve(p.name).to_string();
                self.local_scopes
                    .last_mut()
                    .unwrap()
                    .insert(name_str, local);
                local
            })
            .collect();

        // Lower body
        let result = self.lower_expr_idx(body);

        // Emit return
        self.emit(Inst::Return {
            value: Some(result),
        });

        // Build closure function
        self.blocks.push(std::mem::replace(
            &mut self.current_block,
            Block::new(BlockId(0)),
        ));
        let func = Function {
            id: func_id,
            name: None,
            params: param_locals,
            param_count: params.len() as u32,
            blocks: std::mem::take(&mut self.blocks),
            local_count: self.next_local,
            vreg_count: self.next_vreg,
            captures: capture_vars,
            is_closure: true,
        };
        self.functions.push(func);

        // Restore context
        self.restore_context(saved);
        self.current_func_id = saved_func_id;

        // Emit MakeClosure in calling context
        let dst = self.fresh_vreg();
        self.emit(Inst::MakeClosure {
            dst,
            func: func_id,
            captures: capture_ops,
        });

        Operand::VReg(dst)
    }

    fn lower_call(&mut self, callee: &Expression, args: &[ExprIdx]) -> Operand {
        // Lower args first
        let arg_ops: Vec<Operand> = args.iter().map(|a| self.lower_expr_idx(*a)).collect();

        let dst = self.fresh_vreg();

        // Check if callee is a direct function reference
        if let Expression::VariableRef { name } = callee {
            let name_str = self.resolve(*name);
            if let Some(&func_id) = self.func_names.get(name_str) {
                // Direct call to known function
                self.emit(Inst::Call {
                    dst: Some(dst),
                    func: func_id,
                    args: arg_ops,
                });
                return Operand::VReg(dst);
            }
        }

        // Otherwise, indirect call through closure
        let callee_op = self.lower_expr(callee);
        self.emit(Inst::CallIndirect {
            dst: Some(dst),
            callee: callee_op,
            args: arg_ops,
        });

        Operand::VReg(dst)
    }

    fn lower_expr(&mut self, expr: &Expression) -> Operand {
        match expr {
            Expression::Missing => Operand::IntConst(0),
            Expression::Literal(lit) => self.lower_literal(lit),
            Expression::Infix { op, lhs, rhs } => self.lower_infix(*op, *lhs, *rhs),
            Expression::Prefix { op, expr } => self.lower_prefix(*op, *expr),
            Expression::VariableRef { name } => {
                let name_str = self.resolve(*name).to_string();
                self.lower_var_ref(&name_str)
            }
            Expression::Block { items, tail } => self.lower_block(items, *tail),
            Expression::If {
                condition,
                then_branch,
                else_branch,
            } => self.lower_if(*condition, *then_branch, *else_branch),
            Expression::Function {
                params,
                body,
                captures,
                ..
            } => self.lower_lambda(params, *body, captures),
            Expression::Call { callee, args } => {
                let callee_expr = &self.hir.expressions[*callee];
                self.lower_call(callee_expr, args)
            }
            Expression::Return { value } => {
                let operand = value.map(|idx| self.lower_expr_idx(idx));
                self.emit(Inst::Return { value: operand });
                Operand::IntConst(0)
            }
            Expression::Echo { value } => {
                let ty = self.get_value_type(*value);
                let operand = self.lower_expr_idx(*value);
                self.emit(Inst::Echo { src: operand, ty });
                Operand::IntConst(0)
            }
            Expression::Loop { label, body } => self.lower_loop(label.as_ref(), *body),
            Expression::While {
                condition,
                label,
                body,
            } => self.lower_while(label.as_ref(), *condition, *body),
            Expression::For {
                binding,
                iterable,
                label,
                body,
            } => self.lower_for(*binding, *iterable, label.as_ref(), *body),
            Expression::Range { start, end } => self.lower_range(*start, *end),
            Expression::Break { label } => {
                let label_str = label.map(|l| self.resolve(l).to_string());
                if let Some(ctx) = self.find_loop(label_str.as_deref()) {
                    let target = ctx.break_bb;
                    self.emit(Inst::Jump { target });
                }
                Operand::IntConst(0)
            }
            Expression::Continue { label } => {
                let label_str = label.map(|l| self.resolve(l).to_string());
                if let Some(ctx) = self.find_loop(label_str.as_deref()) {
                    let target = ctx.continue_bb;
                    self.emit(Inst::Jump { target });
                }
                Operand::IntConst(0)
            }
            Expression::List { elements } => self.lower_list(elements),
            Expression::Index { collection, index } => self.lower_index(*collection, *index),
            Expression::Slice {
                collection,
                start,
                end,
            } => self.lower_slice(*collection, *start, *end),
            Expression::Tuple { elements } => self.lower_tuple(elements),
            Expression::TupleAccess { tuple, index } => self.lower_tuple_access(*tuple, *index),
        }
    }

    fn lower_var_ref(&mut self, name: &str) -> Operand {
        // Check if it's a captured variable (inside closure body)
        if let Some(&idx) = self.capture_map.get(name) {
            let dst = self.fresh_vreg();
            self.emit(Inst::LoadCapture { dst, index: idx });
            return Operand::VReg(dst);
        }

        // Otherwise load from local
        if let Some(local) = self.get_local(name) {
            let dst = self.fresh_vreg();
            self.emit(Inst::LoadLocal { dst, local });
            Operand::VReg(dst)
        } else {
            // Should have been caught by type checker
            Operand::IntConst(0)
        }
    }

    fn lower_expr_idx(&mut self, idx: ExprIdx) -> Operand {
        let expr = self.hir.expressions[idx].clone();
        self.lower_expr(&expr)
    }

    fn lower_literal(&self, lit: &Literal) -> Operand {
        match lit {
            Literal::Integer(n) => Operand::IntConst(*n as i64),
            Literal::Float(f) => Operand::FloatConst(*f),
            Literal::Boolean(b) => Operand::BoolConst(*b),
            Literal::String(s) => Operand::StringConst(s.clone()),
        }
    }

    fn lower_infix(&mut self, op: InfixOp, lhs: ExprIdx, rhs: ExprIdx) -> Operand {
        let lhs_op = self.lower_expr_idx(lhs);
        let rhs_op = self.lower_expr_idx(rhs);
        let dst = self.fresh_vreg();

        let inst = match op {
            InfixOp::Add => Inst::AddInt {
                dst,
                lhs: lhs_op,
                rhs: rhs_op,
            },
            InfixOp::Sub => Inst::SubInt {
                dst,
                lhs: lhs_op,
                rhs: rhs_op,
            },
            InfixOp::Mul => Inst::MulInt {
                dst,
                lhs: lhs_op,
                rhs: rhs_op,
            },
            InfixOp::Div => Inst::DivInt {
                dst,
                lhs: lhs_op,
                rhs: rhs_op,
            },
            InfixOp::Mod => Inst::ModInt {
                dst,
                lhs: lhs_op,
                rhs: rhs_op,
            },

            InfixOp::AddFloat => Inst::AddFloat {
                dst,
                lhs: lhs_op,
                rhs: rhs_op,
            },
            InfixOp::SubFloat => Inst::SubFloat {
                dst,
                lhs: lhs_op,
                rhs: rhs_op,
            },
            InfixOp::MulFloat => Inst::MulFloat {
                dst,
                lhs: lhs_op,
                rhs: rhs_op,
            },
            InfixOp::DivFloat => Inst::DivFloat {
                dst,
                lhs: lhs_op,
                rhs: rhs_op,
            },

            InfixOp::Eq => Inst::EqInt {
                dst,
                lhs: lhs_op,
                rhs: rhs_op,
            },
            InfixOp::NotEq => Inst::NeInt {
                dst,
                lhs: lhs_op,
                rhs: rhs_op,
            },
            InfixOp::Gt => Inst::GtInt {
                dst,
                lhs: lhs_op,
                rhs: rhs_op,
            },
            InfixOp::Lt => Inst::LtInt {
                dst,
                lhs: lhs_op,
                rhs: rhs_op,
            },
            InfixOp::Gte => Inst::GeInt {
                dst,
                lhs: lhs_op,
                rhs: rhs_op,
            },
            InfixOp::Lte => Inst::LeInt {
                dst,
                lhs: lhs_op,
                rhs: rhs_op,
            },

            InfixOp::GtFloat => Inst::GtFloat {
                dst,
                lhs: lhs_op,
                rhs: rhs_op,
            },
            InfixOp::LtFloat => Inst::LtFloat {
                dst,
                lhs: lhs_op,
                rhs: rhs_op,
            },
            InfixOp::GteFloat => Inst::GeFloat {
                dst,
                lhs: lhs_op,
                rhs: rhs_op,
            },
            InfixOp::LteFloat => Inst::LeFloat {
                dst,
                lhs: lhs_op,
                rhs: rhs_op,
            },

            InfixOp::And => Inst::Copy { dst, src: rhs_op },
            InfixOp::Or => Inst::Copy { dst, src: lhs_op },
        };

        self.emit(inst);
        Operand::VReg(dst)
    }

    fn lower_prefix(&mut self, op: PrefixOp, expr: ExprIdx) -> Operand {
        let src = self.lower_expr_idx(expr);
        let dst = self.fresh_vreg();

        let inst = match op {
            PrefixOp::Neg => Inst::NegInt { dst, src },
            PrefixOp::Not => Inst::Not { dst, src },
        };

        self.emit(inst);
        Operand::VReg(dst)
    }

    fn lower_block(&mut self, items: &[BlockItem], tail: Option<ExprIdx>) -> Operand {
        self.push_scope();

        for item in items {
            match item {
                BlockItem::Definition { name, value } => {
                    let name_str = self.resolve(*name).to_string();
                    let operand = self.lower_expr_idx(*value);
                    let local = self.alloc_local(&name_str);
                    self.emit(Inst::StoreLocal {
                        local,
                        src: operand,
                    });
                }
                BlockItem::Assignment { name, value } => {
                    let name_str = self.resolve(*name).to_string();
                    let operand = self.lower_expr_idx(*value);
                    if let Some(local) = self.get_local(&name_str) {
                        self.emit(Inst::StoreLocal {
                            local,
                            src: operand,
                        });
                    }
                }
                BlockItem::IndexAssignment {
                    collection,
                    index,
                    value,
                } => {
                    let list = self.lower_expr_idx(*collection);
                    let idx = self.lower_expr_idx(*index);
                    let val = self.lower_expr_idx(*value);
                    self.emit(Inst::ListSet {
                        list,
                        index: idx,
                        value: val,
                    });
                }
                BlockItem::Expression(idx) => {
                    self.lower_expr_idx(*idx);
                }
                BlockItem::Return { value } => {
                    let operand = value.map(|idx| self.lower_expr_idx(idx));
                    self.emit(Inst::Return { value: operand });
                }
                BlockItem::Echo { value } => {
                    let ty = self.get_value_type(*value);
                    let operand = self.lower_expr_idx(*value);
                    self.emit(Inst::Echo { src: operand, ty });
                }
                BlockItem::Break { label } => {
                    let label_str = label.map(|l| self.resolve(l).to_string());
                    if let Some(ctx) = self.find_loop(label_str.as_deref()) {
                        let target = ctx.break_bb;
                        self.emit(Inst::Jump { target });
                    }
                }
                BlockItem::Continue { label } => {
                    let label_str = label.map(|l| self.resolve(l).to_string());
                    if let Some(ctx) = self.find_loop(label_str.as_deref()) {
                        let target = ctx.continue_bb;
                        self.emit(Inst::Jump { target });
                    }
                }
            }
        }

        let result = match tail {
            Some(idx) => self.lower_expr_idx(idx),
            None => Operand::IntConst(0),
        };

        self.pop_scope();
        result
    }

    fn lower_if(&mut self, cond: ExprIdx, then_br: ExprIdx, else_br: Option<ExprIdx>) -> Operand {
        let cond_op = self.lower_expr_idx(cond);

        let then_bb = self.create_block();
        let else_bb = self.create_block();
        let merge_bb = self.create_block();
        let result = self.fresh_vreg();

        self.emit(Inst::Branch {
            cond: cond_op,
            then_bb,
            else_bb,
        });

        // Then branch
        self.switch_to_block(then_bb);
        let then_val = self.lower_expr_idx(then_br);
        self.emit(Inst::Copy {
            dst: result,
            src: then_val,
        });
        self.emit(Inst::Jump { target: merge_bb });

        // Else branch
        self.switch_to_block(else_bb);
        let else_val = else_br
            .map(|e| self.lower_expr_idx(e))
            .unwrap_or(Operand::IntConst(0));
        self.emit(Inst::Copy {
            dst: result,
            src: else_val,
        });
        self.emit(Inst::Jump { target: merge_bb });

        // Merge
        self.switch_to_block(merge_bb);
        Operand::VReg(result)
    }

    fn lower_loop(&mut self, label: Option<&Ident>, body: ExprIdx) -> Operand {
        let loop_bb = self.create_block();
        let exit_bb = self.create_block();

        let label_str = label.map(|l| self.resolve(*l).to_string());

        // Push loop context
        self.loop_stack.push(LoopContext {
            label: label_str,
            continue_bb: loop_bb,
            break_bb: exit_bb,
        });

        // Jump to loop
        self.emit(Inst::Jump { target: loop_bb });

        // Loop body
        self.switch_to_block(loop_bb);
        self.lower_expr_idx(body);
        self.emit(Inst::Jump { target: loop_bb }); // loop back

        // Pop loop context
        self.loop_stack.pop();

        // Exit block
        self.switch_to_block(exit_bb);
        Operand::IntConst(0)
    }

    fn lower_while(&mut self, label: Option<&Ident>, cond: ExprIdx, body: ExprIdx) -> Operand {
        let cond_bb = self.create_block();
        let loop_bb = self.create_block();
        let exit_bb = self.create_block();

        let label_str = label.map(|l| self.resolve(*l).to_string());

        // Push loop context (continue goes to condition check)
        self.loop_stack.push(LoopContext {
            label: label_str,
            continue_bb: cond_bb,
            break_bb: exit_bb,
        });

        // Jump to condition
        self.emit(Inst::Jump { target: cond_bb });

        // Condition block
        self.switch_to_block(cond_bb);
        let cond_op = self.lower_expr_idx(cond);
        self.emit(Inst::Branch {
            cond: cond_op,
            then_bb: loop_bb,
            else_bb: exit_bb,
        });

        // Loop body
        self.switch_to_block(loop_bb);
        self.lower_expr_idx(body);
        self.emit(Inst::Jump { target: cond_bb }); // back to condition

        // Pop loop context
        self.loop_stack.pop();

        // Exit block
        self.switch_to_block(exit_bb);
        Operand::IntConst(0)
    }

    /// Lower for-loop, desugaring to while-loop.
    /// For range iteration: `for i in start..end { body }` -> counter loop
    /// For list iteration: `for item in list { body }` -> indexed loop
    fn lower_for(
        &mut self,
        binding: Ident,
        iterable: ExprIdx,
        label: Option<&Ident>,
        body: ExprIdx,
    ) -> Operand {
        // Check if iterable is a Range expression
        let iterable_expr = self.hir.expressions[iterable].clone();
        if let Expression::Range { start, end } = iterable_expr {
            return self.lower_for_range(binding, start, end, label, body);
        }

        // List iteration: desugar to indexed while loop
        self.lower_for_list(binding, iterable, label, body)
    }

    /// Lower range-based for: `for i in start..end { body }`
    /// Desugars to:
    /// ```text
    /// i = start
    /// __end = end
    /// while i < __end {
    ///     body
    ///     i = i + 1
    /// }
    /// ```
    fn lower_for_range(
        &mut self,
        binding: Ident,
        start: ExprIdx,
        end: ExprIdx,
        label: Option<&Ident>,
        body: ExprIdx,
    ) -> Operand {
        let cond_bb = self.create_block();
        let loop_bb = self.create_block();
        let incr_bb = self.create_block();
        let exit_bb = self.create_block();

        let label_str = label.map(|l| self.resolve(*l).to_string());

        // binding = start
        let binding_str = self.resolve(binding).to_string();
        let start_val = self.lower_expr_idx(start);
        let binding_local = self.alloc_local(&binding_str);
        self.emit(Inst::StoreLocal {
            local: binding_local,
            src: start_val,
        });

        // __end = end
        let end_val = self.lower_expr_idx(end);
        let end_local = self.alloc_local("__end");
        self.emit(Inst::StoreLocal {
            local: end_local,
            src: end_val,
        });

        // Push loop context (continue -> incr_bb)
        self.loop_stack.push(LoopContext {
            label: label_str,
            continue_bb: incr_bb,
            break_bb: exit_bb,
        });

        // Jump to condition
        self.emit(Inst::Jump { target: cond_bb });

        // cond_bb: binding < __end
        self.switch_to_block(cond_bb);
        let idx = self.fresh_vreg();
        self.emit(Inst::LoadLocal {
            dst: idx,
            local: binding_local,
        });
        let len = self.fresh_vreg();
        self.emit(Inst::LoadLocal {
            dst: len,
            local: end_local,
        });
        let cond = self.fresh_vreg();
        self.emit(Inst::LtInt {
            dst: cond,
            lhs: Operand::VReg(idx),
            rhs: Operand::VReg(len),
        });
        self.emit(Inst::Branch {
            cond: Operand::VReg(cond),
            then_bb: loop_bb,
            else_bb: exit_bb,
        });

        // loop_bb: body
        self.switch_to_block(loop_bb);
        self.lower_expr_idx(body);
        self.emit(Inst::Jump { target: incr_bb });

        // incr_bb: binding = binding + 1
        self.switch_to_block(incr_bb);
        let idx2 = self.fresh_vreg();
        self.emit(Inst::LoadLocal {
            dst: idx2,
            local: binding_local,
        });
        let next_idx = self.fresh_vreg();
        self.emit(Inst::AddInt {
            dst: next_idx,
            lhs: Operand::VReg(idx2),
            rhs: Operand::IntConst(1),
        });
        self.emit(Inst::StoreLocal {
            local: binding_local,
            src: Operand::VReg(next_idx),
        });
        self.emit(Inst::Jump { target: cond_bb });

        // Pop loop context
        self.loop_stack.pop();

        // exit_bb
        self.switch_to_block(exit_bb);
        Operand::IntConst(0)
    }

    /// Lower list-based for: `for item in list { body }`
    /// Desugars to:
    /// ```text
    /// __idx = 0
    /// __list = list
    /// __len = len(__list)
    /// while __idx < __len {
    ///     item = __list[__idx]
    ///     body
    ///     __idx = __idx + 1
    /// }
    /// ```
    fn lower_for_list(
        &mut self,
        binding: Ident,
        iterable: ExprIdx,
        label: Option<&Ident>,
        body: ExprIdx,
    ) -> Operand {
        let cond_bb = self.create_block();
        let loop_bb = self.create_block();
        let incr_bb = self.create_block();
        let exit_bb = self.create_block();

        let label_str = label.map(|l| self.resolve(*l).to_string());

        // __idx = 0
        let idx_local = self.alloc_local("__idx");
        self.emit(Inst::StoreLocal {
            local: idx_local,
            src: Operand::IntConst(0),
        });

        // __list = iterable
        let list_val = self.lower_expr_idx(iterable);
        let list_local = self.alloc_local("__list");
        self.emit(Inst::StoreLocal {
            local: list_local,
            src: list_val,
        });

        // __len = len(__list)
        let len_dst = self.fresh_vreg();
        let list_for_len = self.fresh_vreg();
        self.emit(Inst::LoadLocal {
            dst: list_for_len,
            local: list_local,
        });
        self.emit(Inst::ListLen {
            dst: len_dst,
            list: Operand::VReg(list_for_len),
        });
        let len_local = self.alloc_local("__len");
        self.emit(Inst::StoreLocal {
            local: len_local,
            src: Operand::VReg(len_dst),
        });

        // Allocate binding local
        let binding_str = self.resolve(binding).to_string();
        let binding_local = self.alloc_local(&binding_str);

        // Push loop context (continue -> incr_bb)
        self.loop_stack.push(LoopContext {
            label: label_str,
            continue_bb: incr_bb,
            break_bb: exit_bb,
        });

        // Jump to condition
        self.emit(Inst::Jump { target: cond_bb });

        // cond_bb: __idx < __len
        self.switch_to_block(cond_bb);
        let idx = self.fresh_vreg();
        self.emit(Inst::LoadLocal {
            dst: idx,
            local: idx_local,
        });
        let len = self.fresh_vreg();
        self.emit(Inst::LoadLocal {
            dst: len,
            local: len_local,
        });
        let cond = self.fresh_vreg();
        self.emit(Inst::LtInt {
            dst: cond,
            lhs: Operand::VReg(idx),
            rhs: Operand::VReg(len),
        });
        self.emit(Inst::Branch {
            cond: Operand::VReg(cond),
            then_bb: loop_bb,
            else_bb: exit_bb,
        });

        // loop_bb: binding = __list[__idx]; body
        self.switch_to_block(loop_bb);
        let list_v = self.fresh_vreg();
        self.emit(Inst::LoadLocal {
            dst: list_v,
            local: list_local,
        });
        let idx_v = self.fresh_vreg();
        self.emit(Inst::LoadLocal {
            dst: idx_v,
            local: idx_local,
        });
        let elem = self.fresh_vreg();
        self.emit(Inst::ListGet {
            dst: elem,
            list: Operand::VReg(list_v),
            index: Operand::VReg(idx_v),
        });
        self.emit(Inst::StoreLocal {
            local: binding_local,
            src: Operand::VReg(elem),
        });

        self.lower_expr_idx(body);
        self.emit(Inst::Jump { target: incr_bb });

        // incr_bb: __idx = __idx + 1
        self.switch_to_block(incr_bb);
        let idx2 = self.fresh_vreg();
        self.emit(Inst::LoadLocal {
            dst: idx2,
            local: idx_local,
        });
        let next_idx = self.fresh_vreg();
        self.emit(Inst::AddInt {
            dst: next_idx,
            lhs: Operand::VReg(idx2),
            rhs: Operand::IntConst(1),
        });
        self.emit(Inst::StoreLocal {
            local: idx_local,
            src: Operand::VReg(next_idx),
        });
        self.emit(Inst::Jump { target: cond_bb });

        // Pop loop context
        self.loop_stack.pop();

        // exit_bb
        self.switch_to_block(exit_bb);
        Operand::IntConst(0)
    }

    /// Lower standalone range expression (not in for-loop context).
    /// For now, just returns 0 since ranges are primarily used in for-loops.
    fn lower_range(&mut self, _start: ExprIdx, _end: ExprIdx) -> Operand {
        // Standalone range expressions are not supported outside for loops
        // Could implement as a list [start, start+1, ..., end-1] but that's inefficient
        Operand::IntConst(0)
    }

    fn lower_list(&mut self, elements: &[ExprIdx]) -> Operand {
        let dst = self.fresh_vreg();

        // Create new list with capacity
        self.emit(Inst::ListNew {
            dst,
            capacity: elements.len() as u32,
        });

        // Set each element
        for (i, elem_idx) in elements.iter().enumerate() {
            let value = self.lower_expr_idx(*elem_idx);
            self.emit(Inst::ListSet {
                list: Operand::VReg(dst),
                index: Operand::IntConst(i as i64),
                value,
            });
        }

        Operand::VReg(dst)
    }

    fn lower_index(&mut self, collection: ExprIdx, index: ExprIdx) -> Operand {
        let list = self.lower_expr_idx(collection);
        let idx = self.lower_expr_idx(index);
        let dst = self.fresh_vreg();

        self.emit(Inst::ListGet {
            dst,
            list,
            index: idx,
        });

        Operand::VReg(dst)
    }

    fn lower_slice(
        &mut self,
        collection: ExprIdx,
        start: Option<ExprIdx>,
        end: Option<ExprIdx>,
    ) -> Operand {
        let list = self.lower_expr_idx(collection);
        let start_op = start.map(|s| self.lower_expr_idx(s));
        let end_op = end.map(|e| self.lower_expr_idx(e));
        let dst = self.fresh_vreg();

        self.emit(Inst::ListSlice {
            dst,
            list,
            start: start_op,
            end: end_op,
        });

        Operand::VReg(dst)
    }

    fn lower_tuple(&mut self, elements: &[ExprIdx]) -> Operand {
        let element_ops: Vec<Operand> = elements.iter().map(|e| self.lower_expr_idx(*e)).collect();

        let dst = self.fresh_vreg();
        self.emit(Inst::TupleNew {
            dst,
            elements: element_ops,
        });

        Operand::VReg(dst)
    }

    fn lower_tuple_access(&mut self, tuple: ExprIdx, index: u32) -> Operand {
        let tuple_op = self.lower_expr_idx(tuple);
        let dst = self.fresh_vreg();

        self.emit(Inst::TupleGet {
            dst,
            tuple: tuple_op,
            index,
        });

        Operand::VReg(dst)
    }

    fn finish(self) -> Module {
        // Sort functions by id to ensure main is last
        let mut functions = self.functions;
        functions.sort_by_key(|f| f.id.0);

        let main_id = functions
            .iter()
            .find(|f| f.name.as_deref() == Some("main"))
            .map(|f| f.id)
            .unwrap_or(FuncId(functions.len() as u32 - 1));

        Module { functions, main_id }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ast::AstNode;

    fn hir_from_source(source: &str) -> hir::LowerResult {
        let (syntax, _errors) = parse::parse(source);
        let root = ast::Root::cast(syntax).expect("failed to cast to Root");
        hir::lower(root)
    }

    fn make_types() -> InferenceResult {
        InferenceResult::default()
    }

    #[test]
    fn test_lower_literal() {
        let hir = hir_from_source("x := 42");
        let types = make_types();
        let module = lower(&hir, &types);

        assert_eq!(module.functions.len(), 1);
        assert_eq!(module.main().local_count, 1);
    }

    #[test]
    fn test_lower_function_def() {
        let hir = hir_from_source("fn add(a, b) { a + b }");
        let types = make_types();
        let module = lower(&hir, &types);

        // Should have 2 functions: add and main
        assert_eq!(module.functions.len(), 2);

        // Find the add function
        let add_fn = module
            .functions
            .iter()
            .find(|f| f.name.as_deref() == Some("add"))
            .unwrap();
        assert_eq!(add_fn.param_count, 2);
        assert_eq!(add_fn.params.len(), 2);
    }
}
