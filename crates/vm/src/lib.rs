//! Bytecode interpreter for the toy language.

use std::cell::RefCell;
use std::rc::Rc;

use compile::{Chunk, CompiledModule, ConstIdx, Instruction, LocalSlot, Reg};

/// Closure: function + captured environment
#[derive(Debug, Clone)]
pub struct ClosureValue {
    pub func_idx: usize,
    pub env: Rc<RefCell<Vec<Value>>>,
}

impl PartialEq for ClosureValue {
    fn eq(&self, other: &Self) -> bool {
        self.func_idx == other.func_idx && Rc::ptr_eq(&self.env, &other.env)
    }
}

/// Runtime value
#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    Int(i64),
    Float(f64),
    Bool(bool),
    String(String),
    Unit,
    Closure(ClosureValue),
}

impl std::fmt::Display for Value {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Value::Int(n) => write!(f, "{}", n),
            Value::Float(n) => write!(f, "{}", n),
            Value::Bool(b) => write!(f, "{}", b),
            Value::String(s) => write!(f, "{}", s),
            Value::Unit => write!(f, "()"),
            Value::Closure(c) => write!(f, "<closure fn{}>", c.func_idx),
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
    InvalidFunction(u32),
    TypeMismatch {
        expected: &'static str,
        got: &'static str,
    },
    NotAClosure,
    NoClosure,
    CaptureOutOfBounds(u8),
}

impl std::fmt::Display for RuntimeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RuntimeError::DivisionByZero => write!(f, "division by zero"),
            RuntimeError::InvalidRegister(r) => write!(f, "invalid register r{}", r),
            RuntimeError::InvalidLocal(l) => write!(f, "invalid local slot{}", l),
            RuntimeError::InvalidConstant(c) => write!(f, "invalid constant c{}", c),
            RuntimeError::InvalidFunction(idx) => write!(f, "invalid function fn{}", idx),
            RuntimeError::TypeMismatch { expected, got } => {
                write!(f, "type mismatch: expected {}, got {}", expected, got)
            }
            RuntimeError::NotAClosure => write!(f, "expected closure value"),
            RuntimeError::NoClosure => write!(f, "no closure environment in current frame"),
            RuntimeError::CaptureOutOfBounds(idx) => {
                write!(f, "capture index {} out of bounds", idx)
            }
        }
    }
}

impl std::error::Error for RuntimeError {}

/// Call frame for function invocation
#[derive(Debug, Clone)]
struct CallFrame {
    return_pc: usize,
    return_chunk: usize,
    base_reg: usize,
    base_local: usize,
    result_reg: Option<Reg>,
    closure_env: Option<Rc<RefCell<Vec<Value>>>>,
}

/// Virtual machine state
pub struct Vm<'a> {
    regs: Vec<Value>,
    locals: Vec<Value>,
    frames: Vec<CallFrame>,
    chunks: &'a [Chunk],
    current_chunk: usize,
    pc: usize,
    last_value: Value,
}

impl<'a> Vm<'a> {
    fn new(module: &'a CompiledModule) -> Self {
        let main = module.main();
        Self {
            regs: vec![Value::Unit; main.register_count as usize],
            locals: vec![Value::Unit; main.local_count as usize],
            frames: vec![],
            chunks: &module.chunks,
            current_chunk: module.main_idx,
            pc: 0,
            last_value: Value::Unit,
        }
    }

    fn get_reg(&self, r: Reg, base: usize) -> Result<&Value, RuntimeError> {
        let idx = base + r.0 as usize;
        self.regs.get(idx).ok_or(RuntimeError::InvalidRegister(r.0))
    }

    fn set_reg(&mut self, r: Reg, base: usize, v: Value) -> Result<(), RuntimeError> {
        let idx = base + r.0 as usize;
        if idx < self.regs.len() {
            self.regs[idx] = v;
            Ok(())
        } else {
            Err(RuntimeError::InvalidRegister(r.0))
        }
    }

    fn get_local(&self, slot: LocalSlot, base: usize) -> Result<&Value, RuntimeError> {
        let idx = base + slot.0 as usize;
        self.locals
            .get(idx)
            .ok_or(RuntimeError::InvalidLocal(slot.0))
    }

    fn set_local(&mut self, slot: LocalSlot, base: usize, v: Value) -> Result<(), RuntimeError> {
        let idx = base + slot.0 as usize;
        if idx < self.locals.len() {
            self.locals[idx] = v;
            Ok(())
        } else {
            Err(RuntimeError::InvalidLocal(slot.0))
        }
    }

    fn get_int(&self, r: Reg, base: usize) -> Result<i64, RuntimeError> {
        match self.get_reg(r, base)? {
            Value::Int(n) => Ok(*n),
            v => Err(RuntimeError::TypeMismatch {
                expected: "Int",
                got: v.type_name(),
            }),
        }
    }

    fn get_float(&self, r: Reg, base: usize) -> Result<f64, RuntimeError> {
        match self.get_reg(r, base)? {
            Value::Float(n) => Ok(*n),
            v => Err(RuntimeError::TypeMismatch {
                expected: "Float",
                got: v.type_name(),
            }),
        }
    }

    fn get_bool(&self, r: Reg, base: usize) -> Result<bool, RuntimeError> {
        match self.get_reg(r, base)? {
            Value::Bool(b) => Ok(*b),
            v => Err(RuntimeError::TypeMismatch {
                expected: "Bool",
                got: v.type_name(),
            }),
        }
    }

    fn get_closure(&self, r: Reg, base: usize) -> Result<ClosureValue, RuntimeError> {
        match self.get_reg(r, base)? {
            Value::Closure(c) => Ok(c.clone()),
            _ => Err(RuntimeError::NotAClosure),
        }
    }

    fn current_closure_env(&self) -> Result<&Rc<RefCell<Vec<Value>>>, RuntimeError> {
        self.frames
            .last()
            .and_then(|f| f.closure_env.as_ref())
            .ok_or(RuntimeError::NoClosure)
    }

    fn base_reg(&self) -> usize {
        self.frames.last().map(|f| f.base_reg).unwrap_or(0)
    }

    fn base_local(&self) -> usize {
        self.frames.last().map(|f| f.base_local).unwrap_or(0)
    }

    fn do_call(
        &mut self,
        dst: Option<Reg>,
        func_idx: usize,
        arg_base: Reg,
        arg_count: u8,
        closure_env: Option<Rc<RefCell<Vec<Value>>>>,
    ) -> Result<(), RuntimeError> {
        let chunk = self
            .chunks
            .get(func_idx)
            .ok_or(RuntimeError::InvalidFunction(func_idx as u32))?;

        let cur_base_reg = self.base_reg();
        let _cur_base_local = self.base_local();

        // Collect args first before modifying state
        let mut args = Vec::with_capacity(arg_count as usize);
        for i in 0..arg_count {
            let arg_reg = Reg(arg_base.0 + i as u32);
            args.push(self.get_reg(arg_reg, cur_base_reg)?.clone());
        }

        // Save frame
        self.frames.push(CallFrame {
            return_pc: self.pc,
            return_chunk: self.current_chunk,
            base_reg: self.regs.len(),
            base_local: self.locals.len(),
            result_reg: dst,
            closure_env,
        });

        // Allocate new registers/locals
        let new_reg_count = chunk.register_count as usize;
        let new_local_count = chunk.local_count as usize;

        self.regs
            .resize(self.regs.len() + new_reg_count, Value::Unit);
        self.locals
            .resize(self.locals.len() + new_local_count, Value::Unit);

        // Copy args to callee's first locals (params)
        let new_base_local = self.locals.len() - new_local_count;
        for (i, arg) in args.into_iter().enumerate() {
            self.locals[new_base_local + i] = arg;
        }

        // Jump to callee
        self.current_chunk = func_idx;
        self.pc = 0;

        Ok(())
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
            Value::Closure(_) => "Closure",
        }
    }
}

/// Execute a compiled module and return the result.
pub fn run(module: &CompiledModule) -> Result<Value, RuntimeError> {
    let mut vm = Vm::new(module);

    loop {
        let chunk = &vm.chunks[vm.current_chunk];
        if vm.pc >= chunk.instructions.len() {
            break;
        }

        let inst = chunk.instructions[vm.pc].clone();
        vm.pc += 1;

        let base_reg = vm.base_reg();
        let base_local = vm.base_local();

        match inst {
            // Loads
            Instruction::LoadInt { dst, value } => {
                vm.set_reg(dst, base_reg, Value::Int(value))?;
                vm.last_value = Value::Int(value);
            }
            Instruction::LoadBool { dst, value } => {
                vm.set_reg(dst, base_reg, Value::Bool(value))?;
                vm.last_value = Value::Bool(value);
            }
            Instruction::LoadConst { dst, idx } => {
                let val = load_const(&chunk.constants, idx)?;
                vm.last_value = val.clone();
                vm.set_reg(dst, base_reg, val)?;
            }

            // Move
            Instruction::Move { dst, src } => {
                let val = vm.get_reg(src, base_reg)?.clone();
                vm.last_value = val.clone();
                vm.set_reg(dst, base_reg, val)?;
            }

            // Integer arithmetic
            Instruction::AddInt { dst, lhs, rhs } => {
                let result = vm.get_int(lhs, base_reg)? + vm.get_int(rhs, base_reg)?;
                vm.set_reg(dst, base_reg, Value::Int(result))?;
                vm.last_value = Value::Int(result);
            }
            Instruction::SubInt { dst, lhs, rhs } => {
                let result = vm.get_int(lhs, base_reg)? - vm.get_int(rhs, base_reg)?;
                vm.set_reg(dst, base_reg, Value::Int(result))?;
                vm.last_value = Value::Int(result);
            }
            Instruction::MulInt { dst, lhs, rhs } => {
                let result = vm.get_int(lhs, base_reg)? * vm.get_int(rhs, base_reg)?;
                vm.set_reg(dst, base_reg, Value::Int(result))?;
                vm.last_value = Value::Int(result);
            }
            Instruction::DivInt { dst, lhs, rhs } => {
                let divisor = vm.get_int(rhs, base_reg)?;
                if divisor == 0 {
                    return Err(RuntimeError::DivisionByZero);
                }
                let result = vm.get_int(lhs, base_reg)? / divisor;
                vm.set_reg(dst, base_reg, Value::Int(result))?;
                vm.last_value = Value::Int(result);
            }
            Instruction::ModInt { dst, lhs, rhs } => {
                let divisor = vm.get_int(rhs, base_reg)?;
                if divisor == 0 {
                    return Err(RuntimeError::DivisionByZero);
                }
                let result = vm.get_int(lhs, base_reg)? % divisor;
                vm.set_reg(dst, base_reg, Value::Int(result))?;
                vm.last_value = Value::Int(result);
            }
            Instruction::NegInt { dst, src } => {
                let result = -vm.get_int(src, base_reg)?;
                vm.set_reg(dst, base_reg, Value::Int(result))?;
                vm.last_value = Value::Int(result);
            }

            // Float arithmetic
            Instruction::AddFloat { dst, lhs, rhs } => {
                let result = vm.get_float(lhs, base_reg)? + vm.get_float(rhs, base_reg)?;
                vm.set_reg(dst, base_reg, Value::Float(result))?;
                vm.last_value = Value::Float(result);
            }
            Instruction::SubFloat { dst, lhs, rhs } => {
                let result = vm.get_float(lhs, base_reg)? - vm.get_float(rhs, base_reg)?;
                vm.set_reg(dst, base_reg, Value::Float(result))?;
                vm.last_value = Value::Float(result);
            }
            Instruction::MulFloat { dst, lhs, rhs } => {
                let result = vm.get_float(lhs, base_reg)? * vm.get_float(rhs, base_reg)?;
                vm.set_reg(dst, base_reg, Value::Float(result))?;
                vm.last_value = Value::Float(result);
            }
            Instruction::DivFloat { dst, lhs, rhs } => {
                let result = vm.get_float(lhs, base_reg)? / vm.get_float(rhs, base_reg)?;
                vm.set_reg(dst, base_reg, Value::Float(result))?;
                vm.last_value = Value::Float(result);
            }
            Instruction::NegFloat { dst, src } => {
                let result = -vm.get_float(src, base_reg)?;
                vm.set_reg(dst, base_reg, Value::Float(result))?;
                vm.last_value = Value::Float(result);
            }

            // Integer comparisons
            Instruction::EqInt { dst, lhs, rhs } => {
                let result = vm.get_int(lhs, base_reg)? == vm.get_int(rhs, base_reg)?;
                vm.set_reg(dst, base_reg, Value::Bool(result))?;
                vm.last_value = Value::Bool(result);
            }
            Instruction::NeInt { dst, lhs, rhs } => {
                let result = vm.get_int(lhs, base_reg)? != vm.get_int(rhs, base_reg)?;
                vm.set_reg(dst, base_reg, Value::Bool(result))?;
                vm.last_value = Value::Bool(result);
            }
            Instruction::LtInt { dst, lhs, rhs } => {
                let result = vm.get_int(lhs, base_reg)? < vm.get_int(rhs, base_reg)?;
                vm.set_reg(dst, base_reg, Value::Bool(result))?;
                vm.last_value = Value::Bool(result);
            }
            Instruction::LeInt { dst, lhs, rhs } => {
                let result = vm.get_int(lhs, base_reg)? <= vm.get_int(rhs, base_reg)?;
                vm.set_reg(dst, base_reg, Value::Bool(result))?;
                vm.last_value = Value::Bool(result);
            }
            Instruction::GtInt { dst, lhs, rhs } => {
                let result = vm.get_int(lhs, base_reg)? > vm.get_int(rhs, base_reg)?;
                vm.set_reg(dst, base_reg, Value::Bool(result))?;
                vm.last_value = Value::Bool(result);
            }
            Instruction::GeInt { dst, lhs, rhs } => {
                let result = vm.get_int(lhs, base_reg)? >= vm.get_int(rhs, base_reg)?;
                vm.set_reg(dst, base_reg, Value::Bool(result))?;
                vm.last_value = Value::Bool(result);
            }

            // Float comparisons
            Instruction::LtFloat { dst, lhs, rhs } => {
                let result = vm.get_float(lhs, base_reg)? < vm.get_float(rhs, base_reg)?;
                vm.set_reg(dst, base_reg, Value::Bool(result))?;
                vm.last_value = Value::Bool(result);
            }
            Instruction::LeFloat { dst, lhs, rhs } => {
                let result = vm.get_float(lhs, base_reg)? <= vm.get_float(rhs, base_reg)?;
                vm.set_reg(dst, base_reg, Value::Bool(result))?;
                vm.last_value = Value::Bool(result);
            }
            Instruction::GtFloat { dst, lhs, rhs } => {
                let result = vm.get_float(lhs, base_reg)? > vm.get_float(rhs, base_reg)?;
                vm.set_reg(dst, base_reg, Value::Bool(result))?;
                vm.last_value = Value::Bool(result);
            }
            Instruction::GeFloat { dst, lhs, rhs } => {
                let result = vm.get_float(lhs, base_reg)? >= vm.get_float(rhs, base_reg)?;
                vm.set_reg(dst, base_reg, Value::Bool(result))?;
                vm.last_value = Value::Bool(result);
            }

            // Boolean
            Instruction::Not { dst, src } => {
                let result = !vm.get_bool(src, base_reg)?;
                vm.set_reg(dst, base_reg, Value::Bool(result))?;
                vm.last_value = Value::Bool(result);
            }

            // Local variables
            Instruction::StoreLocal { slot, src } => {
                let val = vm.get_reg(src, base_reg)?.clone();
                vm.set_local(slot, base_local, val)?;
            }
            Instruction::LoadLocal { dst, slot } => {
                let val = vm.get_local(slot, base_local)?.clone();
                vm.last_value = val.clone();
                vm.set_reg(dst, base_reg, val)?;
            }

            // Control flow
            Instruction::Jump { target } => {
                vm.pc = target.0 as usize;
            }
            Instruction::JumpIf { cond, target } => {
                if vm.get_bool(cond, base_reg)? {
                    vm.pc = target.0 as usize;
                }
            }
            Instruction::JumpIfNot { cond, target } => {
                if !vm.get_bool(cond, base_reg)? {
                    vm.pc = target.0 as usize;
                }
            }

            // Function calls
            Instruction::Call {
                dst,
                func_idx,
                arg_base,
                arg_count,
            } => {
                vm.do_call(dst, func_idx.0 as usize, arg_base, arg_count, None)?;
            }

            Instruction::CallIndirect {
                dst,
                callee,
                arg_base,
                arg_count,
            } => {
                let closure = vm.get_closure(callee, base_reg)?;
                vm.do_call(
                    dst,
                    closure.func_idx,
                    arg_base,
                    arg_count,
                    Some(closure.env.clone()),
                )?;
            }

            Instruction::Return { src } => {
                let result = match src {
                    Some(r) => vm.get_reg(r, base_reg)?.clone(),
                    None => Value::Unit,
                };

                if let Some(frame) = vm.frames.pop() {
                    // Return to caller
                    vm.current_chunk = frame.return_chunk;
                    vm.pc = frame.return_pc;
                    vm.regs.truncate(frame.base_reg);
                    vm.locals.truncate(frame.base_local);

                    // Store result in caller's register
                    if let Some(dst) = frame.result_reg {
                        let caller_base = vm.base_reg();
                        vm.set_reg(dst, caller_base, result.clone())?;
                    }
                    vm.last_value = result;
                } else {
                    // Return from main
                    vm.last_value = result;
                    break;
                }
            }

            // Closures
            Instruction::MakeClosure {
                dst,
                func_idx,
                capture_base,
                capture_count,
            } => {
                let mut env = Vec::with_capacity(capture_count as usize);
                for i in 0..capture_count {
                    let r = Reg(capture_base.0 + i as u32);
                    env.push(vm.get_reg(r, base_reg)?.clone());
                }

                let closure = ClosureValue {
                    func_idx: func_idx.0 as usize,
                    env: Rc::new(RefCell::new(env)),
                };
                vm.set_reg(dst, base_reg, Value::Closure(closure))?;
            }

            Instruction::LoadCapture { dst, index } => {
                let env = vm.current_closure_env()?;
                let val = env
                    .borrow()
                    .get(index as usize)
                    .cloned()
                    .ok_or(RuntimeError::CaptureOutOfBounds(index))?;
                vm.set_reg(dst, base_reg, val)?;
            }

            Instruction::StoreCapture { index, src } => {
                let val = vm.get_reg(src, base_reg)?.clone();
                let env = vm.current_closure_env()?;
                let mut env_mut = env.borrow_mut();
                if (index as usize) < env_mut.len() {
                    env_mut[index as usize] = val;
                } else {
                    return Err(RuntimeError::CaptureOutOfBounds(index));
                }
            }

            // I/O
            Instruction::Echo { src } => {
                let val = vm.get_reg(src, base_reg)?;
                println!("{}", val);
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
    use mir::{Block, BlockId, FuncId, Function, Inst, Module, Operand, VReg};

    fn make_module(insts: Vec<Inst>, return_vreg: Option<VReg>) -> Module {
        let mut func = Function::new(FuncId(0), Some("main".to_string()));
        let mut block = Block::new(BlockId(0));
        for inst in insts {
            block.push(inst);
        }
        block.push(Inst::Return {
            value: return_vreg.map(Operand::VReg),
        });
        func.blocks.push(block);
        func.vreg_count = 10;
        func.local_count = 10;
        Module {
            functions: vec![func],
            main_id: FuncId(0),
        }
    }

    #[test]
    fn test_add_int() {
        let mir = make_module(
            vec![Inst::AddInt {
                dst: VReg(0),
                lhs: Operand::IntConst(2),
                rhs: Operand::IntConst(3),
            }],
            Some(VReg(0)),
        );
        let compiled = compile(&mir);
        let result = run(&compiled).unwrap();
        assert_eq!(result, Value::Int(5));
    }

    #[test]
    fn test_local_vars() {
        use mir::LocalId;
        let mir = make_module(
            vec![
                Inst::StoreLocal {
                    local: LocalId(0),
                    src: Operand::IntConst(42),
                },
                Inst::LoadLocal {
                    dst: VReg(0),
                    local: LocalId(0),
                },
            ],
            Some(VReg(0)),
        );
        let compiled = compile(&mir);
        let result = run(&compiled).unwrap();
        assert_eq!(result, Value::Int(42));
    }

    #[test]
    fn test_division_by_zero() {
        let mir = make_module(
            vec![Inst::DivInt {
                dst: VReg(0),
                lhs: Operand::IntConst(10),
                rhs: Operand::IntConst(0),
            }],
            Some(VReg(0)),
        );
        let compiled = compile(&mir);
        let result = run(&compiled);
        assert_eq!(result, Err(RuntimeError::DivisionByZero));
    }

    #[test]
    fn test_function_call() {
        // fn add(a, b) { a + b }
        // main: add(1, 2) -> 3
        use mir::LocalId;

        // Function: add(a, b) = a + b
        let mut add_func = Function::new(FuncId(0), Some("add".to_string()));
        add_func.param_count = 2;
        add_func.local_count = 2;
        let mut add_block = Block::new(BlockId(0));
        add_block.push(Inst::LoadLocal {
            dst: VReg(0),
            local: LocalId(0),
        });
        add_block.push(Inst::LoadLocal {
            dst: VReg(1),
            local: LocalId(1),
        });
        add_block.push(Inst::AddInt {
            dst: VReg(2),
            lhs: Operand::VReg(VReg(0)),
            rhs: Operand::VReg(VReg(1)),
        });
        add_block.push(Inst::Return {
            value: Some(Operand::VReg(VReg(2))),
        });
        add_func.blocks.push(add_block);
        add_func.vreg_count = 3;

        // Main function: call add(1, 2)
        let mut main_func = Function::new(FuncId(1), Some("main".to_string()));
        let mut main_block = Block::new(BlockId(0));
        main_block.push(Inst::Call {
            dst: Some(VReg(0)),
            func: FuncId(0),
            args: vec![Operand::IntConst(1), Operand::IntConst(2)],
        });
        main_block.push(Inst::Return {
            value: Some(Operand::VReg(VReg(0))),
        });
        main_func.blocks.push(main_block);
        main_func.vreg_count = 5;
        main_func.local_count = 0;

        let module = Module {
            functions: vec![add_func, main_func],
            main_id: FuncId(1),
        };

        let compiled = compile(&module);
        let result = run(&compiled).unwrap();
        assert_eq!(result, Value::Int(3));
    }

    #[test]
    fn test_closure() {
        // fn make_adder(x) { fn(y) { x + y } }
        // main: make_adder(5)(10) -> 15
        use mir::LocalId;

        // Inner closure function: fn(y) { x + y }
        // x is capture[0], y is param[0]
        let mut inner_func = Function::new(FuncId(0), Some("inner".to_string()));
        inner_func.param_count = 1;
        inner_func.local_count = 1;
        inner_func.is_closure = true;
        let mut inner_block = Block::new(BlockId(0));
        inner_block.push(Inst::LoadCapture {
            dst: VReg(0),
            index: 0, // x
        });
        inner_block.push(Inst::LoadLocal {
            dst: VReg(1),
            local: LocalId(0), // y
        });
        inner_block.push(Inst::AddInt {
            dst: VReg(2),
            lhs: Operand::VReg(VReg(0)),
            rhs: Operand::VReg(VReg(1)),
        });
        inner_block.push(Inst::Return {
            value: Some(Operand::VReg(VReg(2))),
        });
        inner_func.blocks.push(inner_block);
        inner_func.vreg_count = 3;

        // make_adder function: fn(x) { closure(inner, [x]) }
        let mut make_adder = Function::new(FuncId(1), Some("make_adder".to_string()));
        make_adder.param_count = 1;
        make_adder.local_count = 1;
        let mut make_adder_block = Block::new(BlockId(0));
        make_adder_block.push(Inst::LoadLocal {
            dst: VReg(0),
            local: LocalId(0), // x
        });
        make_adder_block.push(Inst::MakeClosure {
            dst: VReg(1),
            func: FuncId(0),
            captures: vec![Operand::VReg(VReg(0))],
        });
        make_adder_block.push(Inst::Return {
            value: Some(Operand::VReg(VReg(1))),
        });
        make_adder.blocks.push(make_adder_block);
        make_adder.vreg_count = 5;

        // Main: make_adder(5)(10)
        let mut main_func = Function::new(FuncId(2), Some("main".to_string()));
        let mut main_block = Block::new(BlockId(0));
        main_block.push(Inst::Call {
            dst: Some(VReg(0)),
            func: FuncId(1),
            args: vec![Operand::IntConst(5)],
        });
        main_block.push(Inst::CallIndirect {
            dst: Some(VReg(1)),
            callee: Operand::VReg(VReg(0)),
            args: vec![Operand::IntConst(10)],
        });
        main_block.push(Inst::Return {
            value: Some(Operand::VReg(VReg(1))),
        });
        main_func.blocks.push(main_block);
        main_func.vreg_count = 5;
        main_func.local_count = 0;

        let module = Module {
            functions: vec![inner_func, make_adder, main_func],
            main_id: FuncId(2),
        };

        let compiled = compile(&module);
        let result = run(&compiled).unwrap();
        assert_eq!(result, Value::Int(15));
    }
}
