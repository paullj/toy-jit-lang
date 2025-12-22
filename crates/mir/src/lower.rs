use std::collections::HashMap;

use hir::{BlockItem, Definition, ExprIdx, Expression, InfixOp, Item, Literal, PrefixOp};
use infer::InferenceResult;

use crate::ir::{Block, BlockId, Function, Inst, LocalId, Module, Operand, VReg};

pub fn lower(hir: &hir::LowerResult, _types: &InferenceResult) -> Module {
    let mut ctx = LowerCtx::new(hir);
    ctx.lower_items();
    ctx.finish()
}

struct LowerCtx<'a> {
    hir: &'a hir::LowerResult,
    func: Function,
    current_block: Block,
    next_vreg: u32,
    next_local: u32,
    next_block: u32,
    /// Stack of scopes, each scope maps names to local IDs
    local_scopes: Vec<HashMap<String, LocalId>>,
}

impl<'a> LowerCtx<'a> {
    fn new(hir: &'a hir::LowerResult) -> Self {
        Self {
            hir,
            func: Function::new(None),
            current_block: Block::new(BlockId(0)),
            next_vreg: 0,
            next_local: 0,
            next_block: 1,                      // 0 is the entry block
            local_scopes: vec![HashMap::new()], // Start with global scope
        }
    }

    fn create_block(&mut self) -> BlockId {
        let id = BlockId(self.next_block);
        self.next_block += 1;
        id
    }

    fn switch_to_block(&mut self, id: BlockId) {
        // Save current block if it has instructions
        if !self.current_block.insts.is_empty() || self.current_block.id.0 != id.0 {
            let old_block = std::mem::replace(&mut self.current_block, Block::new(id));
            self.func.blocks.push(old_block);
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
        // Always allocate a new local in the current scope
        let id = LocalId(self.next_local);
        self.next_local += 1;
        if let Some(scope) = self.local_scopes.last_mut() {
            scope.insert(name.to_string(), id);
        }
        id
    }

    fn get_local(&self, name: &str) -> Option<LocalId> {
        // Search from innermost to outermost scope
        for scope in self.local_scopes.iter().rev() {
            if let Some(&id) = scope.get(name) {
                return Some(id);
            }
        }
        None
    }

    fn emit(&mut self, inst: Inst) {
        self.current_block.push(inst);
    }

    fn lower_items(&mut self) {
        for item in &self.hir.items {
            self.lower_item(item);
        }
        // Add implicit return at end
        self.emit(Inst::Return { value: None });
    }

    fn lower_item(&mut self, item: &Item) {
        match item {
            Item::Definition(Definition::Variable { name, value }) => {
                let operand = self.lower_expr(value);
                let local = self.alloc_local(name);
                self.emit(Inst::StoreLocal {
                    local,
                    src: operand,
                });
            }
            Item::Assignment { name, value } => {
                let operand = self.lower_expr(value);
                if let Some(local) = self.get_local(name) {
                    self.emit(Inst::StoreLocal {
                        local,
                        src: operand,
                    });
                }
                // Note: undefined variable errors already caught by type checker
            }
            Item::Expression(expr) => {
                // Expression statement - evaluate for side effects, discard result
                self.lower_expr(expr);
            }
        }
    }

    fn lower_expr(&mut self, expr: &Expression) -> Operand {
        match expr {
            Expression::Missing => {
                // Should have been caught by type checker
                Operand::IntConst(0)
            }
            Expression::Literal(lit) => self.lower_literal(lit),
            Expression::Infix { op, lhs, rhs } => self.lower_infix(*op, *lhs, *rhs),
            Expression::Prefix { op, expr } => self.lower_prefix(*op, *expr),
            Expression::VariableRef { name } => {
                if let Some(local) = self.get_local(name) {
                    let dst = self.fresh_vreg();
                    self.emit(Inst::LoadLocal { dst, local });
                    Operand::VReg(dst)
                } else {
                    // Should have been caught by type checker
                    Operand::IntConst(0)
                }
            }
            Expression::Block { items, tail } => self.lower_block(items, *tail),
            Expression::If {
                condition,
                then_branch,
                else_branch,
            } => self.lower_if(*condition, *then_branch, *else_branch),
        }
    }

    fn lower_expr_idx(&mut self, idx: ExprIdx) -> Operand {
        let expr = &self.hir.expressions[idx];
        self.lower_expr(expr)
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
        // For And/Or, we should do short-circuit evaluation
        // For now, just evaluate both sides
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

            // And/Or: for now, just compute both and combine
            // TODO: short-circuit with branches
            InfixOp::And => {
                // a and b -> if a then b else false
                // For now: compute both, return rhs if lhs is true
                // This is incorrect but placeholder until we have branches
                Inst::Copy { dst, src: rhs_op }
            }
            InfixOp::Or => {
                // a or b -> if a then true else b
                // For now: placeholder
                Inst::Copy { dst, src: lhs_op }
            }
        };

        self.emit(inst);
        Operand::VReg(dst)
    }

    fn lower_prefix(&mut self, op: PrefixOp, expr: ExprIdx) -> Operand {
        let src = self.lower_expr_idx(expr);
        let dst = self.fresh_vreg();

        // For Neg, we need to determine if it's int or float from types
        // For now, assume int (type info available but not used yet)
        let inst = match op {
            PrefixOp::Neg => Inst::NegInt { dst, src },
            PrefixOp::Not => Inst::Not { dst, src },
        };

        self.emit(inst);
        Operand::VReg(dst)
    }

    fn lower_block(&mut self, items: &[BlockItem], tail: Option<ExprIdx>) -> Operand {
        self.push_scope();

        // Lower all items in the block
        for item in items {
            match item {
                BlockItem::Definition { name, value } => {
                    let operand = self.lower_expr_idx(*value);
                    let local = self.alloc_local(name);
                    self.emit(Inst::StoreLocal {
                        local,
                        src: operand,
                    });
                }
                BlockItem::Assignment { name, value } => {
                    let operand = self.lower_expr_idx(*value);
                    if let Some(local) = self.get_local(name) {
                        self.emit(Inst::StoreLocal {
                            local,
                            src: operand,
                        });
                    }
                }
                BlockItem::Expression(idx) => {
                    // Evaluate for side effects
                    self.lower_expr_idx(*idx);
                }
            }
        }

        // Return the tail expression value, or Unit (represented as 0)
        let result = match tail {
            Some(idx) => self.lower_expr_idx(idx),
            None => Operand::IntConst(0), // Unit
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

        // Emit branch
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
            .unwrap_or(Operand::IntConst(0)); // Unit
        self.emit(Inst::Copy {
            dst: result,
            src: else_val,
        });
        self.emit(Inst::Jump { target: merge_bb });

        // Merge
        self.switch_to_block(merge_bb);
        Operand::VReg(result)
    }

    fn finish(mut self) -> Module {
        self.func.blocks.push(self.current_block);
        self.func.vreg_count = self.next_vreg;
        self.func.local_count = self.next_local;
        Module { main: self.func }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use hir::{Definition, Expression, Item, Literal};
    use la_arena::Arena;

    fn make_hir(items: Vec<Item>) -> hir::LowerResult {
        hir::LowerResult {
            items,
            expressions: Arena::new(),
            expr_spans: Default::default(),
            item_spans: vec![Default::default()],
            symbols: Default::default(),
            diagnostics: Default::default(),
        }
    }

    fn make_types() -> InferenceResult {
        InferenceResult::default()
    }

    #[test]
    fn test_lower_literal() {
        let hir = make_hir(vec![Item::Definition(Definition::Variable {
            name: "x".to_string(),
            value: Expression::Literal(Literal::Integer(42)),
        })]);
        let types = make_types();
        let module = lower(&hir, &types);

        assert_eq!(module.main.local_count, 1);
        assert!(module.main.blocks.len() == 1);
    }
}
