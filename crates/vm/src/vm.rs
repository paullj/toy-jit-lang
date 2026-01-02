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

        let mut heap = Heap::new();
        // Register struct metadata for display purposes
        for meta in &module.struct_metadata {
            heap.register_struct_meta(meta.struct_id, meta.name.clone(), meta.field_names.clone());
        }

        Self {
            stack: Box::new([Value::unit(); MAX_STACK_SIZE]),
            stack_top: frame_size,
            heap,
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

    /// Get list index from slot (unchecked in release)
    #[inline(always)]
    fn get_list_idx(&self, base: usize, slot: u8) -> u32 {
        self.get(base, slot).as_list_idx_unchecked()
    }

    /// Normalize list index (handle negative indexing)
    #[inline(always)]
    fn normalize_index(index: i64, len: usize) -> Result<usize, RuntimeError> {
        let normalized = if index < 0 { len as i64 + index } else { index };
        if normalized < 0 || normalized as usize >= len {
            return Err(RuntimeError::IndexOutOfBounds { index, len });
        }
        Ok(normalized as usize)
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

    #[inline(always)]
    fn do_call(
        &mut self,
        reader: &mut BytecodeReader,
        dst: Option<u8>,
        func_idx: usize,
        arg_base: u8,
        arg_count: u8,
        closure_idx: Option<u32>,
    ) -> Result<(), RuntimeError> {
        // SAFETY: func_idx validated at compile time; bounds checked in debug mode
        debug_assert!(func_idx < self.chunks.len(), "invalid function index");
        let chunk = unsafe { self.chunks.get_unchecked(func_idx) };

        let caller_base = self.stack_base; // Use cached value

        // New frame starts at current stack top
        let new_base = self.stack_top;
        let new_frame_size = chunk.local_count as usize + chunk.register_count as usize;
        let new_top = new_base + new_frame_size;

        // Check for stack overflow
        if new_top > MAX_STACK_SIZE {
            return Err(RuntimeError::StackOverflow);
        }

        // Copy args first, then zero remaining slots (avoids redundant zeroing)
        let arg_start = caller_base + arg_base as usize;
        match arg_count {
            0 => {}
            1 => {
                self.stack[new_base] = self.stack[arg_start];
            }
            2 => {
                self.stack[new_base] = self.stack[arg_start];
                self.stack[new_base + 1] = self.stack[arg_start + 1];
            }
            3 => {
                self.stack[new_base] = self.stack[arg_start];
                self.stack[new_base + 1] = self.stack[arg_start + 1];
                self.stack[new_base + 2] = self.stack[arg_start + 2];
            }
            _ => {
                for i in 0..arg_count as usize {
                    self.stack[new_base + i] = self.stack[arg_start + i];
                }
            }
        }

        // Zero remaining slots (after args)
        let arg_end = new_base + arg_count as usize;
        for i in arg_end..new_top {
            self.stack[i] = Value::unit();
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

        // Switch reader to new chunk code (avoids re-indexing after return)
        reader.switch_code(&chunk.code, 0);

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
                    // do_call switches reader to new chunk's code
                    self.do_call(
                        &mut reader,
                        dst_opt,
                        func_idx as usize,
                        arg_base,
                        arg_count,
                        None,
                    )?;
                }

                Opcode::CallIndirect => {
                    let dst = reader.read_u8();
                    let callee = reader.read_u8();
                    let arg_base = reader.read_u8();
                    let arg_count = reader.read_u8();
                    let closure_idx = self.get_closure_idx(base, callee);
                    let func_idx = self.heap.get_closure(closure_idx).func_idx as usize;
                    let dst_opt = if dst == NO_SLOT { None } else { Some(dst) };
                    // do_call switches reader to new chunk's code
                    self.do_call(
                        &mut reader,
                        dst_opt,
                        func_idx,
                        arg_base,
                        arg_count,
                        Some(closure_idx),
                    )?;
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

                        // Switch reader back to caller's chunk and position (in-place)
                        reader.switch_code(&self.chunks[self.current_chunk].code, frame.return_pc);
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

                // List operations
                Opcode::ListNew => {
                    let dst = reader.read_u8();
                    let capacity = reader.read_u8();
                    self.maybe_gc();
                    let list_idx = self.heap.alloc_list(capacity as usize);
                    let result = Value::list(list_idx);
                    self.set(base, dst, result);
                }

                Opcode::ListSet => {
                    let list = reader.read_u8();
                    let index = reader.read_u8();
                    let value = reader.read_u8();
                    let list_idx = self.get_list_idx(base, list);
                    let idx = self.get_int(base, index);
                    let val = self.get(base, value);

                    let list_data = self.heap.get_list_mut(list_idx);
                    let len = list_data.elements.len();
                    let normalized = Self::normalize_index(idx, len)?;
                    list_data.elements[normalized] = val;
                }

                Opcode::ListGet => {
                    let dst = reader.read_u8();
                    let list = reader.read_u8();
                    let index = reader.read_u8();
                    let list_idx = self.get_list_idx(base, list);
                    let idx = self.get_int(base, index);

                    let list_data = self.heap.get_list(list_idx);
                    let len = list_data.elements.len();
                    let normalized = Self::normalize_index(idx, len)?;
                    let result = list_data.elements[normalized];
                    self.set(base, dst, result);
                }

                Opcode::ListSlice => {
                    let dst = reader.read_u8();
                    let list = reader.read_u8();
                    let start = reader.read_u8();
                    let end = reader.read_u8();
                    self.maybe_gc();
                    let list_idx = self.get_list_idx(base, list);
                    let start_val = self.get_int(base, start);
                    let end_val = self.get_int(base, end);

                    let list_data = self.heap.get_list(list_idx);
                    let len = list_data.elements.len();

                    // SLICE_MISSING is sentinel for "missing" bounds
                    let start_idx = if start_val == compile::SLICE_MISSING {
                        0
                    } else if start_val < 0 {
                        (len as i64 + start_val).max(0) as usize
                    } else {
                        (start_val as usize).min(len)
                    };

                    let end_idx = if end_val == compile::SLICE_MISSING {
                        len
                    } else if end_val < 0 {
                        (len as i64 + end_val).max(0) as usize
                    } else {
                        (end_val as usize).min(len)
                    };

                    let slice: Vec<Value> = if start_idx < end_idx {
                        list_data.elements[start_idx..end_idx].to_vec()
                    } else {
                        Vec::new()
                    };

                    let new_list_idx = self.heap.alloc_list(slice.len());
                    self.heap.get_list_mut(new_list_idx).elements = slice;
                    let result = Value::list(new_list_idx);
                    self.set(base, dst, result);
                }
                Opcode::ListLen => {
                    let dst = reader.read_u8();
                    let list = reader.read_u8();
                    let list_idx = self.get_list_idx(base, list);
                    let list_data = self.heap.get_list(list_idx);
                    let len = list_data.elements.len() as i64;
                    self.set(base, dst, Value::int(len));
                }
                // Tuple operations
                Opcode::TupleNew => {
                    let dst = reader.read_u8();
                    let elem_base = reader.read_u8();
                    let elem_count = reader.read_u8();
                    self.maybe_gc();

                    let elements: Vec<Value> = (0..elem_count)
                        .map(|i| self.get(base, elem_base + i))
                        .collect();

                    let tuple_idx = self.heap.alloc_tuple(elements);
                    let result = Value::tuple(tuple_idx);
                    self.set(base, dst, result);
                }

                Opcode::TupleGet => {
                    let dst = reader.read_u8();
                    let tuple = reader.read_u8();
                    let index = reader.read_u8();

                    let tuple_idx = self.get(base, tuple).as_tuple_idx_unchecked();
                    let tuple_data = self.heap.get_tuple(tuple_idx);
                    let result = tuple_data.elements[index as usize];
                    self.set(base, dst, result);
                }
                // Struct operations
                Opcode::StructNew => {
                    let dst = reader.read_u8();
                    let struct_id = reader.read_u16();
                    let field_base = reader.read_u8();
                    let field_count = reader.read_u8();
                    self.maybe_gc();

                    let fields: Vec<Value> = (0..field_count)
                        .map(|i| self.get(base, field_base + i))
                        .collect();

                    let struct_idx = self.heap.alloc_struct(struct_id as u32, fields);
                    let result = Value::struct_obj(struct_idx);
                    self.set(base, dst, result);
                }
                Opcode::StructGet => {
                    let dst = reader.read_u8();
                    let struct_ref = reader.read_u8();
                    let field_index = reader.read_u8();

                    let struct_idx = self.get(base, struct_ref).as_struct_idx_unchecked();
                    let struct_data = self.heap.get_struct(struct_idx);
                    let result = struct_data.fields[field_index as usize];
                    self.set(base, dst, result);
                }
                Opcode::StructSet => {
                    let struct_ref = reader.read_u8();
                    let field_index = reader.read_u8();
                    let value = reader.read_u8();

                    let struct_idx = self.get(base, struct_ref).as_struct_idx_unchecked();
                    let val = self.get(base, value);
                    let struct_data = self.heap.get_struct_mut(struct_idx);
                    struct_data.fields[field_index as usize] = val;
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
