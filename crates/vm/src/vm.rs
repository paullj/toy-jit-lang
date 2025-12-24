//! Virtual machine state and execution.

use std::cell::RefCell;
use std::rc::Rc;

use compile::{Chunk, CompiledModule, ConstIdx, Instruction, Slot};

use crate::error::RuntimeError;
use crate::frame::CallFrame;
use crate::value::{ClosureValue, Value};

/// Pre-allocated stack capacity (slots)
const STACK_CAPACITY: usize = 8192;
/// Pre-allocated frames capacity
const FRAMES_CAPACITY: usize = 256;

/// Virtual machine state
pub struct Vm<'a> {
    /// Unified value stack (locals + temps for all frames)
    stack: Vec<Value>,
    /// Call frame stack
    frames: Vec<CallFrame>,
    chunks: &'a [Chunk],
    current_chunk: usize,
    pc: usize,
    last_value: Value,
}

impl<'a> Vm<'a> {
    pub fn new(module: &'a CompiledModule) -> Self {
        let main = module.main();
        let frame_size = main.local_count as usize + main.register_count as usize;

        let mut stack = Vec::with_capacity(STACK_CAPACITY);
        stack.resize(frame_size, Value::Unit);

        Self {
            stack,
            frames: Vec::with_capacity(FRAMES_CAPACITY),
            chunks: &module.chunks,
            current_chunk: module.main_idx,
            pc: 0,
            last_value: Value::Unit,
        }
    }

    /// Get current frame's stack base (0 for main)
    #[inline]
    fn stack_base(&self) -> usize {
        self.frames.last().map(|f| f.stack_base).unwrap_or(0)
    }

    /// Convert slot index to absolute stack index
    #[inline]
    fn slot_idx(&self, slot: Slot) -> usize {
        self.stack_base() + slot.0 as usize
    }

    /// Get value at slot (unchecked in release)
    #[inline]
    fn get(&self, slot: Slot) -> &Value {
        let idx = self.slot_idx(slot);
        debug_assert!(idx < self.stack.len(), "slot {} out of bounds", slot.0);
        unsafe { self.stack.get_unchecked(idx) }
    }

    /// Set value at slot (unchecked in release)
    #[inline]
    fn set(&mut self, slot: Slot, value: Value) {
        let idx = self.slot_idx(slot);
        debug_assert!(idx < self.stack.len(), "slot {} out of bounds", slot.0);
        unsafe { *self.stack.get_unchecked_mut(idx) = value };
    }

    /// Get int from slot
    #[inline]
    fn get_int(&self, slot: Slot) -> Result<i64, RuntimeError> {
        match self.get(slot) {
            Value::Int(n) => Ok(*n),
            v => Err(RuntimeError::TypeMismatch {
                expected: "Int",
                got: v.type_name(),
            }),
        }
    }

    /// Get float from slot
    #[inline]
    fn get_float(&self, slot: Slot) -> Result<f64, RuntimeError> {
        match self.get(slot) {
            Value::Float(n) => Ok(*n),
            v => Err(RuntimeError::TypeMismatch {
                expected: "Float",
                got: v.type_name(),
            }),
        }
    }

    /// Get bool from slot
    #[inline]
    fn get_bool(&self, slot: Slot) -> Result<bool, RuntimeError> {
        match self.get(slot) {
            Value::Bool(b) => Ok(*b),
            v => Err(RuntimeError::TypeMismatch {
                expected: "Bool",
                got: v.type_name(),
            }),
        }
    }

    /// Get closure from slot
    #[inline]
    fn get_closure(&self, slot: Slot) -> Result<ClosureValue, RuntimeError> {
        match self.get(slot) {
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

    fn do_call(
        &mut self,
        dst: Option<Slot>,
        func_idx: usize,
        arg_base: Slot,
        arg_count: u8,
        closure_env: Option<Rc<RefCell<Vec<Value>>>>,
    ) -> Result<(), RuntimeError> {
        let chunk = self
            .chunks
            .get(func_idx)
            .ok_or(RuntimeError::InvalidFunction(func_idx as u32))?;

        let caller_base = self.stack_base();

        // New frame starts at current stack top
        let new_base = self.stack.len();
        let new_frame_size = chunk.local_count as usize + chunk.register_count as usize;

        // Extend stack for new frame
        self.stack.resize(new_base + new_frame_size, Value::Unit);

        // Copy args to new frame's locals (slots 0..arg_count)
        let arg_start = caller_base + arg_base.0 as usize;
        for i in 0..arg_count as usize {
            self.stack[new_base + i] = self.stack[arg_start + i].clone();
        }

        // Compute result slot relative to caller's base
        let result_slot = dst.map(|s| s.0);

        // Push frame
        self.frames.push(CallFrame {
            return_pc: self.pc,
            return_chunk: self.current_chunk,
            stack_base: new_base,
            result_slot,
            closure_env,
        });

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

            match inst {
                // Loads
                Instruction::LoadInt { dst, value } => {
                    self.set(dst, Value::Int(value));
                    self.last_value = Value::Int(value);
                }
                Instruction::LoadBool { dst, value } => {
                    self.set(dst, Value::Bool(value));
                    self.last_value = Value::Bool(value);
                }
                Instruction::LoadConst { dst, idx } => {
                    let val = load_const(&chunk.constants, idx)?;
                    self.last_value = val.clone();
                    self.set(dst, val);
                }

                // Move
                Instruction::Move { dst, src } => {
                    let val = self.get(src).clone();
                    self.last_value = val.clone();
                    self.set(dst, val);
                }

                // Integer arithmetic
                Instruction::AddInt { dst, lhs, rhs } => {
                    let result = self.get_int(lhs)? + self.get_int(rhs)?;
                    self.set(dst, Value::Int(result));
                    self.last_value = Value::Int(result);
                }
                Instruction::SubInt { dst, lhs, rhs } => {
                    let result = self.get_int(lhs)? - self.get_int(rhs)?;
                    self.set(dst, Value::Int(result));
                    self.last_value = Value::Int(result);
                }
                Instruction::MulInt { dst, lhs, rhs } => {
                    let result = self.get_int(lhs)? * self.get_int(rhs)?;
                    self.set(dst, Value::Int(result));
                    self.last_value = Value::Int(result);
                }
                Instruction::DivInt { dst, lhs, rhs } => {
                    let divisor = self.get_int(rhs)?;
                    if divisor == 0 {
                        return Err(RuntimeError::DivisionByZero);
                    }
                    let result = self.get_int(lhs)? / divisor;
                    self.set(dst, Value::Int(result));
                    self.last_value = Value::Int(result);
                }
                Instruction::ModInt { dst, lhs, rhs } => {
                    let divisor = self.get_int(rhs)?;
                    if divisor == 0 {
                        return Err(RuntimeError::DivisionByZero);
                    }
                    let result = self.get_int(lhs)? % divisor;
                    self.set(dst, Value::Int(result));
                    self.last_value = Value::Int(result);
                }
                Instruction::NegInt { dst, src } => {
                    let result = -self.get_int(src)?;
                    self.set(dst, Value::Int(result));
                    self.last_value = Value::Int(result);
                }

                // Float arithmetic
                Instruction::AddFloat { dst, lhs, rhs } => {
                    let result = self.get_float(lhs)? + self.get_float(rhs)?;
                    self.set(dst, Value::Float(result));
                    self.last_value = Value::Float(result);
                }
                Instruction::SubFloat { dst, lhs, rhs } => {
                    let result = self.get_float(lhs)? - self.get_float(rhs)?;
                    self.set(dst, Value::Float(result));
                    self.last_value = Value::Float(result);
                }
                Instruction::MulFloat { dst, lhs, rhs } => {
                    let result = self.get_float(lhs)? * self.get_float(rhs)?;
                    self.set(dst, Value::Float(result));
                    self.last_value = Value::Float(result);
                }
                Instruction::DivFloat { dst, lhs, rhs } => {
                    let result = self.get_float(lhs)? / self.get_float(rhs)?;
                    self.set(dst, Value::Float(result));
                    self.last_value = Value::Float(result);
                }
                Instruction::NegFloat { dst, src } => {
                    let result = -self.get_float(src)?;
                    self.set(dst, Value::Float(result));
                    self.last_value = Value::Float(result);
                }

                // Integer comparisons
                Instruction::EqInt { dst, lhs, rhs } => {
                    let result = self.get_int(lhs)? == self.get_int(rhs)?;
                    self.set(dst, Value::Bool(result));
                    self.last_value = Value::Bool(result);
                }
                Instruction::NeInt { dst, lhs, rhs } => {
                    let result = self.get_int(lhs)? != self.get_int(rhs)?;
                    self.set(dst, Value::Bool(result));
                    self.last_value = Value::Bool(result);
                }
                Instruction::LtInt { dst, lhs, rhs } => {
                    let result = self.get_int(lhs)? < self.get_int(rhs)?;
                    self.set(dst, Value::Bool(result));
                    self.last_value = Value::Bool(result);
                }
                Instruction::LeInt { dst, lhs, rhs } => {
                    let result = self.get_int(lhs)? <= self.get_int(rhs)?;
                    self.set(dst, Value::Bool(result));
                    self.last_value = Value::Bool(result);
                }
                Instruction::GtInt { dst, lhs, rhs } => {
                    let result = self.get_int(lhs)? > self.get_int(rhs)?;
                    self.set(dst, Value::Bool(result));
                    self.last_value = Value::Bool(result);
                }
                Instruction::GeInt { dst, lhs, rhs } => {
                    let result = self.get_int(lhs)? >= self.get_int(rhs)?;
                    self.set(dst, Value::Bool(result));
                    self.last_value = Value::Bool(result);
                }

                // Float comparisons
                Instruction::LtFloat { dst, lhs, rhs } => {
                    let result = self.get_float(lhs)? < self.get_float(rhs)?;
                    self.set(dst, Value::Bool(result));
                    self.last_value = Value::Bool(result);
                }
                Instruction::LeFloat { dst, lhs, rhs } => {
                    let result = self.get_float(lhs)? <= self.get_float(rhs)?;
                    self.set(dst, Value::Bool(result));
                    self.last_value = Value::Bool(result);
                }
                Instruction::GtFloat { dst, lhs, rhs } => {
                    let result = self.get_float(lhs)? > self.get_float(rhs)?;
                    self.set(dst, Value::Bool(result));
                    self.last_value = Value::Bool(result);
                }
                Instruction::GeFloat { dst, lhs, rhs } => {
                    let result = self.get_float(lhs)? >= self.get_float(rhs)?;
                    self.set(dst, Value::Bool(result));
                    self.last_value = Value::Bool(result);
                }

                // Boolean
                Instruction::Not { dst, src } => {
                    let result = !self.get_bool(src)?;
                    self.set(dst, Value::Bool(result));
                    self.last_value = Value::Bool(result);
                }

                // Control flow
                Instruction::Jump { target } => {
                    self.pc = target.0 as usize;
                }
                Instruction::JumpIf { cond, target } => {
                    if self.get_bool(cond)? {
                        self.pc = target.0 as usize;
                    }
                }
                Instruction::JumpIfNot { cond, target } => {
                    if !self.get_bool(cond)? {
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
                    let closure = self.get_closure(callee)?;
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
                        Some(s) => self.get(s).clone(),
                        None => Value::Unit,
                    };

                    if let Some(frame) = self.frames.pop() {
                        // Return to caller
                        self.current_chunk = frame.return_chunk;
                        self.pc = frame.return_pc;

                        // Get caller's stack base before truncating
                        let caller_base = self.stack_base();

                        // Truncate stack (deallocate this frame)
                        self.stack.truncate(frame.stack_base);

                        // Store result in caller's slot
                        if let Some(slot) = frame.result_slot {
                            let idx = caller_base + slot as usize;
                            self.stack[idx] = result.clone();
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
                    let base = self.stack_base();
                    let mut env = Vec::with_capacity(capture_count as usize);
                    for i in 0..capture_count {
                        let idx = base + capture_base.0 as usize + i as usize;
                        env.push(self.stack[idx].clone());
                    }

                    let closure = ClosureValue {
                        func_idx: func_idx.0 as usize,
                        env: Rc::new(RefCell::new(env)),
                    };
                    self.set(dst, Value::Closure(closure));
                }

                Instruction::LoadCapture { dst, index } => {
                    let env = self.current_closure_env()?;
                    let val = env
                        .borrow()
                        .get(index as usize)
                        .cloned()
                        .ok_or(RuntimeError::CaptureOutOfBounds(index))?;
                    self.set(dst, val);
                }

                Instruction::StoreCapture { index, src } => {
                    let val = self.get(src).clone();
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
                    let val = self.get(src);
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
