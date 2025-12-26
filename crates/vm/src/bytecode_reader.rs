use compile::Opcode;

/// Reader for compact bytecode decoding using raw pointers for speed.
pub struct BytecodeReader {
    /// Current position pointer
    ptr: *const u8,
    /// Start of bytecode (for pc calculation)
    start: *const u8,
}

impl BytecodeReader {
    #[inline]
    pub fn new(code: &[u8]) -> Self {
        Self {
            ptr: code.as_ptr(),
            start: code.as_ptr(),
        }
    }

    #[inline(always)]
    pub fn pc(&self) -> usize {
        // NOTE: ptr >= start always, both from same allocation
        unsafe { self.ptr.offset_from(self.start) as usize }
    }

    /// Read opcode (unchecked)
    #[inline(always)]
    pub fn read_opcode(&mut self) -> Opcode {
        let byte = self.read_u8();
        // NOTE: Bytecode is well-formed, byte is valid opcode
        unsafe { Opcode::from_u8_unchecked(byte) }
    }

    #[inline(always)]
    pub fn read_u8(&mut self) -> u8 {
        // NOTE: Caller ensures we don't read past end
        unsafe {
            let v = *self.ptr;
            self.ptr = self.ptr.add(1);
            v
        }
    }

    #[inline(always)]
    pub fn read_u16(&mut self) -> u16 {
        // NOTE: Caller ensures we don't read past end
        unsafe {
            let v = std::ptr::read_unaligned(self.ptr as *const u16);
            self.ptr = self.ptr.add(2);
            u16::from_le(v)
        }
    }

    #[inline(always)]
    pub fn read_i64(&mut self) -> i64 {
        // NOTE: Caller ensures we don't read past end
        unsafe {
            let v = std::ptr::read_unaligned(self.ptr as *const i64);
            self.ptr = self.ptr.add(8);
            i64::from_le(v)
        }
    }

    /// Jump forward by offset bytes
    #[inline(always)]
    pub fn jump_forward(&mut self, offset: u16) {
        // NOTE: Compiler ensures jumps are within bounds
        unsafe { self.ptr = self.ptr.add(offset as usize) };
    }

    /// Jump backward by offset bytes
    #[inline(always)]
    pub fn jump_backward(&mut self, offset: u16) {
        // NOTE: Compiler ensures jumps are within bounds
        unsafe { self.ptr = self.ptr.sub(offset as usize) };
    }

    /// Switch to different bytecode at given PC (avoids creating new reader)
    #[inline(always)]
    pub fn switch_code(&mut self, code: &[u8], pc: usize) {
        self.start = code.as_ptr();
        // NOTE: Caller ensures pc is valid within code
        self.ptr = unsafe { code.as_ptr().add(pc) };
    }
}
