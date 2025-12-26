//! NaN-boxed 8-byte runtime value representation.
//!
//! Uses IEEE 754 quiet NaN encoding to pack all value types into 64 bits:
//! - Floats: raw IEEE 754 double
//! - Other types: quiet NaN + 3-bit tag + 48-bit payload

use std::cell::Cell;

use compile::Spur;
use lasso::{Key, Rodeo};

// NaN boxing constants
// Use negative quiet NaN (sign=1, exp=0x7FF, quiet=1) to avoid collisions with real NaNs
// Bits 48-50 are free for tags, bits 0-47 for payload
const QNAN: u64 = 0xFFF8_0000_0000_0000;
const TAG_MASK: u64 = 0x0007_0000_0000_0000; // 3 bits for type tag (bits 48-50)
const PAYLOAD_MASK: u64 = 0x0000_FFFF_FFFF_FFFF; // 48-bit payload

// Tags (bits 48-50) - OR'd with QNAN
const TAG_INT: u64 = 0x0001_0000_0000_0000; // 001
const TAG_BOOL: u64 = 0x0002_0000_0000_0000; // 010
const TAG_UNIT: u64 = 0x0003_0000_0000_0000; // 011
const TAG_INTERNED: u64 = 0x0004_0000_0000_0000; // 100
const TAG_DYNSTR: u64 = 0x0005_0000_0000_0000; // 101
const TAG_CLOSURE: u64 = 0x0006_0000_0000_0000; // 110

// Combined patterns for fast checking
const QNAN_INT: u64 = QNAN | TAG_INT;
const QNAN_BOOL: u64 = QNAN | TAG_BOOL;
const QNAN_UNIT: u64 = QNAN | TAG_UNIT;
const QNAN_INTERNED: u64 = QNAN | TAG_INTERNED;
const QNAN_DYNSTR: u64 = QNAN | TAG_DYNSTR;
const QNAN_CLOSURE: u64 = QNAN | TAG_CLOSURE;

// Mask for type checking: QNAN + TAG
const TYPE_MASK: u64 = QNAN | TAG_MASK;

/// NaN-boxed 8-byte runtime value.
///
/// Encoding:
/// - Float: raw IEEE 754 double (not a quiet NaN)
/// - Int: QNAN | TAG_INT | 48-bit signed payload
/// - Bool: QNAN | TAG_BOOL | 0 or 1
/// - Unit: QNAN | TAG_UNIT
/// - InternedString: QNAN | TAG_INTERNED | spur index
/// - DynamicString: QNAN | TAG_DYNSTR | heap index
/// - Closure: QNAN | TAG_CLOSURE | heap index
#[derive(Clone, Copy)]
#[repr(transparent)]
pub struct Value(u64);

impl Value {
    #[inline(always)]
    pub const fn unit() -> Self {
        Self(QNAN_UNIT)
    }

    #[inline(always)]
    pub const fn bool(b: bool) -> Self {
        Self(QNAN_BOOL | (b as u64))
    }

    #[inline(always)]
    pub const fn int(n: i64) -> Self {
        // Mask to 48 bits (preserves sign via two's complement)
        Self(QNAN_INT | ((n as u64) & PAYLOAD_MASK))
    }

    #[inline(always)]
    pub fn float(f: f64) -> Self {
        Self(f.to_bits())
    }

    /// Create an interned string value (references module string table)
    #[inline(always)]
    pub fn interned_string(spur: Spur) -> Self {
        Self(QNAN_INTERNED | (spur.into_usize() as u64))
    }

    /// Create a dynamic string value (references heap)
    #[inline(always)]
    pub const fn dynamic_string(idx: u32) -> Self {
        Self(QNAN_DYNSTR | (idx as u64))
    }

    #[inline(always)]
    pub const fn closure(idx: u32) -> Self {
        Self(QNAN_CLOSURE | (idx as u64))
    }

    /// Check if this is a float (not NaN-boxed)
    #[inline(always)]
    pub fn is_float(&self) -> bool {
        (self.0 & QNAN) != QNAN
    }

    /// Check if this is an integer
    #[inline(always)]
    pub fn is_int(&self) -> bool {
        (self.0 & TYPE_MASK) == QNAN_INT
    }

    /// Check if this is a boolean
    #[inline(always)]
    pub fn is_bool(&self) -> bool {
        (self.0 & TYPE_MASK) == QNAN_BOOL
    }

    /// Check if this is unit
    #[inline(always)]
    pub fn is_unit(&self) -> bool {
        (self.0 & TYPE_MASK) == QNAN_UNIT
    }

    /// Check if this is an interned string
    #[inline(always)]
    pub fn is_interned_string(&self) -> bool {
        (self.0 & TYPE_MASK) == QNAN_INTERNED
    }

    /// Check if this is a dynamic string
    #[inline(always)]
    pub fn is_dynamic_string(&self) -> bool {
        (self.0 & TYPE_MASK) == QNAN_DYNSTR
    }

    /// Check if this is a closure
    #[inline(always)]
    pub fn is_closure(&self) -> bool {
        (self.0 & TYPE_MASK) == QNAN_CLOSURE
    }

    /// Check if this is any string type
    #[inline(always)]
    pub fn is_string(&self) -> bool {
        self.is_interned_string() || self.is_dynamic_string()
    }

    #[inline(always)]
    pub fn as_bool(&self) -> Option<bool> {
        if self.is_bool() {
            Some((self.0 & 1) != 0)
        } else {
            None
        }
    }

    /// Get bool value. Panics in debug if wrong type.
    #[inline(always)]
    pub fn as_bool_unchecked(&self) -> bool {
        debug_assert!(self.is_bool(), "expected Bool");
        (self.0 & 1) != 0
    }

    #[inline(always)]
    pub fn as_int(&self) -> Option<i64> {
        if self.is_int() {
            Some(self.extract_signed_payload())
        } else {
            None
        }
    }

    /// Get int value. Panics in debug if wrong type.
    #[inline(always)]
    pub fn as_int_unchecked(&self) -> i64 {
        debug_assert!(self.is_int(), "expected Int");
        self.extract_signed_payload()
    }

    /// Extract 48-bit payload as signed i64 (sign-extend)
    #[inline(always)]
    fn extract_signed_payload(&self) -> i64 {
        let payload = self.0 & PAYLOAD_MASK;
        // Sign-extend from 48 bits: shift left then arithmetic shift right
        ((payload as i64) << 16) >> 16
    }

    #[inline(always)]
    pub fn as_float(&self) -> Option<f64> {
        if self.is_float() {
            Some(f64::from_bits(self.0))
        } else {
            None
        }
    }

    /// Get float value. Panics in debug if wrong type.
    #[inline(always)]
    pub fn as_float_unchecked(&self) -> f64 {
        debug_assert!(self.is_float(), "expected Float");
        f64::from_bits(self.0)
    }

    #[inline(always)]
    pub fn as_interned_string(&self) -> Option<Spur> {
        if self.is_interned_string() {
            let payload = (self.0 & PAYLOAD_MASK) as usize;
            Spur::try_from_usize(payload)
        } else {
            None
        }
    }

    #[inline(always)]
    pub fn as_dynamic_string_idx(&self) -> Option<u32> {
        if self.is_dynamic_string() {
            Some((self.0 & PAYLOAD_MASK) as u32)
        } else {
            None
        }
    }

    #[inline(always)]
    pub fn as_closure_idx(&self) -> Option<u32> {
        if self.is_closure() {
            Some((self.0 & PAYLOAD_MASK) as u32)
        } else {
            None
        }
    }

    /// Get closure index. Panics in debug if wrong type.
    #[inline(always)]
    pub fn as_closure_idx_unchecked(&self) -> u32 {
        debug_assert!(self.is_closure(), "expected Closure");
        (self.0 & PAYLOAD_MASK) as u32
    }

    pub fn type_name(&self) -> &'static str {
        if self.is_float() {
            "Float"
        } else {
            match self.0 & TYPE_MASK {
                QNAN_INT => "Int",
                QNAN_BOOL => "Bool",
                QNAN_UNIT => "Unit",
                QNAN_INTERNED | QNAN_DYNSTR => "String",
                QNAN_CLOSURE => "Closure",
                _ => "Unknown",
            }
        }
    }

    /// Display value as string (for results after execution).
    /// Panics if value is an interned string (should be materialized before returning).
    pub fn display(&self, heap: &Heap) -> String {
        if self.is_float() {
            return format!("{}", f64::from_bits(self.0));
        }

        match self.0 & TYPE_MASK {
            QNAN_UNIT => "()".to_string(),
            QNAN_BOOL => format!("{}", (self.0 & 1) != 0),
            QNAN_INT => format!("{}", self.extract_signed_payload()),
            QNAN_INTERNED => {
                panic!("cannot display interned string without interner - should be materialized")
            }
            QNAN_DYNSTR => {
                let idx = (self.0 & PAYLOAD_MASK) as u32;
                heap.get_string(idx).to_string()
            }
            QNAN_CLOSURE => {
                let idx = (self.0 & PAYLOAD_MASK) as u32;
                let closure = heap.get_closure(idx);
                format!("<closure fn{}>", closure.func_idx)
            }
            _ => "<unknown>".to_string(),
        }
    }

    /// Display value with interner (for use during execution with Echo).
    pub fn display_with_interner(&self, heap: &Heap, interner: &Rodeo) -> String {
        if self.is_float() {
            return format!("{}", f64::from_bits(self.0));
        }

        match self.0 & TYPE_MASK {
            QNAN_UNIT => "()".to_string(),
            QNAN_BOOL => format!("{}", (self.0 & 1) != 0),
            QNAN_INT => format!("{}", self.extract_signed_payload()),
            QNAN_INTERNED => {
                let payload = (self.0 & PAYLOAD_MASK) as usize;
                let spur = Spur::try_from_usize(payload).expect("invalid interned string spur");
                interner.resolve(&spur).to_string()
            }
            QNAN_DYNSTR => {
                let idx = (self.0 & PAYLOAD_MASK) as u32;
                heap.get_string(idx).to_string()
            }
            QNAN_CLOSURE => {
                let idx = (self.0 & PAYLOAD_MASK) as u32;
                let closure = heap.get_closure(idx);
                format!("<closure fn{}>", closure.func_idx)
            }
            _ => "<unknown>".to_string(),
        }
    }

    /// Structural equality, requires heap and interner for string comparison.
    pub fn eq(&self, other: &Value, heap: &Heap, interner: &Rodeo) -> bool {
        // Fast path: bitwise equal (works for all non-NaN types)
        if self.0 == other.0 {
            // For floats, NaN != NaN, so check that
            if self.is_float() {
                let f = f64::from_bits(self.0);
                return !f.is_nan();
            }
            return true;
        }

        // Different bits - check type match
        let self_type = self.0 & TYPE_MASK;
        let other_type = other.0 & TYPE_MASK;

        // Floats
        if self.is_float() && other.is_float() {
            let a = f64::from_bits(self.0);
            let b = f64::from_bits(other.0);
            return a == b;
        }

        // String comparison (any combination of interned/dynamic)
        if (self_type == QNAN_INTERNED || self_type == QNAN_DYNSTR)
            && (other_type == QNAN_INTERNED || other_type == QNAN_DYNSTR)
        {
            let a = self.get_str(heap, interner);
            let b = other.get_str(heap, interner);
            return a == b;
        }

        // Closure comparison
        if self_type == QNAN_CLOSURE && other_type == QNAN_CLOSURE {
            let a_idx = (self.0 & PAYLOAD_MASK) as u32;
            let b_idx = (other.0 & PAYLOAD_MASK) as u32;
            let a = heap.get_closure(a_idx);
            let b = heap.get_closure(b_idx);
            if a.func_idx != b.func_idx {
                return false;
            }
            if a.captures.len() != b.captures.len() {
                return false;
            }
            for (ca, cb) in a.captures.iter().zip(b.captures.iter()) {
                if !ca.get().eq(&cb.get(), heap, interner) {
                    return false;
                }
            }
            return true;
        }

        false
    }

    /// Get string content (works for both interned and dynamic)
    fn get_str<'a>(&self, heap: &'a Heap, interner: &'a Rodeo) -> &'a str {
        match self.0 & TYPE_MASK {
            QNAN_INTERNED => {
                let payload = (self.0 & PAYLOAD_MASK) as usize;
                let spur = Spur::try_from_usize(payload).expect("invalid interned string spur");
                interner.resolve(&spur)
            }
            QNAN_DYNSTR => {
                let idx = (self.0 & PAYLOAD_MASK) as u32;
                heap.get_string(idx)
            }
            _ => panic!("not a string"),
        }
    }
}

impl std::fmt::Debug for Value {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.is_float() {
            return write!(f, "Float({})", f64::from_bits(self.0));
        }

        match self.0 & TYPE_MASK {
            QNAN_UNIT => write!(f, "Unit"),
            QNAN_BOOL => write!(f, "Bool({})", (self.0 & 1) != 0),
            QNAN_INT => write!(f, "Int({})", self.extract_signed_payload()),
            QNAN_INTERNED => write!(f, "InternedString(spur={})", self.0 & PAYLOAD_MASK),
            QNAN_DYNSTR => write!(f, "DynamicString(idx={})", self.0 & PAYLOAD_MASK),
            QNAN_CLOSURE => write!(f, "Closure(idx={})", self.0 & PAYLOAD_MASK),
            _ => write!(f, "Unknown(0x{:016x})", self.0),
        }
    }
}

/// Closure data stored in heap pool.
pub struct ClosureData {
    pub func_idx: u32,
    pub captures: Box<[Cell<Value>]>,
}

/// GC statistics for debugging/profiling.
#[derive(Debug, Clone, Default)]
pub struct GcStats {
    pub collections: u64,
    pub bytes_freed: u64,
    pub strings_freed: u64,
    pub closures_freed: u64,
}

/// Heap for dynamic strings and closures with mark-and-sweep GC.
pub struct Heap {
    // Object storage (None = freed slot)
    strings: Vec<Option<String>>,
    closures: Vec<Option<ClosureData>>,

    // Free lists for slot reuse
    free_strings: Vec<u32>,
    free_closures: Vec<u32>,

    // Mark bits (separate for cache efficiency)
    string_marks: Vec<bool>,
    closure_marks: Vec<bool>,

    // GC state
    bytes_allocated: usize,
    gc_threshold: usize,
    stats: GcStats,
}

/// Initial GC threshold (64KB)
const INITIAL_GC_THRESHOLD: usize = 64 * 1024;
/// GC threshold growth factor
const GC_THRESHOLD_GROWTH: usize = 2;

impl Heap {
    pub fn new() -> Self {
        Self {
            strings: Vec::new(),
            closures: Vec::new(),
            free_strings: Vec::new(),
            free_closures: Vec::new(),
            string_marks: Vec::new(),
            closure_marks: Vec::new(),
            bytes_allocated: 0,
            gc_threshold: INITIAL_GC_THRESHOLD,
            stats: GcStats::default(),
        }
    }

    /// Allocate a string on the heap, returns index.
    pub fn alloc_string(&mut self, s: String) -> u32 {
        let size = s.len();
        let idx = if let Some(free_idx) = self.free_strings.pop() {
            self.strings[free_idx as usize] = Some(s);
            self.string_marks[free_idx as usize] = false;
            free_idx
        } else {
            let idx = self.strings.len() as u32;
            self.strings.push(Some(s));
            self.string_marks.push(false);
            idx
        };
        self.bytes_allocated += size;
        idx
    }

    /// Get string by index. Panics if freed.
    pub fn get_string(&self, idx: u32) -> &str {
        self.strings[idx as usize]
            .as_ref()
            .expect("accessing freed string")
    }

    /// Allocate a closure on the heap, returns index.
    pub fn alloc_closure(&mut self, func_idx: u32, captures: Vec<Value>) -> u32 {
        // Estimate closure size: func_idx (4) + captures (8 * len with NaN boxing) + box overhead (16)
        let size = 4 + 8 * captures.len() + 16;
        let data = ClosureData {
            func_idx,
            captures: captures.into_iter().map(Cell::new).collect(),
        };

        let idx = if let Some(free_idx) = self.free_closures.pop() {
            self.closures[free_idx as usize] = Some(data);
            self.closure_marks[free_idx as usize] = false;
            free_idx
        } else {
            let idx = self.closures.len() as u32;
            self.closures.push(Some(data));
            self.closure_marks.push(false);
            idx
        };
        self.bytes_allocated += size;
        idx
    }

    /// Get closure by index. Panics if freed.
    pub fn get_closure(&self, idx: u32) -> &ClosureData {
        self.closures[idx as usize]
            .as_ref()
            .expect("accessing freed closure")
    }

    /// Check if GC should run based on allocation threshold.
    pub fn should_gc(&self) -> bool {
        self.bytes_allocated > self.gc_threshold
    }

    /// Get current GC statistics.
    pub fn stats(&self) -> &GcStats {
        &self.stats
    }

    /// Get current bytes allocated.
    pub fn bytes_allocated(&self) -> usize {
        self.bytes_allocated
    }

    /// Mark a value as reachable. Returns handles to trace if it's a closure.
    fn mark_value(&mut self, value: Value) -> Option<Vec<Value>> {
        if value.is_dynamic_string() {
            let idx = value.as_dynamic_string_idx().unwrap() as usize;
            if idx < self.string_marks.len() && !self.string_marks[idx] {
                self.string_marks[idx] = true;
            }
            None
        } else if value.is_closure() {
            let idx = value.as_closure_idx().unwrap() as usize;
            if idx < self.closure_marks.len() && !self.closure_marks[idx] {
                self.closure_marks[idx] = true;
                // Return captures to trace
                if let Some(closure) = &self.closures[idx] {
                    return Some(closure.captures.iter().map(|c| c.get()).collect());
                }
            }
            None
        } else {
            None // Non-heap types
        }
    }

    /// Mark phase: trace from roots and mark all reachable objects.
    pub fn mark(&mut self, roots: impl Iterator<Item = Value>) {
        let mut worklist: Vec<Value> = roots.collect();

        while let Some(value) = worklist.pop() {
            if let Some(captures) = self.mark_value(value) {
                worklist.extend(captures);
            }
        }
    }

    /// Sweep phase: free unmarked objects and reset marks.
    pub fn sweep(&mut self) {
        let mut bytes_freed = 0u64;
        let mut strings_freed = 0u64;
        let mut closures_freed = 0u64;

        // Sweep strings
        for (i, marked) in self.string_marks.iter_mut().enumerate() {
            if !*marked && let Some(s) = self.strings[i].take() {
                bytes_freed += s.len() as u64;
                strings_freed += 1;
                self.free_strings.push(i as u32);
            }
            *marked = false; // Reset for next GC
        }

        // Sweep closures
        for (i, marked) in self.closure_marks.iter_mut().enumerate() {
            if !*marked && let Some(closure) = self.closures[i].take() {
                let size = 4 + 8 * closure.captures.len() + 16;
                bytes_freed += size as u64;
                closures_freed += 1;
                self.free_closures.push(i as u32);
            }
            *marked = false;
        }

        self.bytes_allocated = self.bytes_allocated.saturating_sub(bytes_freed as usize);
        self.stats.bytes_freed += bytes_freed;
        self.stats.strings_freed += strings_freed;
        self.stats.closures_freed += closures_freed;
    }

    /// Run a full GC cycle with the given roots.
    pub fn collect(&mut self, roots: impl Iterator<Item = Value>) {
        self.mark(roots);
        self.sweep();
        self.stats.collections += 1;
        // Grow threshold to avoid thrashing
        self.gc_threshold = (self.bytes_allocated * GC_THRESHOLD_GROWTH).max(INITIAL_GC_THRESHOLD);
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
        assert_eq!(std::mem::size_of::<Value>(), 8);
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
    fn test_int_edge_cases() {
        // Test 48-bit signed range: -2^47 to 2^47-1
        let max_48 = (1i64 << 47) - 1; // 140737488355327
        let min_48 = -(1i64 << 47); // -140737488355328

        assert_eq!(Value::int(max_48).as_int(), Some(max_48));
        assert_eq!(Value::int(min_48).as_int(), Some(min_48));
        assert_eq!(Value::int(0).as_int(), Some(0));
        assert_eq!(Value::int(-1).as_int(), Some(-1));
        assert_eq!(Value::int(1).as_int(), Some(1));
    }

    #[test]
    fn test_float_special_values() {
        // Normal floats
        assert_eq!(Value::float(0.0).as_float(), Some(0.0));
        assert_eq!(Value::float(-0.0).as_float(), Some(-0.0));
        assert_eq!(Value::float(1.0).as_float(), Some(1.0));
        assert_eq!(Value::float(-1.0).as_float(), Some(-1.0));

        // Infinity
        assert_eq!(Value::float(f64::INFINITY).as_float(), Some(f64::INFINITY));
        assert_eq!(
            Value::float(f64::NEG_INFINITY).as_float(),
            Some(f64::NEG_INFINITY)
        );

        // NaN (special case - NaN != NaN but we should get a NaN back)
        let nan_val = Value::float(f64::NAN);
        let result = nan_val.as_float();
        assert!(result.is_some());
        assert!(result.unwrap().is_nan());
    }

    #[test]
    fn test_heap_dynamic_string() {
        let mut heap = Heap::new();
        let idx = heap.alloc_string("hello".to_string());
        assert_eq!(heap.get_string(idx), "hello");

        let val = Value::dynamic_string(idx);
        assert_eq!(val.as_dynamic_string_idx(), Some(idx));
        assert_eq!(val.display(&heap), "hello");
    }

    #[test]
    fn test_interned_string() {
        let heap = Heap::new();
        let mut interner = Rodeo::default();
        let spur = interner.get_or_intern("world");

        let val = Value::interned_string(spur);
        assert_eq!(val.as_interned_string(), Some(spur));
        assert_eq!(val.display_with_interner(&heap, &interner), "world");
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
        let mut interner = Rodeo::default();

        // Primitives
        assert!(Value::int(5).eq(&Value::int(5), &heap, &interner));
        assert!(!Value::int(5).eq(&Value::int(6), &heap, &interner));

        // Interned strings (same spur = fast path)
        let spur = interner.get_or_intern("test");
        let s1 = Value::interned_string(spur);
        let s2 = Value::interned_string(spur);
        assert!(s1.eq(&s2, &heap, &interner));

        // Dynamic strings
        let d1 = heap.alloc_string("test".to_string());
        let d2 = heap.alloc_string("test".to_string());
        assert!(Value::dynamic_string(d1).eq(&Value::dynamic_string(d2), &heap, &interner));

        // Interned vs dynamic with same content
        assert!(s1.eq(&Value::dynamic_string(d1), &heap, &interner));
    }

    // GC tests

    #[test]
    fn test_gc_reclaims_unreachable_strings() {
        let mut heap = Heap::new();

        // Allocate some strings
        let idx1 = heap.alloc_string("hello".to_string());
        let idx2 = heap.alloc_string("world".to_string());
        let _idx3 = heap.alloc_string("unreachable".to_string());

        // Only idx1 and idx2 are roots
        let roots = vec![Value::dynamic_string(idx1), Value::dynamic_string(idx2)];
        heap.collect(roots.into_iter());

        // idx1 and idx2 should still be accessible
        assert_eq!(heap.get_string(idx1), "hello");
        assert_eq!(heap.get_string(idx2), "world");

        // Stats should show one string freed
        assert_eq!(heap.stats().strings_freed, 1);
        assert_eq!(heap.stats().collections, 1);
    }

    #[test]
    fn test_gc_reclaims_unreachable_closures() {
        let mut heap = Heap::new();

        // Allocate closures
        let idx1 = heap.alloc_closure(1, vec![Value::int(10)]);
        let _idx2 = heap.alloc_closure(2, vec![Value::int(20)]); // unreachable

        // Only idx1 is a root
        let roots = vec![Value::closure(idx1)];
        heap.collect(roots.into_iter());

        // idx1 should still be accessible
        assert_eq!(heap.get_closure(idx1).func_idx, 1);

        // Stats should show one closure freed
        assert_eq!(heap.stats().closures_freed, 1);
    }

    #[test]
    fn test_gc_traces_closure_captures() {
        let mut heap = Heap::new();

        // Create a string that's only reachable through a closure capture
        let str_idx = heap.alloc_string("captured".to_string());
        let closure_idx = heap.alloc_closure(0, vec![Value::dynamic_string(str_idx)]);

        // Only closure is a direct root, but string should survive via capture
        let roots = vec![Value::closure(closure_idx)];
        heap.collect(roots.into_iter());

        // Both should survive
        assert_eq!(heap.get_string(str_idx), "captured");
        assert_eq!(heap.get_closure(closure_idx).func_idx, 0);
        assert_eq!(heap.stats().strings_freed, 0);
        assert_eq!(heap.stats().closures_freed, 0);
    }

    #[test]
    fn test_gc_reuses_freed_slots() {
        let mut heap = Heap::new();

        // Allocate and free a string
        let idx1 = heap.alloc_string("first".to_string());
        heap.collect(std::iter::empty()); // Free everything

        // Allocate a new string - should reuse slot
        let idx2 = heap.alloc_string("second".to_string());
        assert_eq!(idx1, idx2); // Same index reused
        assert_eq!(heap.get_string(idx2), "second");
    }

    #[test]
    fn test_gc_threshold_grows() {
        let mut heap = Heap::new();
        let initial_threshold = heap.gc_threshold;

        // Allocate enough to exceed threshold (64KB)
        for _ in 0..1000 {
            heap.alloc_string("x".repeat(100));
        }

        // Trigger GC
        heap.collect(std::iter::empty());

        // Threshold should have grown (at minimum stays at initial)
        assert!(heap.gc_threshold >= initial_threshold);
    }

    #[test]
    fn test_gc_bytes_tracking() {
        let mut heap = Heap::new();

        // Allocate strings
        heap.alloc_string("hello".to_string()); // 5 bytes
        heap.alloc_string("world".to_string()); // 5 bytes

        assert!(heap.bytes_allocated() >= 10);

        // Free all
        heap.collect(std::iter::empty());

        // Bytes should be reduced
        assert_eq!(heap.bytes_allocated(), 0);
        assert!(heap.stats().bytes_freed >= 10);
    }
}
