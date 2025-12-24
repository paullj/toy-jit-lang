//! Virtual machine state and execution.

use std::cell::RefCell;
use std::rc::Rc;

use compile::{Chunk, CompiledModule, ConstIdx, Instruction, LocalSlot, Reg};

use crate::error::RuntimeError;
use crate::frame::CallFrame;
use crate::value::{ClosureValue, Value};

/// Virtual machine state
pub struct Vm<'a> {
    pub(crate) regs: Vec<Value>,
    pub(crate) locals: Vec<Value>,
    pub(crate) frames: Vec<CallFrame>,
    pub(crate) chunks: &'a [Chunk],
    pub(crate) current_chunk: usize,
    pub(crate) pc: usize,
    pub(crate) last_value: Value,
}

impl<'a> Vm<'a> {
    pub fn new(module: &'a CompiledModule) -> Self {
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

    /// Execute the VM until completion.
    pub fn execute(&mut self) -> Result<Value, RuntimeError> {
        loop {
            let chunk = &self.chunks[self.current_chunk];
            if self.pc >= chunk.instructions.len() {
                break;
            }

            let inst = chunk.instructions[self.pc].clone();
            self.pc += 1;

            let base_reg = self.base_reg();
            let base_local = self.base_local();

            match inst {
                // Loads
                Instruction::LoadInt { dst, value } => {
                    self.set_reg(dst, base_reg, Value::Int(value))?;
                    self.last_value = Value::Int(value);
                }
                Instruction::LoadBool { dst, value } => {
                    self.set_reg(dst, base_reg, Value::Bool(value))?;
                    self.last_value = Value::Bool(value);
                }
                Instruction::LoadConst { dst, idx } => {
                    let val = load_const(&chunk.constants, idx)?;
                    self.last_value = val.clone();
                    self.set_reg(dst, base_reg, val)?;
                }

                // Move
                Instruction::Move { dst, src } => {
                    let val = self.get_reg(src, base_reg)?.clone();
                    self.last_value = val.clone();
                    self.set_reg(dst, base_reg, val)?;
                }

                // Integer arithmetic
                Instruction::AddInt { dst, lhs, rhs } => {
                    let result = self.get_int(lhs, base_reg)? + self.get_int(rhs, base_reg)?;
                    self.set_reg(dst, base_reg, Value::Int(result))?;
                    self.last_value = Value::Int(result);
                }
                Instruction::SubInt { dst, lhs, rhs } => {
                    let result = self.get_int(lhs, base_reg)? - self.get_int(rhs, base_reg)?;
                    self.set_reg(dst, base_reg, Value::Int(result))?;
                    self.last_value = Value::Int(result);
                }
                Instruction::MulInt { dst, lhs, rhs } => {
                    let result = self.get_int(lhs, base_reg)? * self.get_int(rhs, base_reg)?;
                    self.set_reg(dst, base_reg, Value::Int(result))?;
                    self.last_value = Value::Int(result);
                }
                Instruction::DivInt { dst, lhs, rhs } => {
                    let divisor = self.get_int(rhs, base_reg)?;
                    if divisor == 0 {
                        return Err(RuntimeError::DivisionByZero);
                    }
                    let result = self.get_int(lhs, base_reg)? / divisor;
                    self.set_reg(dst, base_reg, Value::Int(result))?;
                    self.last_value = Value::Int(result);
                }
                Instruction::ModInt { dst, lhs, rhs } => {
                    let divisor = self.get_int(rhs, base_reg)?;
                    if divisor == 0 {
                        return Err(RuntimeError::DivisionByZero);
                    }
                    let result = self.get_int(lhs, base_reg)? % divisor;
                    self.set_reg(dst, base_reg, Value::Int(result))?;
                    self.last_value = Value::Int(result);
                }
                Instruction::NegInt { dst, src } => {
                    let result = -self.get_int(src, base_reg)?;
                    self.set_reg(dst, base_reg, Value::Int(result))?;
                    self.last_value = Value::Int(result);
                }

                // Float arithmetic
                Instruction::AddFloat { dst, lhs, rhs } => {
                    let result = self.get_float(lhs, base_reg)? + self.get_float(rhs, base_reg)?;
                    self.set_reg(dst, base_reg, Value::Float(result))?;
                    self.last_value = Value::Float(result);
                }
                Instruction::SubFloat { dst, lhs, rhs } => {
                    let result = self.get_float(lhs, base_reg)? - self.get_float(rhs, base_reg)?;
                    self.set_reg(dst, base_reg, Value::Float(result))?;
                    self.last_value = Value::Float(result);
                }
                Instruction::MulFloat { dst, lhs, rhs } => {
                    let result = self.get_float(lhs, base_reg)? * self.get_float(rhs, base_reg)?;
                    self.set_reg(dst, base_reg, Value::Float(result))?;
                    self.last_value = Value::Float(result);
                }
                Instruction::DivFloat { dst, lhs, rhs } => {
                    let result = self.get_float(lhs, base_reg)? / self.get_float(rhs, base_reg)?;
                    self.set_reg(dst, base_reg, Value::Float(result))?;
                    self.last_value = Value::Float(result);
                }
                Instruction::NegFloat { dst, src } => {
                    let result = -self.get_float(src, base_reg)?;
                    self.set_reg(dst, base_reg, Value::Float(result))?;
                    self.last_value = Value::Float(result);
                }

                // Integer comparisons
                Instruction::EqInt { dst, lhs, rhs } => {
                    let result = self.get_int(lhs, base_reg)? == self.get_int(rhs, base_reg)?;
                    self.set_reg(dst, base_reg, Value::Bool(result))?;
                    self.last_value = Value::Bool(result);
                }
                Instruction::NeInt { dst, lhs, rhs } => {
                    let result = self.get_int(lhs, base_reg)? != self.get_int(rhs, base_reg)?;
                    self.set_reg(dst, base_reg, Value::Bool(result))?;
                    self.last_value = Value::Bool(result);
                }
                Instruction::LtInt { dst, lhs, rhs } => {
                    let result = self.get_int(lhs, base_reg)? < self.get_int(rhs, base_reg)?;
                    self.set_reg(dst, base_reg, Value::Bool(result))?;
                    self.last_value = Value::Bool(result);
                }
                Instruction::LeInt { dst, lhs, rhs } => {
                    let result = self.get_int(lhs, base_reg)? <= self.get_int(rhs, base_reg)?;
                    self.set_reg(dst, base_reg, Value::Bool(result))?;
                    self.last_value = Value::Bool(result);
                }
                Instruction::GtInt { dst, lhs, rhs } => {
                    let result = self.get_int(lhs, base_reg)? > self.get_int(rhs, base_reg)?;
                    self.set_reg(dst, base_reg, Value::Bool(result))?;
                    self.last_value = Value::Bool(result);
                }
                Instruction::GeInt { dst, lhs, rhs } => {
                    let result = self.get_int(lhs, base_reg)? >= self.get_int(rhs, base_reg)?;
                    self.set_reg(dst, base_reg, Value::Bool(result))?;
                    self.last_value = Value::Bool(result);
                }

                // Float comparisons
                Instruction::LtFloat { dst, lhs, rhs } => {
                    let result = self.get_float(lhs, base_reg)? < self.get_float(rhs, base_reg)?;
                    self.set_reg(dst, base_reg, Value::Bool(result))?;
                    self.last_value = Value::Bool(result);
                }
                Instruction::LeFloat { dst, lhs, rhs } => {
                    let result = self.get_float(lhs, base_reg)? <= self.get_float(rhs, base_reg)?;
                    self.set_reg(dst, base_reg, Value::Bool(result))?;
                    self.last_value = Value::Bool(result);
                }
                Instruction::GtFloat { dst, lhs, rhs } => {
                    let result = self.get_float(lhs, base_reg)? > self.get_float(rhs, base_reg)?;
                    self.set_reg(dst, base_reg, Value::Bool(result))?;
                    self.last_value = Value::Bool(result);
                }
                Instruction::GeFloat { dst, lhs, rhs } => {
                    let result = self.get_float(lhs, base_reg)? >= self.get_float(rhs, base_reg)?;
                    self.set_reg(dst, base_reg, Value::Bool(result))?;
                    self.last_value = Value::Bool(result);
                }

                // Boolean
                Instruction::Not { dst, src } => {
                    let result = !self.get_bool(src, base_reg)?;
                    self.set_reg(dst, base_reg, Value::Bool(result))?;
                    self.last_value = Value::Bool(result);
                }

                // Local variables
                Instruction::StoreLocal { slot, src } => {
                    let val = self.get_reg(src, base_reg)?.clone();
                    self.set_local(slot, base_local, val)?;
                }
                Instruction::LoadLocal { dst, slot } => {
                    let val = self.get_local(slot, base_local)?.clone();
                    self.last_value = val.clone();
                    self.set_reg(dst, base_reg, val)?;
                }

                // Control flow
                Instruction::Jump { target } => {
                    self.pc = target.0 as usize;
                }
                Instruction::JumpIf { cond, target } => {
                    if self.get_bool(cond, base_reg)? {
                        self.pc = target.0 as usize;
                    }
                }
                Instruction::JumpIfNot { cond, target } => {
                    if !self.get_bool(cond, base_reg)? {
                        self.pc = target.0 as usize;
                    }
                }

                // Function calls
                Instruction::Call {
                    dst,
                    func_idx,
                    arg_base,
                    arg_count,
                } => {
                    self.do_call(dst, func_idx.0 as usize, arg_base, arg_count, None)?;
                }

                Instruction::CallIndirect {
                    dst,
                    callee,
                    arg_base,
                    arg_count,
                } => {
                    let closure = self.get_closure(callee, base_reg)?;
                    self.do_call(
                        dst,
                        closure.func_idx,
                        arg_base,
                        arg_count,
                        Some(closure.env.clone()),
                    )?;
                }

                Instruction::Return { src } => {
                    let result = match src {
                        Some(r) => self.get_reg(r, base_reg)?.clone(),
                        None => Value::Unit,
                    };

                    if let Some(frame) = self.frames.pop() {
                        // Return to caller
                        self.current_chunk = frame.return_chunk;
                        self.pc = frame.return_pc;
                        self.regs.truncate(frame.base_reg);
                        self.locals.truncate(frame.base_local);

                        // Store result in caller's register
                        if let Some(dst) = frame.result_reg {
                            let caller_base = self.base_reg();
                            self.set_reg(dst, caller_base, result.clone())?;
                        }
                        self.last_value = result;
                    } else {
                        // Return from main
                        self.last_value = result;
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
                        env.push(self.get_reg(r, base_reg)?.clone());
                    }

                    let closure = ClosureValue {
                        func_idx: func_idx.0 as usize,
                        env: Rc::new(RefCell::new(env)),
                    };
                    self.set_reg(dst, base_reg, Value::Closure(closure))?;
                }

                Instruction::LoadCapture { dst, index } => {
                    let env = self.current_closure_env()?;
                    let val = env
                        .borrow()
                        .get(index as usize)
                        .cloned()
                        .ok_or(RuntimeError::CaptureOutOfBounds(index))?;
                    self.set_reg(dst, base_reg, val)?;
                }

                Instruction::StoreCapture { index, src } => {
                    let val = self.get_reg(src, base_reg)?.clone();
                    let env = self.current_closure_env()?;
                    let mut env_mut = env.borrow_mut();
                    if (index as usize) < env_mut.len() {
                        env_mut[index as usize] = val;
                    } else {
                        return Err(RuntimeError::CaptureOutOfBounds(index));
                    }
                }

                // I/O
                Instruction::Echo { src } => {
                    let val = self.get_reg(src, base_reg)?;
                    println!("{}", val);
                }

                // End
                Instruction::Halt => {
                    break;
                }
            }
        }

        Ok(self.last_value.clone())
    }
}

fn load_const(pool: &compile::ConstantPool, idx: ConstIdx) -> Result<Value, RuntimeError> {
    match pool.get(idx) {
        Some(compile::Constant::Float(f)) => Ok(Value::Float(*f)),
        Some(compile::Constant::String(s)) => Ok(Value::String(s.clone())),
        None => Err(RuntimeError::InvalidConstant(idx.0)),
    }
}
