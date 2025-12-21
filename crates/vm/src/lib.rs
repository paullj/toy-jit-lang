//! Bytecode interpreter for the toy language.

use compile::{CompiledModule, ConstIdx, Instruction, LocalSlot, Reg};

/// Runtime value
#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    Int(i64),
    Float(f64),
    Bool(bool),
    String(String),
    Unit,
}

impl std::fmt::Display for Value {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Value::Int(n) => write!(f, "{}", n),
            Value::Float(n) => write!(f, "{}", n),
            Value::Bool(b) => write!(f, "{}", b),
            Value::String(s) => write!(f, "{}", s),
            Value::Unit => write!(f, "()"),
        }
    }
}

/// Runtime error
#[derive(Debug, Clone, PartialEq)]
pub enum RuntimeError {
    DivisionByZero,
    InvalidRegister(u32),
    InvalidLocal(u32),
    InvalidConstant(u32),
    TypeMismatch {
        expected: &'static str,
        got: &'static str,
    },
}

impl std::fmt::Display for RuntimeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RuntimeError::DivisionByZero => write!(f, "division by zero"),
            RuntimeError::InvalidRegister(r) => write!(f, "invalid register r{}", r),
            RuntimeError::InvalidLocal(l) => write!(f, "invalid local slot{}", l),
            RuntimeError::InvalidConstant(c) => write!(f, "invalid constant c{}", c),
            RuntimeError::TypeMismatch { expected, got } => {
                write!(f, "type mismatch: expected {}, got {}", expected, got)
            }
        }
    }
}

impl std::error::Error for RuntimeError {}

/// Virtual machine state
pub struct Vm {
    regs: Vec<Value>,
    locals: Vec<Value>,
    last_value: Value,
}

impl Vm {
    fn new(register_count: u32, local_count: u32) -> Self {
        Self {
            regs: vec![Value::Unit; register_count as usize],
            locals: vec![Value::Unit; local_count as usize],
            last_value: Value::Unit,
        }
    }

    fn get_reg(&self, r: Reg) -> Result<&Value, RuntimeError> {
        self.regs
            .get(r.0 as usize)
            .ok_or(RuntimeError::InvalidRegister(r.0))
    }

    fn set_reg(&mut self, r: Reg, v: Value) -> Result<(), RuntimeError> {
        if (r.0 as usize) < self.regs.len() {
            self.regs[r.0 as usize] = v;
            Ok(())
        } else {
            Err(RuntimeError::InvalidRegister(r.0))
        }
    }

    fn get_local(&self, slot: LocalSlot) -> Result<&Value, RuntimeError> {
        self.locals
            .get(slot.0 as usize)
            .ok_or(RuntimeError::InvalidLocal(slot.0))
    }

    fn set_local(&mut self, slot: LocalSlot, v: Value) -> Result<(), RuntimeError> {
        if (slot.0 as usize) < self.locals.len() {
            self.locals[slot.0 as usize] = v;
            Ok(())
        } else {
            Err(RuntimeError::InvalidLocal(slot.0))
        }
    }

    fn get_int(&self, r: Reg) -> Result<i64, RuntimeError> {
        match self.get_reg(r)? {
            Value::Int(n) => Ok(*n),
            v => Err(RuntimeError::TypeMismatch {
                expected: "Int",
                got: v.type_name(),
            }),
        }
    }

    fn get_float(&self, r: Reg) -> Result<f64, RuntimeError> {
        match self.get_reg(r)? {
            Value::Float(n) => Ok(*n),
            v => Err(RuntimeError::TypeMismatch {
                expected: "Float",
                got: v.type_name(),
            }),
        }
    }

    fn get_bool(&self, r: Reg) -> Result<bool, RuntimeError> {
        match self.get_reg(r)? {
            Value::Bool(b) => Ok(*b),
            v => Err(RuntimeError::TypeMismatch {
                expected: "Bool",
                got: v.type_name(),
            }),
        }
    }
}

impl Value {
    fn type_name(&self) -> &'static str {
        match self {
            Value::Int(_) => "Int",
            Value::Float(_) => "Float",
            Value::Bool(_) => "Bool",
            Value::String(_) => "String",
            Value::Unit => "Unit",
        }
    }
}

/// Execute a compiled module and return the result.
pub fn run(module: &CompiledModule) -> Result<Value, RuntimeError> {
    let chunk = &module.main;
    let mut vm = Vm::new(chunk.register_count, chunk.local_count);
    let mut pc = 0;

    while pc < chunk.instructions.len() {
        let inst = &chunk.instructions[pc];
        pc += 1;

        match inst {
            // Loads
            Instruction::LoadInt { dst, value } => {
                vm.set_reg(*dst, Value::Int(*value))?;
                vm.last_value = Value::Int(*value);
            }
            Instruction::LoadBool { dst, value } => {
                vm.set_reg(*dst, Value::Bool(*value))?;
                vm.last_value = Value::Bool(*value);
            }
            Instruction::LoadConst { dst, idx } => {
                let val = load_const(&chunk.constants, *idx)?;
                vm.last_value = val.clone();
                vm.set_reg(*dst, val)?;
            }

            // Move
            Instruction::Move { dst, src } => {
                let val = vm.get_reg(*src)?.clone();
                vm.last_value = val.clone();
                vm.set_reg(*dst, val)?;
            }

            // Integer arithmetic
            Instruction::AddInt { dst, lhs, rhs } => {
                let result = vm.get_int(*lhs)? + vm.get_int(*rhs)?;
                vm.set_reg(*dst, Value::Int(result))?;
                vm.last_value = Value::Int(result);
            }
            Instruction::SubInt { dst, lhs, rhs } => {
                let result = vm.get_int(*lhs)? - vm.get_int(*rhs)?;
                vm.set_reg(*dst, Value::Int(result))?;
                vm.last_value = Value::Int(result);
            }
            Instruction::MulInt { dst, lhs, rhs } => {
                let result = vm.get_int(*lhs)? * vm.get_int(*rhs)?;
                vm.set_reg(*dst, Value::Int(result))?;
                vm.last_value = Value::Int(result);
            }
            Instruction::DivInt { dst, lhs, rhs } => {
                let divisor = vm.get_int(*rhs)?;
                if divisor == 0 {
                    return Err(RuntimeError::DivisionByZero);
                }
                let result = vm.get_int(*lhs)? / divisor;
                vm.set_reg(*dst, Value::Int(result))?;
                vm.last_value = Value::Int(result);
            }
            Instruction::ModInt { dst, lhs, rhs } => {
                let divisor = vm.get_int(*rhs)?;
                if divisor == 0 {
                    return Err(RuntimeError::DivisionByZero);
                }
                let result = vm.get_int(*lhs)? % divisor;
                vm.set_reg(*dst, Value::Int(result))?;
                vm.last_value = Value::Int(result);
            }
            Instruction::NegInt { dst, src } => {
                let result = -vm.get_int(*src)?;
                vm.set_reg(*dst, Value::Int(result))?;
                vm.last_value = Value::Int(result);
            }

            // Float arithmetic
            Instruction::AddFloat { dst, lhs, rhs } => {
                let result = vm.get_float(*lhs)? + vm.get_float(*rhs)?;
                vm.set_reg(*dst, Value::Float(result))?;
                vm.last_value = Value::Float(result);
            }
            Instruction::SubFloat { dst, lhs, rhs } => {
                let result = vm.get_float(*lhs)? - vm.get_float(*rhs)?;
                vm.set_reg(*dst, Value::Float(result))?;
                vm.last_value = Value::Float(result);
            }
            Instruction::MulFloat { dst, lhs, rhs } => {
                let result = vm.get_float(*lhs)? * vm.get_float(*rhs)?;
                vm.set_reg(*dst, Value::Float(result))?;
                vm.last_value = Value::Float(result);
            }
            Instruction::DivFloat { dst, lhs, rhs } => {
                let result = vm.get_float(*lhs)? / vm.get_float(*rhs)?;
                vm.set_reg(*dst, Value::Float(result))?;
                vm.last_value = Value::Float(result);
            }
            Instruction::NegFloat { dst, src } => {
                let result = -vm.get_float(*src)?;
                vm.set_reg(*dst, Value::Float(result))?;
                vm.last_value = Value::Float(result);
            }

            // Integer comparisons
            Instruction::EqInt { dst, lhs, rhs } => {
                let result = vm.get_int(*lhs)? == vm.get_int(*rhs)?;
                vm.set_reg(*dst, Value::Bool(result))?;
                vm.last_value = Value::Bool(result);
            }
            Instruction::NeInt { dst, lhs, rhs } => {
                let result = vm.get_int(*lhs)? != vm.get_int(*rhs)?;
                vm.set_reg(*dst, Value::Bool(result))?;
                vm.last_value = Value::Bool(result);
            }
            Instruction::LtInt { dst, lhs, rhs } => {
                let result = vm.get_int(*lhs)? < vm.get_int(*rhs)?;
                vm.set_reg(*dst, Value::Bool(result))?;
                vm.last_value = Value::Bool(result);
            }
            Instruction::LeInt { dst, lhs, rhs } => {
                let result = vm.get_int(*lhs)? <= vm.get_int(*rhs)?;
                vm.set_reg(*dst, Value::Bool(result))?;
                vm.last_value = Value::Bool(result);
            }
            Instruction::GtInt { dst, lhs, rhs } => {
                let result = vm.get_int(*lhs)? > vm.get_int(*rhs)?;
                vm.set_reg(*dst, Value::Bool(result))?;
                vm.last_value = Value::Bool(result);
            }
            Instruction::GeInt { dst, lhs, rhs } => {
                let result = vm.get_int(*lhs)? >= vm.get_int(*rhs)?;
                vm.set_reg(*dst, Value::Bool(result))?;
                vm.last_value = Value::Bool(result);
            }

            // Float comparisons
            Instruction::LtFloat { dst, lhs, rhs } => {
                let result = vm.get_float(*lhs)? < vm.get_float(*rhs)?;
                vm.set_reg(*dst, Value::Bool(result))?;
                vm.last_value = Value::Bool(result);
            }
            Instruction::LeFloat { dst, lhs, rhs } => {
                let result = vm.get_float(*lhs)? <= vm.get_float(*rhs)?;
                vm.set_reg(*dst, Value::Bool(result))?;
                vm.last_value = Value::Bool(result);
            }
            Instruction::GtFloat { dst, lhs, rhs } => {
                let result = vm.get_float(*lhs)? > vm.get_float(*rhs)?;
                vm.set_reg(*dst, Value::Bool(result))?;
                vm.last_value = Value::Bool(result);
            }
            Instruction::GeFloat { dst, lhs, rhs } => {
                let result = vm.get_float(*lhs)? >= vm.get_float(*rhs)?;
                vm.set_reg(*dst, Value::Bool(result))?;
                vm.last_value = Value::Bool(result);
            }

            // Boolean
            Instruction::Not { dst, src } => {
                let result = !vm.get_bool(*src)?;
                vm.set_reg(*dst, Value::Bool(result))?;
                vm.last_value = Value::Bool(result);
            }

            // Local variables
            Instruction::StoreLocal { slot, src } => {
                let val = vm.get_reg(*src)?.clone();
                vm.set_local(*slot, val)?;
            }
            Instruction::LoadLocal { dst, slot } => {
                let val = vm.get_local(*slot)?.clone();
                vm.last_value = val.clone();
                vm.set_reg(*dst, val)?;
            }

            // Control flow
            Instruction::Jump { target } => {
                pc = target.0 as usize;
            }
            Instruction::JumpIf { cond, target } => {
                if vm.get_bool(*cond)? {
                    pc = target.0 as usize;
                }
            }
            Instruction::JumpIfNot { cond, target } => {
                if !vm.get_bool(*cond)? {
                    pc = target.0 as usize;
                }
            }

            // End
            Instruction::Halt => {
                break;
            }
        }
    }

    Ok(vm.last_value)
}

fn load_const(pool: &compile::ConstantPool, idx: ConstIdx) -> Result<Value, RuntimeError> {
    match pool.get(idx) {
        Some(compile::Constant::Float(f)) => Ok(Value::Float(*f)),
        Some(compile::Constant::String(s)) => Ok(Value::String(s.clone())),
        None => Err(RuntimeError::InvalidConstant(idx.0)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use compile::compile;
    use mir::{Block, BlockId, Function, Inst, Module, Operand, VReg};

    fn make_module(insts: Vec<Inst>) -> Module {
        let mut func = Function::new(None);
        let mut block = Block::new(BlockId(0));
        for inst in insts {
            block.push(inst);
        }
        block.push(Inst::Return { value: None });
        func.blocks.push(block);
        func.vreg_count = 10;
        func.local_count = 10;
        Module { main: func }
    }

    #[test]
    fn test_add_int() {
        let mir = make_module(vec![Inst::AddInt {
            dst: VReg(0),
            lhs: Operand::IntConst(2),
            rhs: Operand::IntConst(3),
        }]);
        let compiled = compile(&mir);
        let result = run(&compiled).unwrap();
        assert_eq!(result, Value::Int(5));
    }

    #[test]
    fn test_local_vars() {
        use mir::LocalId;
        let mir = make_module(vec![
            Inst::StoreLocal {
                local: LocalId(0),
                src: Operand::IntConst(42),
            },
            Inst::LoadLocal {
                dst: VReg(0),
                local: LocalId(0),
            },
        ]);
        let compiled = compile(&mir);
        let result = run(&compiled).unwrap();
        assert_eq!(result, Value::Int(42));
    }

    #[test]
    fn test_division_by_zero() {
        let mir = make_module(vec![Inst::DivInt {
            dst: VReg(0),
            lhs: Operand::IntConst(10),
            rhs: Operand::IntConst(0),
        }]);
        let compiled = compile(&mir);
        let result = run(&compiled);
        assert_eq!(result, Err(RuntimeError::DivisionByZero));
    }
}
