//! Compact 16-byte runtime value representation.

use std::cell::Cell;

/// Value tag discriminant
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ValueTag {
    Unit = 0,
    Bool = 1,
    Int = 2,
    Float = 3,
    String = 4,
    Closure = 5,
}

/// Compact 16-byte runtime value.
///
/// Heap-allocated types (String, Closure) store an index into the Heap pools.
#[derive(Clone, Copy)]
#[repr(C)]
pub struct Value {
    tag: ValueTag,
    _pad: [u8; 7],
    bits: u64,
}

impl Value {
    #[inline]
    pub const fn unit() -> Self {
        Self {
            tag: ValueTag::Unit,
            _pad: [0; 7],
            bits: 0,
        }
    }

    #[inline]
    pub const fn bool(b: bool) -> Self {
        Self {
            tag: ValueTag::Bool,
            _pad: [0; 7],
            bits: b as u64,
        }
    }

    #[inline]
    pub const fn int(n: i64) -> Self {
        Self {
            tag: ValueTag::Int,
            _pad: [0; 7],
            bits: n as u64,
        }
    }

    #[inline]
    pub fn float(f: f64) -> Self {
        Self {
            tag: ValueTag::Float,
            _pad: [0; 7],
            bits: f.to_bits(),
        }
    }

    #[inline]
    pub const fn string(idx: u32) -> Self {
        Self {
            tag: ValueTag::String,
            _pad: [0; 7],
            bits: idx as u64,
        }
    }

    #[inline]
    pub const fn closure(idx: u32) -> Self {
        Self {
            tag: ValueTag::Closure,
            _pad: [0; 7],
            bits: idx as u64,
        }
    }

    #[inline]
    pub fn tag(&self) -> ValueTag {
        self.tag
    }

    #[inline]
    pub fn as_bool(&self) -> Option<bool> {
        if self.tag == ValueTag::Bool {
            Some(self.bits != 0)
        } else {
            None
        }
    }

    #[inline]
    pub fn as_int(&self) -> Option<i64> {
        if self.tag == ValueTag::Int {
            Some(self.bits as i64)
        } else {
            None
        }
    }

    #[inline]
    pub fn as_float(&self) -> Option<f64> {
        if self.tag == ValueTag::Float {
            Some(f64::from_bits(self.bits))
        } else {
            None
        }
    }

    #[inline]
    pub fn as_string_idx(&self) -> Option<u32> {
        if self.tag == ValueTag::String {
            Some(self.bits as u32)
        } else {
            None
        }
    }

    #[inline]
    pub fn as_closure_idx(&self) -> Option<u32> {
        if self.tag == ValueTag::Closure {
            Some(self.bits as u32)
        } else {
            None
        }
    }

    #[inline]
    pub fn is_unit(&self) -> bool {
        self.tag == ValueTag::Unit
    }

    pub fn type_name(&self) -> &'static str {
        match self.tag {
            ValueTag::Unit => "Unit",
            ValueTag::Bool => "Bool",
            ValueTag::Int => "Int",
            ValueTag::Float => "Float",
            ValueTag::String => "String",
            ValueTag::Closure => "Closure",
        }
    }

    /// Display value as string, requires heap for String/Closure lookup.
    pub fn display(&self, heap: &Heap) -> String {
        match self.tag {
            ValueTag::Unit => "()".to_string(),
            ValueTag::Bool => format!("{}", self.bits != 0),
            ValueTag::Int => format!("{}", self.bits as i64),
            ValueTag::Float => format!("{}", f64::from_bits(self.bits)),
            ValueTag::String => heap.get_string(self.bits as u32).to_string(),
            ValueTag::Closure => {
                let closure = heap.get_closure(self.bits as u32);
                format!("<closure fn{}>", closure.func_idx)
            }
        }
    }

    /// Structural equality, requires heap for String/Closure comparison.
    pub fn eq(&self, other: &Value, heap: &Heap) -> bool {
        if self.tag != other.tag {
            return false;
        }
        match self.tag {
            ValueTag::Unit => true,
            ValueTag::Bool | ValueTag::Int => self.bits == other.bits,
            ValueTag::Float => {
                let a = f64::from_bits(self.bits);
                let b = f64::from_bits(other.bits);
                a == b
            }
            ValueTag::String => {
                let a = heap.get_string(self.bits as u32);
                let b = heap.get_string(other.bits as u32);
                a == b
            }
            ValueTag::Closure => {
                // Closures equal if same func and same captures
                let a = heap.get_closure(self.bits as u32);
                let b = heap.get_closure(other.bits as u32);
                if a.func_idx != b.func_idx {
                    return false;
                }
                if a.captures.len() != b.captures.len() {
                    return false;
                }
                for (ca, cb) in a.captures.iter().zip(b.captures.iter()) {
                    if !ca.get().eq(&cb.get(), heap) {
                        return false;
                    }
                }
                true
            }
        }
    }
}

impl std::fmt::Debug for Value {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self.tag {
            ValueTag::Unit => write!(f, "Unit"),
            ValueTag::Bool => write!(f, "Bool({})", self.bits != 0),
            ValueTag::Int => write!(f, "Int({})", self.bits as i64),
            ValueTag::Float => write!(f, "Float({})", f64::from_bits(self.bits)),
            ValueTag::String => write!(f, "String(idx={})", self.bits),
            ValueTag::Closure => write!(f, "Closure(idx={})", self.bits),
        }
    }
}

/// Closure data stored in heap pool.
pub struct ClosureData {
    pub func_idx: u32,
    pub captures: Box<[Cell<Value>]>,
}

/// Heap for strings and closures.
pub struct Heap {
    strings: Vec<String>,
    closures: Vec<ClosureData>,
}

impl Heap {
    pub fn new() -> Self {
        Self {
            strings: Vec::new(),
            closures: Vec::new(),
        }
    }

    pub fn alloc_string(&mut self, s: String) -> u32 {
        let idx = self.strings.len() as u32;
        self.strings.push(s);
        idx
    }

    pub fn get_string(&self, idx: u32) -> &str {
        &self.strings[idx as usize]
    }

    pub fn alloc_closure(&mut self, func_idx: u32, captures: Vec<Value>) -> u32 {
        let idx = self.closures.len() as u32;
        self.closures.push(ClosureData {
            func_idx,
            captures: captures.into_iter().map(Cell::new).collect(),
        });
        idx
    }

    pub fn get_closure(&self, idx: u32) -> &ClosureData {
        &self.closures[idx as usize]
    }
}

impl Default for Heap {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_value_size() {
        assert_eq!(std::mem::size_of::<Value>(), 16);
    }

    #[test]
    fn test_primitives() {
        assert!(Value::unit().is_unit());
        assert_eq!(Value::bool(true).as_bool(), Some(true));
        assert_eq!(Value::bool(false).as_bool(), Some(false));
        assert_eq!(Value::int(42).as_int(), Some(42));
        assert_eq!(Value::int(-1).as_int(), Some(-1));
        assert_eq!(Value::float(2.5).as_float(), Some(2.5));
    }

    #[test]
    fn test_heap_string() {
        let mut heap = Heap::new();
        let idx = heap.alloc_string("hello".to_string());
        assert_eq!(heap.get_string(idx), "hello");

        let val = Value::string(idx);
        assert_eq!(val.as_string_idx(), Some(idx));
        assert_eq!(val.display(&heap), "hello");
    }

    #[test]
    fn test_heap_closure() {
        let mut heap = Heap::new();
        let captures = vec![Value::int(10), Value::int(20)];
        let idx = heap.alloc_closure(5, captures);

        let closure = heap.get_closure(idx);
        assert_eq!(closure.func_idx, 5);
        assert_eq!(closure.captures.len(), 2);
        assert_eq!(closure.captures[0].get().as_int(), Some(10));
    }

    #[test]
    fn test_mutable_captures() {
        let mut heap = Heap::new();
        let idx = heap.alloc_closure(0, vec![Value::int(1)]);

        let closure = heap.get_closure(idx);
        closure.captures[0].set(Value::int(99));
        assert_eq!(closure.captures[0].get().as_int(), Some(99));
    }

    #[test]
    fn test_structural_equality() {
        let mut heap = Heap::new();

        // Primitives
        assert!(Value::int(5).eq(&Value::int(5), &heap));
        assert!(!Value::int(5).eq(&Value::int(6), &heap));

        // Strings
        let s1 = heap.alloc_string("test".to_string());
        let s2 = heap.alloc_string("test".to_string());
        assert!(Value::string(s1).eq(&Value::string(s2), &heap));
    }
}
