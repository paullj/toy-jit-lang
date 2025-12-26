//! Virtual machine state and execution.

use compile::{Chunk, CompiledModule, ConstIdx, NO_SLOT, Opcode, Rodeo};

use crate::bytecode_reader::BytecodeReader;
use crate::error::RuntimeError;
use crate::frame::CallFrame;
use crate::value::{Heap, Value};

/// Maximum stack size (slots) - 8K slots = 128KB with 16-byte Values
const MAX_STACK_SIZE: usize = 8192;
/// Maximum call depth
const MAX_FRAMES: usize = 256;

/// Virtual machine state
pub struct Vm<'a> {
    /// Fixed-size value stack (locals + temps for all frames)
    stack: Box<[Value; MAX_STACK_SIZE]>,
    /// Current stack top (next free slot)
    stack_top: usize,
    /// Heap for dynamic strings and closures
    heap: Heap,
    /// Interned string table (from compiled module)
    strings: &'a Rodeo,
    /// Fixed-size call frame stack
    frames: [CallFrame; MAX_FRAMES],
    /// Number of active frames
    frame_count: usize,
    chunks: &'a [Chunk],
    current_chunk: usize,
    /// Cached stack base (updated on call/return)
    stack_base: usize,
}

impl<'a> Vm<'a> {
    pub fn new(module: &'a CompiledModule) -> Self {
        let main = module.main();
        let frame_size = main.local_count as usize + main.register_count as usize;

        Self {
            stack: Box::new([Value::unit(); MAX_STACK_SIZE]),
            stack_top: frame_size,
            heap: Heap::new(),
            strings: &module.strings,
            frames: [CallFrame::default(); MAX_FRAMES],
            frame_count: 0, // main doesn't push a frame initially
            chunks: &module.chunks,
            current_chunk: module.main_idx,
            stack_base: 0, // main frame base is 0
        }
    }

    /// Convert slot index to absolute stack index
    #[inline(always)]
    fn slot_idx(&self, base: usize, slot: u8) -> usize {
        base + slot as usize
    }

    /// Get value at slot (unchecked in release)
    #[inline(always)]
    fn get(&self, base: usize, slot: u8) -> Value {
        let idx = self.slot_idx(base, slot);
        debug_assert!(idx < self.stack_top, "slot {} out of bounds", slot);
        unsafe { *self.stack.get_unchecked(idx) }
    }

    /// Set value at slot (unchecked in release)
    #[inline(always)]
    fn set(&mut self, base: usize, slot: u8, value: Value) {
        let idx = self.slot_idx(base, slot);
        debug_assert!(idx < self.stack_top, "slot {} out of bounds", slot);
        unsafe { *self.stack.get_unchecked_mut(idx) = value };
    }

    /// Get int from slot (unchecked in release)
    #[inline(always)]
    fn get_int(&self, base: usize, slot: u8) -> i64 {
        self.get(base, slot).as_int_unchecked()
    }

    /// Get float from slot (unchecked in release)
    #[inline(always)]
    fn get_float(&self, base: usize, slot: u8) -> f64 {
        self.get(base, slot).as_float_unchecked()
    }

    /// Get bool from slot (unchecked in release)
    #[inline(always)]
    fn get_bool(&self, base: usize, slot: u8) -> bool {
        self.get(base, slot).as_bool_unchecked()
    }

    /// Get closure index from slot (unchecked in release)
    #[inline(always)]
    fn get_closure_idx(&self, base: usize, slot: u8) -> u32 {
        self.get(base, slot).as_closure_idx_unchecked()
    }

    /// Get current frame's closure index
    fn current_closure_idx(&self) -> Result<u32, RuntimeError> {
        if self.frame_count > 0 {
            self.frames[self.frame_count - 1]
                .closure_idx
                .ok_or(RuntimeError::NoClosure)
        } else {
            Err(RuntimeError::NoClosure)
        }
    }

    fn do_call(
        &mut self,
        reader: &mut BytecodeReader,
        dst: Option<u8>,
        func_idx: usize,
        arg_base: u8,
        arg_count: u8,
        closure_idx: Option<u32>,
    ) -> Result<(), RuntimeError> {
        let chunk = self
            .chunks
            .get(func_idx)
            .ok_or(RuntimeError::InvalidFunction(func_idx as u32))?;

        let caller_base = self.stack_base; // Use cached value

        // New frame starts at current stack top
        let new_base = self.stack_top;
        let new_frame_size = chunk.local_count as usize + chunk.register_count as usize;
        let new_top = new_base + new_frame_size;

        // Check for stack overflow
        if new_top > MAX_STACK_SIZE {
            return Err(RuntimeError::StackOverflow);
        }

        // Initialize new frame slots to unit (args will be overwritten)
        for i in new_base..new_top {
            self.stack[i] = Value::unit();
        }

        // Copy args to new frame's locals (slots 0..arg_count)
        let arg_start = caller_base + arg_base as usize;
        for i in 0..arg_count as usize {
            self.stack[new_base + i] = self.stack[arg_start + i];
        }

        self.stack_top = new_top;

        // Check for call stack overflow
        if self.frame_count >= MAX_FRAMES {
            return Err(RuntimeError::StackOverflow);
        }

        // Push frame
        self.frames[self.frame_count] = CallFrame {
            return_pc: reader.pc(),
            return_chunk: self.current_chunk,
            stack_base: new_base,
            result_slot: dst,
            closure_idx,
        };
        self.frame_count += 1;

        self.stack_base = new_base; // Update cache
        self.current_chunk = func_idx;
        reader.set_pc(0);

        Ok(())
    }

    /// Execute the VM until completion. Returns (result, heap).
    /// If the result is an interned string, it's resolved to a dynamic string for portability.
    pub fn execute(mut self) -> Result<(Value, Heap), RuntimeError> {
        let mut reader = BytecodeReader::new(&self.chunks[self.current_chunk].code);

        loop {
            // Use cached stack base for this instruction
            let base = self.stack_base;
            let opcode = reader.read_opcode();

            match opcode {
                // Loads
                Opcode::LoadInt => {
                    let dst = reader.read_u8();
                    let value = reader.read_i64();
                    self.set(base, dst, Value::int(value));
                }
                Opcode::LoadBool => {
                    let dst = reader.read_u8();
                    let value = reader.read_u8() != 0;
                    self.set(base, dst, Value::bool(value));
                }
                Opcode::LoadConst => {
                    let dst = reader.read_u8();
                    let idx = reader.read_u16();
                    let chunk = &self.chunks[self.current_chunk];
                    let val = self.load_const(&chunk.constants, ConstIdx(idx as u32))?;
                    self.set(base, dst, val);
                }

                // Move
                Opcode::Move => {
                    let dst = reader.read_u8();
                    let src = reader.read_u8();
                    let val = self.get(base, src);
                    self.set(base, dst, val);
                }

                // Integer arithmetic
                Opcode::AddInt => {
                    let dst = reader.read_u8();
                    let lhs = reader.read_u8();
                    let rhs = reader.read_u8();
                    let result = Value::int(self.get_int(base, lhs) + self.get_int(base, rhs));
                    self.set(base, dst, result);
                }
                Opcode::SubInt => {
                    let dst = reader.read_u8();
                    let lhs = reader.read_u8();
                    let rhs = reader.read_u8();
                    let result = Value::int(self.get_int(base, lhs) - self.get_int(base, rhs));
                    self.set(base, dst, result);
                }
                Opcode::MulInt => {
                    let dst = reader.read_u8();
                    let lhs = reader.read_u8();
                    let rhs = reader.read_u8();
                    let result = Value::int(self.get_int(base, lhs) * self.get_int(base, rhs));
                    self.set(base, dst, result);
                }
                Opcode::DivInt => {
                    let dst = reader.read_u8();
                    let lhs = reader.read_u8();
                    let rhs = reader.read_u8();
                    let divisor = self.get_int(base, rhs);
                    if divisor == 0 {
                        return Err(RuntimeError::DivisionByZero);
                    }
                    let result = Value::int(self.get_int(base, lhs) / divisor);
                    self.set(base, dst, result);
                }
                Opcode::ModInt => {
                    let dst = reader.read_u8();
                    let lhs = reader.read_u8();
                    let rhs = reader.read_u8();
                    let divisor = self.get_int(base, rhs);
                    if divisor == 0 {
                        return Err(RuntimeError::DivisionByZero);
                    }
                    let result = Value::int(self.get_int(base, lhs) % divisor);
                    self.set(base, dst, result);
                }
                Opcode::NegInt => {
                    let dst = reader.read_u8();
                    let src = reader.read_u8();
                    let result = Value::int(-self.get_int(base, src));
                    self.set(base, dst, result);
                }

                // Float arithmetic
                Opcode::AddFloat => {
                    let dst = reader.read_u8();
                    let lhs = reader.read_u8();
                    let rhs = reader.read_u8();
                    let result =
                        Value::float(self.get_float(base, lhs) + self.get_float(base, rhs));
                    self.set(base, dst, result);
                }
                Opcode::SubFloat => {
                    let dst = reader.read_u8();
                    let lhs = reader.read_u8();
                    let rhs = reader.read_u8();
                    let result =
                        Value::float(self.get_float(base, lhs) - self.get_float(base, rhs));
                    self.set(base, dst, result);
                }
                Opcode::MulFloat => {
                    let dst = reader.read_u8();
                    let lhs = reader.read_u8();
                    let rhs = reader.read_u8();
                    let result =
                        Value::float(self.get_float(base, lhs) * self.get_float(base, rhs));
                    self.set(base, dst, result);
                }
                Opcode::DivFloat => {
                    let dst = reader.read_u8();
                    let lhs = reader.read_u8();
                    let rhs = reader.read_u8();
                    let result =
                        Value::float(self.get_float(base, lhs) / self.get_float(base, rhs));
                    self.set(base, dst, result);
                }
                Opcode::NegFloat => {
                    let dst = reader.read_u8();
                    let src = reader.read_u8();
                    let result = Value::float(-self.get_float(base, src));
                    self.set(base, dst, result);
                }

                // Integer comparisons
                Opcode::EqInt => {
                    let dst = reader.read_u8();
                    let lhs = reader.read_u8();
                    let rhs = reader.read_u8();
                    let result = Value::bool(self.get_int(base, lhs) == self.get_int(base, rhs));
                    self.set(base, dst, result);
                }
                Opcode::NeInt => {
                    let dst = reader.read_u8();
                    let lhs = reader.read_u8();
                    let rhs = reader.read_u8();
                    let result = Value::bool(self.get_int(base, lhs) != self.get_int(base, rhs));
                    self.set(base, dst, result);
                }
                Opcode::LtInt => {
                    let dst = reader.read_u8();
                    let lhs = reader.read_u8();
                    let rhs = reader.read_u8();
                    let result = Value::bool(self.get_int(base, lhs) < self.get_int(base, rhs));
                    self.set(base, dst, result);
                }
                Opcode::LeInt => {
                    let dst = reader.read_u8();
                    let lhs = reader.read_u8();
                    let rhs = reader.read_u8();
                    let result = Value::bool(self.get_int(base, lhs) <= self.get_int(base, rhs));
                    self.set(base, dst, result);
                }
                Opcode::GtInt => {
                    let dst = reader.read_u8();
                    let lhs = reader.read_u8();
                    let rhs = reader.read_u8();
                    let result = Value::bool(self.get_int(base, lhs) > self.get_int(base, rhs));
                    self.set(base, dst, result);
                }
                Opcode::GeInt => {
                    let dst = reader.read_u8();
                    let lhs = reader.read_u8();
                    let rhs = reader.read_u8();
                    let result = Value::bool(self.get_int(base, lhs) >= self.get_int(base, rhs));
                    self.set(base, dst, result);
                }

                // Float comparisons
                Opcode::LtFloat => {
                    let dst = reader.read_u8();
                    let lhs = reader.read_u8();
                    let rhs = reader.read_u8();
                    let result = Value::bool(self.get_float(base, lhs) < self.get_float(base, rhs));
                    self.set(base, dst, result);
                }
                Opcode::LeFloat => {
                    let dst = reader.read_u8();
                    let lhs = reader.read_u8();
                    let rhs = reader.read_u8();
                    let result =
                        Value::bool(self.get_float(base, lhs) <= self.get_float(base, rhs));
                    self.set(base, dst, result);
                }
                Opcode::GtFloat => {
                    let dst = reader.read_u8();
                    let lhs = reader.read_u8();
                    let rhs = reader.read_u8();
                    let result = Value::bool(self.get_float(base, lhs) > self.get_float(base, rhs));
                    self.set(base, dst, result);
                }
                Opcode::GeFloat => {
                    let dst = reader.read_u8();
                    let lhs = reader.read_u8();
                    let rhs = reader.read_u8();
                    let result =
                        Value::bool(self.get_float(base, lhs) >= self.get_float(base, rhs));
                    self.set(base, dst, result);
                }

                // Boolean
                Opcode::Not => {
                    let dst = reader.read_u8();
                    let src = reader.read_u8();
                    let result = Value::bool(!self.get_bool(base, src));
                    self.set(base, dst, result);
                }

                // Control flow - jumps
                Opcode::JumpFwd => {
                    let offset = reader.read_u16();
                    reader.jump_forward(offset);
                }
                Opcode::JumpBack => {
                    let offset = reader.read_u16();
                    reader.jump_backward(offset);
                }
                Opcode::JumpIfFwd => {
                    let cond = reader.read_u8();
                    let offset = reader.read_u16();
                    if self.get_bool(base, cond) {
                        reader.jump_forward(offset);
                    }
                }
                Opcode::JumpIfBack => {
                    let cond = reader.read_u8();
                    let offset = reader.read_u16();
                    if self.get_bool(base, cond) {
                        reader.jump_backward(offset);
                    }
                }
                Opcode::JumpIfNotFwd => {
                    let cond = reader.read_u8();
                    let offset = reader.read_u16();
                    if !self.get_bool(base, cond) {
                        reader.jump_forward(offset);
                    }
                }
                Opcode::JumpIfNotBack => {
                    let cond = reader.read_u8();
                    let offset = reader.read_u16();
                    if !self.get_bool(base, cond) {
                        reader.jump_backward(offset);
                    }
                }

                // Function calls
                Opcode::Call => {
                    let dst = reader.read_u8();
                    let func_idx = reader.read_u16();
                    let arg_base = reader.read_u8();
                    let arg_count = reader.read_u8();
                    let dst_opt = if dst == NO_SLOT { None } else { Some(dst) };
                    self.do_call(
                        &mut reader,
                        dst_opt,
                        func_idx as usize,
                        arg_base,
                        arg_count,
                        None,
                    )?;
                    // Switch reader to new chunk
                    reader = BytecodeReader::new(&self.chunks[self.current_chunk].code);
                }

                Opcode::CallIndirect => {
                    let dst = reader.read_u8();
                    let callee = reader.read_u8();
                    let arg_base = reader.read_u8();
                    let arg_count = reader.read_u8();
                    let closure_idx = self.get_closure_idx(base, callee);
                    let func_idx = self.heap.get_closure(closure_idx).func_idx as usize;
                    let dst_opt = if dst == NO_SLOT { None } else { Some(dst) };
                    self.do_call(
                        &mut reader,
                        dst_opt,
                        func_idx,
                        arg_base,
                        arg_count,
                        Some(closure_idx),
                    )?;
                    // Switch reader to new chunk
                    reader = BytecodeReader::new(&self.chunks[self.current_chunk].code);
                }

                Opcode::Return => {
                    let src = reader.read_u8();
                    let result = if src == NO_SLOT {
                        Value::unit()
                    } else {
                        self.get(base, src)
                    };

                    if self.frame_count > 0 {
                        self.frame_count -= 1;
                        let frame = self.frames[self.frame_count];

                        // Return to caller
                        self.current_chunk = frame.return_chunk;

                        // Deallocate frame by resetting stack_top
                        self.stack_top = frame.stack_base;

                        // Update stack_base to caller's base
                        self.stack_base = if self.frame_count > 0 {
                            self.frames[self.frame_count - 1].stack_base
                        } else {
                            0
                        };
                        let caller_base = self.stack_base;

                        // Store result in caller's slot
                        if let Some(slot) = frame.result_slot {
                            let idx = caller_base + slot as usize;
                            self.stack[idx] = result;
                        }

                        // Switch reader back to caller's chunk and position
                        reader = BytecodeReader::new(&self.chunks[self.current_chunk].code);
                        reader.set_pc(frame.return_pc);
                    } else {
                        // Return from main
                        let result = self.materialize_value(result);
                        return Ok((result, self.heap));
                    }
                }

                // Closures
                Opcode::MakeClosure => {
                    let dst = reader.read_u8();
                    let func_idx = reader.read_u16();
                    let capture_base = reader.read_u8();
                    let capture_count = reader.read_u8();

                    // GC before allocation (while captures are still on stack as roots)
                    self.maybe_gc();

                    let mut captures = Vec::with_capacity(capture_count as usize);
                    for i in 0..capture_count {
                        let idx = base + capture_base as usize + i as usize;
                        captures.push(self.stack[idx]);
                    }

                    let closure_idx = self.heap.alloc_closure(func_idx as u32, captures);
                    self.set(base, dst, Value::closure(closure_idx));
                }

                Opcode::LoadCapture => {
                    let dst = reader.read_u8();
                    let index = reader.read_u8();
                    let closure_idx = self.current_closure_idx()?;
                    let closure = self.heap.get_closure(closure_idx);
                    let val = closure
                        .captures
                        .get(index as usize)
                        .map(|c| c.get())
                        .ok_or(RuntimeError::CaptureOutOfBounds(index))?;
                    self.set(base, dst, val);
                }

                Opcode::StoreCapture => {
                    let index = reader.read_u8();
                    let src = reader.read_u8();
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
                Opcode::Echo => {
                    let src = reader.read_u8();
                    let val = self.get(base, src);
                    println!("{}", val.display_with_interner(&self.heap, self.strings));
                }

                // End
                Opcode::Halt => {
                    return Ok((Value::unit(), self.heap));
                }
            }
        }
    }

    /// If value is an interned string, resolve it to a dynamic string.
    fn materialize_value(&mut self, value: Value) -> Value {
        if let Some(spur) = value.as_interned_string() {
            let s = self.strings.resolve(&spur).to_string();
            let idx = self.heap.alloc_string(s);
            Value::dynamic_string(idx)
        } else {
            value
        }
    }

    /// Collect GC roots from VM state.
    fn gc_roots(&self) -> Vec<Value> {
        let mut roots = Vec::with_capacity(self.stack_top + self.frame_count);

        // All active stack values are roots
        roots.extend(self.stack[..self.stack_top].iter().copied());

        // Closure indices from active call frames are roots
        for i in 0..self.frame_count {
            if let Some(closure_idx) = self.frames[i].closure_idx {
                roots.push(Value::closure(closure_idx));
            }
        }

        roots
    }

    /// Run GC if threshold exceeded. Call after allocations.
    fn maybe_gc(&mut self) {
        if self.heap.should_gc() {
            let roots = self.gc_roots();
            self.heap.collect(roots.into_iter());
        }
    }

    fn load_const(
        &mut self,
        pool: &compile::ConstantPool,
        idx: ConstIdx,
    ) -> Result<Value, RuntimeError> {
        match pool.get(idx) {
            Some(compile::Constant::Float(f)) => Ok(Value::float(*f)),
            Some(compile::Constant::StringRef(spur)) => {
                // No clone! Just store the Spur key directly
                Ok(Value::interned_string(*spur))
            }
            None => Err(RuntimeError::InvalidConstant(idx.0)),
        }
    }
}
