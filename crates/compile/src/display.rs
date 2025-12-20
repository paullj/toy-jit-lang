use std::fmt;

use crate::bytecode::Instruction;
use crate::chunk::{Chunk, CompiledModule};

impl fmt::Display for Instruction {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Instruction::LoadInt { dst, value } => write!(f, "load.i {} {}", dst, value),
            Instruction::LoadBool { dst, value } => write!(f, "load.b {} {}", dst, value),
            Instruction::LoadConst { dst, idx } => write!(f, "load.c {} {}", dst, idx),
            Instruction::Move { dst, src } => write!(f, "move {} {}", dst, src),

            Instruction::AddInt { dst, lhs, rhs } => write!(f, "add.i {} {} {}", dst, lhs, rhs),
            Instruction::SubInt { dst, lhs, rhs } => write!(f, "sub.i {} {} {}", dst, lhs, rhs),
            Instruction::MulInt { dst, lhs, rhs } => write!(f, "mul.i {} {} {}", dst, lhs, rhs),
            Instruction::DivInt { dst, lhs, rhs } => write!(f, "div.i {} {} {}", dst, lhs, rhs),
            Instruction::ModInt { dst, lhs, rhs } => write!(f, "mod.i {} {} {}", dst, lhs, rhs),
            Instruction::NegInt { dst, src } => write!(f, "neg.i {} {}", dst, src),

            Instruction::AddFloat { dst, lhs, rhs } => write!(f, "add.f {} {} {}", dst, lhs, rhs),
            Instruction::SubFloat { dst, lhs, rhs } => write!(f, "sub.f {} {} {}", dst, lhs, rhs),
            Instruction::MulFloat { dst, lhs, rhs } => write!(f, "mul.f {} {} {}", dst, lhs, rhs),
            Instruction::DivFloat { dst, lhs, rhs } => write!(f, "div.f {} {} {}", dst, lhs, rhs),
            Instruction::NegFloat { dst, src } => write!(f, "neg.f {} {}", dst, src),

            Instruction::EqInt { dst, lhs, rhs } => write!(f, "eq.i {} {} {}", dst, lhs, rhs),
            Instruction::NeInt { dst, lhs, rhs } => write!(f, "ne.i {} {} {}", dst, lhs, rhs),
            Instruction::LtInt { dst, lhs, rhs } => write!(f, "lt.i {} {} {}", dst, lhs, rhs),
            Instruction::LeInt { dst, lhs, rhs } => write!(f, "le.i {} {} {}", dst, lhs, rhs),
            Instruction::GtInt { dst, lhs, rhs } => write!(f, "gt.i {} {} {}", dst, lhs, rhs),
            Instruction::GeInt { dst, lhs, rhs } => write!(f, "ge.i {} {} {}", dst, lhs, rhs),

            Instruction::LtFloat { dst, lhs, rhs } => write!(f, "lt.f {} {} {}", dst, lhs, rhs),
            Instruction::LeFloat { dst, lhs, rhs } => write!(f, "le.f {} {} {}", dst, lhs, rhs),
            Instruction::GtFloat { dst, lhs, rhs } => write!(f, "gt.f {} {} {}", dst, lhs, rhs),
            Instruction::GeFloat { dst, lhs, rhs } => write!(f, "ge.f {} {} {}", dst, lhs, rhs),

            Instruction::Not { dst, src } => write!(f, "not {} {}", dst, src),

            Instruction::StoreLocal { slot, src } => write!(f, "store {} {}", slot, src),
            Instruction::LoadLocal { dst, slot } => write!(f, "load {} {}", dst, slot),

            Instruction::Jump { target } => write!(f, "jump {}", target),
            Instruction::JumpIf { cond, target } => write!(f, "jump.if {} {}", cond, target),
            Instruction::JumpIfNot { cond, target } => write!(f, "jump.ifn {} {}", cond, target),

            Instruction::Halt => write!(f, "halt"),
        }
    }
}

impl fmt::Display for Chunk {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(
            f,
            "; locals: {}, registers: {}",
            self.local_count, self.register_count
        )?;

        if !self.constants.is_empty() {
            writeln!(f, "; constants:")?;
            for (idx, c) in self.constants.iter() {
                writeln!(f, ";   {} = {}", idx, c)?;
            }
        }

        writeln!(f)?;
        for (i, inst) in self.instructions.iter().enumerate() {
            writeln!(f, "{:4}: {}", i, inst)?;
        }
        Ok(())
    }
}

impl fmt::Display for CompiledModule {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "=== main ===")?;
        write!(f, "{}", self.main)
    }
}
