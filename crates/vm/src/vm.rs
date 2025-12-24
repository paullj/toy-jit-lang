//! Virtual machine state and execution.

use compile::{Chunk, CompiledModule, ConstIdx, Instruction, Slot};

use crate::error::RuntimeError;
use crate::frame::CallFrame;
use crate::value::{Heap, Value};

/// Pre-allocated stack capacity (slots)
const STACK_CAPACITY: usize = 8192;
/// Pre-allocated frames capacity
const FRAMES_CAPACITY: usize = 256;

/// Virtual machine state
pub struct Vm<'a> {
    /// Unified value stack (locals + temps for all frames)
    stack: Vec<Value>,
    /// Heap for strings and closures
    heap: Heap,
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
        stack.resize(frame_size, Value::unit());

        Self {
            stack,
            heap: Heap::new(),
            frames: Vec::with_capacity(FRAMES_CAPACITY),
            chunks: &module.chunks,
            current_chunk: module.main_idx,
            pc: 0,
            last_value: Value::unit(),
        }
    }

    /// Get current frame's stack base (0 for main)
    #[inline(always)]
    fn stack_base(&self) -> usize {
        self.frames.last().map(|f| f.stack_base).unwrap_or(0)
    }

    /// Convert slot index to absolute stack index
    #[inline(always)]
    fn slot_idx(&self, base: usize, slot: Slot) -> usize {
        base + slot.0 as usize
    }

    /// Get value at slot (unchecked in release)
    #[inline(always)]
    fn get(&self, base: usize, slot: Slot) -> Value {
        let idx = self.slot_idx(base, slot);
        debug_assert!(idx < self.stack.len(), "slot {} out of bounds", slot.0);
        unsafe { *self.stack.get_unchecked(idx) }
    }

    /// Set value at slot (unchecked in release)
    #[inline(always)]
    fn set(&mut self, base: usize, slot: Slot, value: Value) {
        let idx = self.slot_idx(base, slot);
        debug_assert!(idx < self.stack.len(), "slot {} out of bounds", slot.0);
        unsafe { *self.stack.get_unchecked_mut(idx) = value };
    }

    /// Get int from slot
    #[inline(always)]
    fn get_int(&self, base: usize, slot: Slot) -> Result<i64, RuntimeError> {
        let val = self.get(base, slot);
        val.as_int().ok_or(RuntimeError::TypeMismatch {
            expected: "Int",
            got: val.type_name(),
        })
    }

    /// Get float from slot
    #[inline(always)]
    fn get_float(&self, base: usize, slot: Slot) -> Result<f64, RuntimeError> {
        let val = self.get(base, slot);
        val.as_float().ok_or(RuntimeError::TypeMismatch {
            expected: "Float",
            got: val.type_name(),
        })
    }

    /// Get bool from slot
    #[inline(always)]
    fn get_bool(&self, base: usize, slot: Slot) -> Result<bool, RuntimeError> {
        let val = self.get(base, slot);
        val.as_bool().ok_or(RuntimeError::TypeMismatch {
            expected: "Bool",
            got: val.type_name(),
        })
    }

    /// Get closure index from slot
    #[inline(always)]
    fn get_closure_idx(&self, base: usize, slot: Slot) -> Result<u32, RuntimeError> {
        let val = self.get(base, slot);
        val.as_closure_idx().ok_or(RuntimeError::NotAClosure)
    }

    /// Get current frame's closure index
    fn current_closure_idx(&self) -> Result<u32, RuntimeError> {
        self.frames
            .last()
            .and_then(|f| f.closure_idx)
            .ok_or(RuntimeError::NoClosure)
    }

    fn do_call(
        &mut self,
        dst: Option<Slot>,
        func_idx: usize,
        arg_base: Slot,
        arg_count: u8,
        closure_idx: Option<u32>,
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
        self.stack.resize(new_base + new_frame_size, Value::unit());

        // Copy args to new frame's locals (slots 0..arg_count)
        let arg_start = caller_base + arg_base.0 as usize;
        for i in 0..arg_count as usize {
            self.stack[new_base + i] = self.stack[arg_start + i];
        }

        // Compute result slot relative to caller's base
        let result_slot = dst.map(|s| s.0);

        // Push frame
        self.frames.push(CallFrame {
            return_pc: self.pc,
            return_chunk: self.current_chunk,
            stack_base: new_base,
            result_slot,
            closure_idx,
        });

        self.current_chunk = func_idx;
        self.pc = 0;

        Ok(())
    }

    /// Execute the VM until completion. Returns (result, heap) to allow displaying heap values.
    pub fn execute(mut self) -> Result<(Value, Heap), RuntimeError> {
        loop {
            let chunk = &self.chunks[self.current_chunk];
            if self.pc >= chunk.instructions.len() {
                break;
            }

            let inst = &chunk.instructions[self.pc];
            self.pc += 1;

            // Cache stack base for this instruction
            let base = self.stack_base();

            match *inst {
                // Loads
                Instruction::LoadInt { dst, value } => {
                    let val = Value::int(value);
                    self.set(base, dst, val);
                    self.last_value = val;
                }
                Instruction::LoadBool { dst, value } => {
                    let val = Value::bool(value);
                    self.set(base, dst, val);
                    self.last_value = val;
                }
                Instruction::LoadConst { dst, idx } => {
                    let val = self.load_const(&chunk.constants, idx)?;
                    self.last_value = val;
                    self.set(base, dst, val);
                }

                // Move
                Instruction::Move { dst, src } => {
                    let val = self.get(base, src);
                    self.last_value = val;
                    self.set(base, dst, val);
                }

                // Integer arithmetic
                Instruction::AddInt { dst, lhs, rhs } => {
                    let result = Value::int(self.get_int(base, lhs)? + self.get_int(base, rhs)?);
                    self.set(base, dst, result);
                    self.last_value = result;
                }
                Instruction::SubInt { dst, lhs, rhs } => {
                    let result = Value::int(self.get_int(base, lhs)? - self.get_int(base, rhs)?);
                    self.set(base, dst, result);
                    self.last_value = result;
                }
                Instruction::MulInt { dst, lhs, rhs } => {
                    let result = Value::int(self.get_int(base, lhs)? * self.get_int(base, rhs)?);
                    self.set(base, dst, result);
                    self.last_value = result;
                }
                Instruction::DivInt { dst, lhs, rhs } => {
                    let divisor = self.get_int(base, rhs)?;
                    if divisor == 0 {
                        return Err(RuntimeError::DivisionByZero);
                    }
                    let result = Value::int(self.get_int(base, lhs)? / divisor);
                    self.set(base, dst, result);
                    self.last_value = result;
                }
                Instruction::ModInt { dst, lhs, rhs } => {
                    let divisor = self.get_int(base, rhs)?;
                    if divisor == 0 {
                        return Err(RuntimeError::DivisionByZero);
                    }
                    let result = Value::int(self.get_int(base, lhs)? % divisor);
                    self.set(base, dst, result);
                    self.last_value = result;
                }
                Instruction::NegInt { dst, src } => {
                    let result = Value::int(-self.get_int(base, src)?);
                    self.set(base, dst, result);
                    self.last_value = result;
                }

                // Float arithmetic
                Instruction::AddFloat { dst, lhs, rhs } => {
                    let result =
                        Value::float(self.get_float(base, lhs)? + self.get_float(base, rhs)?);
                    self.set(base, dst, result);
                    self.last_value = result;
                }
                Instruction::SubFloat { dst, lhs, rhs } => {
                    let result =
                        Value::float(self.get_float(base, lhs)? - self.get_float(base, rhs)?);
                    self.set(base, dst, result);
                    self.last_value = result;
                }
                Instruction::MulFloat { dst, lhs, rhs } => {
                    let result =
                        Value::float(self.get_float(base, lhs)? * self.get_float(base, rhs)?);
                    self.set(base, dst, result);
                    self.last_value = result;
                }
                Instruction::DivFloat { dst, lhs, rhs } => {
                    let result =
                        Value::float(self.get_float(base, lhs)? / self.get_float(base, rhs)?);
                    self.set(base, dst, result);
                    self.last_value = result;
                }
                Instruction::NegFloat { dst, src } => {
                    let result = Value::float(-self.get_float(base, src)?);
                    self.set(base, dst, result);
                    self.last_value = result;
                }

                // Integer comparisons
                Instruction::EqInt { dst, lhs, rhs } => {
                    let result = Value::bool(self.get_int(base, lhs)? == self.get_int(base, rhs)?);
                    self.set(base, dst, result);
                    self.last_value = result;
                }
                Instruction::NeInt { dst, lhs, rhs } => {
                    let result = Value::bool(self.get_int(base, lhs)? != self.get_int(base, rhs)?);
                    self.set(base, dst, result);
                    self.last_value = result;
                }
                Instruction::LtInt { dst, lhs, rhs } => {
                    let result = Value::bool(self.get_int(base, lhs)? < self.get_int(base, rhs)?);
                    self.set(base, dst, result);
                    self.last_value = result;
                }
                Instruction::LeInt { dst, lhs, rhs } => {
                    let result = Value::bool(self.get_int(base, lhs)? <= self.get_int(base, rhs)?);
                    self.set(base, dst, result);
                    self.last_value = result;
                }
                Instruction::GtInt { dst, lhs, rhs } => {
                    let result = Value::bool(self.get_int(base, lhs)? > self.get_int(base, rhs)?);
                    self.set(base, dst, result);
                    self.last_value = result;
                }
                Instruction::GeInt { dst, lhs, rhs } => {
                    let result = Value::bool(self.get_int(base, lhs)? >= self.get_int(base, rhs)?);
                    self.set(base, dst, result);
                    self.last_value = result;
                }

                // Float comparisons
                Instruction::LtFloat { dst, lhs, rhs } => {
                    let result =
                        Value::bool(self.get_float(base, lhs)? < self.get_float(base, rhs)?);
                    self.set(base, dst, result);
                    self.last_value = result;
                }
                Instruction::LeFloat { dst, lhs, rhs } => {
                    let result =
                        Value::bool(self.get_float(base, lhs)? <= self.get_float(base, rhs)?);
                    self.set(base, dst, result);
                    self.last_value = result;
                }
                Instruction::GtFloat { dst, lhs, rhs } => {
                    let result =
                        Value::bool(self.get_float(base, lhs)? > self.get_float(base, rhs)?);
                    self.set(base, dst, result);
                    self.last_value = result;
                }
                Instruction::GeFloat { dst, lhs, rhs } => {
                    let result =
                        Value::bool(self.get_float(base, lhs)? >= self.get_float(base, rhs)?);
                    self.set(base, dst, result);
                    self.last_value = result;
                }

                // Boolean
                Instruction::Not { dst, src } => {
                    let result = Value::bool(!self.get_bool(base, src)?);
                    self.set(base, dst, result);
                    self.last_value = result;
                }

                // Control flow
                Instruction::Jump { target } => {
                    self.pc = target.0 as usize;
                }
                Instruction::JumpIf { cond, target } => {
                    if self.get_bool(base, cond)? {
                        self.pc = target.0 as usize;
                    }
                }
                Instruction::JumpIfNot { cond, target } => {
                    if !self.get_bool(base, cond)? {
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
                    let closure_idx = self.get_closure_idx(base, callee)?;
                    let func_idx = self.heap.get_closure(closure_idx).func_idx as usize;
                    self.do_call(dst, func_idx, arg_base, arg_count, Some(closure_idx))?;
                }

                Instruction::Return { src } => {
                    let result = match src {
                        Some(s) => self.get(base, s),
                        None => Value::unit(),
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
                            self.stack[idx] = result;
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
                    let mut captures = Vec::with_capacity(capture_count as usize);
                    for i in 0..capture_count {
                        let idx = base + capture_base.0 as usize + i as usize;
                        captures.push(self.stack[idx]);
                    }

                    let closure_idx = self.heap.alloc_closure(func_idx.0, captures);
                    self.set(base, dst, Value::closure(closure_idx));
                }

                Instruction::LoadCapture { dst, index } => {
                    let closure_idx = self.current_closure_idx()?;
                    let closure = self.heap.get_closure(closure_idx);
                    let val = closure
                        .captures
                        .get(index as usize)
                        .map(|c| c.get())
                        .ok_or(RuntimeError::CaptureOutOfBounds(index))?;
                    self.set(base, dst, val);
                }

                Instruction::StoreCapture { index, src } => {
                    let val = self.get(base, src);
                    let closure_idx = self.current_closure_idx()?;
                    let closure = self.heap.get_closure(closure_idx);
                    if (index as usize) < closure.captures.len() {
                        closure.captures[index as usize].set(val);
                    } else {
                        return Err(RuntimeError::CaptureOutOfBounds(index));
                    }
                }

                // I/O
                Instruction::Echo { src } => {
                    let val = self.get(base, src);
                    println!("{}", val.display(&self.heap));
                }

                // End
                Instruction::Halt => {
                    break;
                }
            }
        }

        Ok((self.last_value, self.heap))
    }

    fn load_const(
        &mut self,
        pool: &compile::ConstantPool,
        idx: ConstIdx,
    ) -> Result<Value, RuntimeError> {
        match pool.get(idx) {
            Some(compile::Constant::Float(f)) => Ok(Value::float(*f)),
            Some(compile::Constant::String(s)) => {
                let str_idx = self.heap.alloc_string(s.clone());
                Ok(Value::string(str_idx))
            }
            None => Err(RuntimeError::InvalidConstant(idx.0)),
        }
    }
}
