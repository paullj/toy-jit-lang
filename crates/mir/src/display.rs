use std::fmt;

use crate::ir::{Block, Function, Inst, Module};

impl fmt::Display for Inst {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Inst::AddInt { dst, lhs, rhs } => write!(f, "{} = add.i {} {}", dst, lhs, rhs),
            Inst::SubInt { dst, lhs, rhs } => write!(f, "{} = sub.i {} {}", dst, lhs, rhs),
            Inst::MulInt { dst, lhs, rhs } => write!(f, "{} = mul.i {} {}", dst, lhs, rhs),
            Inst::DivInt { dst, lhs, rhs } => write!(f, "{} = div.i {} {}", dst, lhs, rhs),
            Inst::ModInt { dst, lhs, rhs } => write!(f, "{} = mod.i {} {}", dst, lhs, rhs),
            Inst::NegInt { dst, src } => write!(f, "{} = neg.i {}", dst, src),

            Inst::AddFloat { dst, lhs, rhs } => write!(f, "{} = add.f {} {}", dst, lhs, rhs),
            Inst::SubFloat { dst, lhs, rhs } => write!(f, "{} = sub.f {} {}", dst, lhs, rhs),
            Inst::MulFloat { dst, lhs, rhs } => write!(f, "{} = mul.f {} {}", dst, lhs, rhs),
            Inst::DivFloat { dst, lhs, rhs } => write!(f, "{} = div.f {} {}", dst, lhs, rhs),
            Inst::NegFloat { dst, src } => write!(f, "{} = neg.f {}", dst, src),

            Inst::EqInt { dst, lhs, rhs } => write!(f, "{} = eq.i {} {}", dst, lhs, rhs),
            Inst::NeInt { dst, lhs, rhs } => write!(f, "{} = ne.i {} {}", dst, lhs, rhs),
            Inst::LtInt { dst, lhs, rhs } => write!(f, "{} = lt.i {} {}", dst, lhs, rhs),
            Inst::LeInt { dst, lhs, rhs } => write!(f, "{} = le.i {} {}", dst, lhs, rhs),
            Inst::GtInt { dst, lhs, rhs } => write!(f, "{} = gt.i {} {}", dst, lhs, rhs),
            Inst::GeInt { dst, lhs, rhs } => write!(f, "{} = ge.i {} {}", dst, lhs, rhs),

            Inst::LtFloat { dst, lhs, rhs } => write!(f, "{} = lt.f {} {}", dst, lhs, rhs),
            Inst::LeFloat { dst, lhs, rhs } => write!(f, "{} = le.f {} {}", dst, lhs, rhs),
            Inst::GtFloat { dst, lhs, rhs } => write!(f, "{} = gt.f {} {}", dst, lhs, rhs),
            Inst::GeFloat { dst, lhs, rhs } => write!(f, "{} = ge.f {} {}", dst, lhs, rhs),

            Inst::Not { dst, src } => write!(f, "{} = not {}", dst, src),
            Inst::Copy { dst, src } => write!(f, "{} = copy {}", dst, src),

            Inst::StoreLocal { local, src } => write!(f, "store {} {}", local, src),
            Inst::LoadLocal { dst, local } => write!(f, "{} = load {}", dst, local),

            Inst::Jump { target } => write!(f, "jump {}", target),
            Inst::Branch {
                cond,
                then_bb,
                else_bb,
            } => {
                write!(f, "branch {} {} {}", cond, then_bb, else_bb)
            }
            Inst::Return { value: Some(v) } => write!(f, "ret {}", v),
            Inst::Return { value: None } => write!(f, "ret"),

            // Function calls
            Inst::Call { dst, func, args } => {
                if let Some(d) = dst {
                    write!(f, "{} = ", d)?;
                }
                write!(f, "call {}", func)?;
                for arg in args {
                    write!(f, " {}", arg)?;
                }
                Ok(())
            }
            Inst::CallIndirect { dst, callee, args } => {
                if let Some(d) = dst {
                    write!(f, "{} = ", d)?;
                }
                write!(f, "call_indirect {}", callee)?;
                for arg in args {
                    write!(f, " {}", arg)?;
                }
                Ok(())
            }

            // Closures
            Inst::MakeClosure {
                dst,
                func,
                captures,
            } => {
                write!(f, "{} = make_closure {}", dst, func)?;
                for cap in captures {
                    write!(f, " {}", cap)?;
                }
                Ok(())
            }
            Inst::LoadCapture { dst, index } => {
                write!(f, "{} = load_capture {}", dst, index)
            }
            Inst::StoreCapture { index, src } => {
                write!(f, "store_capture {} {}", index, src)
            }
            Inst::Echo { src } => {
                write!(f, "echo {}", src)
            }
        }
    }
}

impl fmt::Display for Block {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "{}:", self.id)?;
        for inst in &self.insts {
            writeln!(f, "    {}", inst)?;
        }
        Ok(())
    }
}

impl fmt::Display for Function {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let name = self.name.as_deref().unwrap_or("anon");
        write!(f, "fn {} {} (", self.id, name)?;
        for (i, param) in self.params.iter().enumerate() {
            if i > 0 {
                write!(f, ", ")?;
            }
            write!(f, "{}", param)?;
        }
        writeln!(f, ") {{")?;
        writeln!(
            f,
            "  ; locals: {}, vregs: {}, closure: {}",
            self.local_count, self.vreg_count, self.is_closure
        )?;
        if !self.captures.is_empty() {
            write!(f, "  ; captures:")?;
            for cap in &self.captures {
                write!(f, " {}@{}", cap.name, cap.outer_local)?;
            }
            writeln!(f)?;
        }
        for block in &self.blocks {
            for line in block.to_string().lines() {
                writeln!(f, "  {}", line)?;
            }
        }
        writeln!(f, "}}")
    }
}

impl fmt::Display for Module {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "; main: {}", self.main_id)?;
        for func in &self.functions {
            write!(f, "{}", func)?;
        }
        Ok(())
    }
}
