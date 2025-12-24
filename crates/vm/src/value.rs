//! Compact 16-byte runtime value representation.

use std::cell::Cell;

use compile::Spur;
use lasso::{Key, Rodeo};

/// Value tag discriminant
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ValueTag {
    Unit = 0,
    Bool = 1,
    Int = 2,
    Float = 3,
    InternedString = 4,
    DynamicString = 5,
    Closure = 6,
}

/// Compact 16-byte runtime value.
///
/// Heap-allocated types (DynamicString, Closure) store an index into the Heap pools.
/// InternedString stores a Spur key into the module's string interner.
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

    /// Create an interned string value (references module string table)
    #[inline]
    pub fn interned_string(spur: Spur) -> Self {
        Self {
            tag: ValueTag::InternedString,
            _pad: [0; 7],
            bits: spur.into_usize() as u64,
        }
    }

    /// Create a dynamic string value (references heap)
    #[inline]
    pub const fn dynamic_string(idx: u32) -> Self {
        Self {
            tag: ValueTag::DynamicString,
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
    pub fn is_string(&self) -> bool {
        self.tag == ValueTag::InternedString || self.tag == ValueTag::DynamicString
    }

    #[inline]
    pub fn as_interned_string(&self) -> Option<Spur> {
        if self.tag == ValueTag::InternedString {
            // We stored a valid Spur's usize, so this should always succeed
            Spur::try_from_usize(self.bits as usize)
        } else {
            None
        }
    }

    #[inline]
    pub fn as_dynamic_string_idx(&self) -> Option<u32> {
        if self.tag == ValueTag::DynamicString {
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
            ValueTag::InternedString | ValueTag::DynamicString => "String",
            ValueTag::Closure => "Closure",
        }
    }

    /// Display value as string (for results after execution).
    /// Panics if value is an interned string (should be materialized before returning).
    pub fn display(&self, heap: &Heap) -> String {
        match self.tag {
            ValueTag::Unit => "()".to_string(),
            ValueTag::Bool => format!("{}", self.bits != 0),
            ValueTag::Int => format!("{}", self.bits as i64),
            ValueTag::Float => format!("{}", f64::from_bits(self.bits)),
            ValueTag::InternedString => {
                panic!("cannot display interned string without interner - should be materialized")
            }
            ValueTag::DynamicString => heap.get_string(self.bits as u32).to_string(),
            ValueTag::Closure => {
                let closure = heap.get_closure(self.bits as u32);
                format!("<closure fn{}>", closure.func_idx)
            }
        }
    }

    /// Display value with interner (for use during execution with Echo).
    pub fn display_with_interner(&self, heap: &Heap, interner: &Rodeo) -> String {
        match self.tag {
            ValueTag::Unit => "()".to_string(),
            ValueTag::Bool => format!("{}", self.bits != 0),
            ValueTag::Int => format!("{}", self.bits as i64),
            ValueTag::Float => format!("{}", f64::from_bits(self.bits)),
            ValueTag::InternedString => {
                let spur =
                    Spur::try_from_usize(self.bits as usize).expect("invalid interned string spur");
                interner.resolve(&spur).to_string()
            }
            ValueTag::DynamicString => heap.get_string(self.bits as u32).to_string(),
            ValueTag::Closure => {
                let closure = heap.get_closure(self.bits as u32);
                format!("<closure fn{}>", closure.func_idx)
            }
        }
    }

    /// Structural equality, requires heap and interner for string comparison.
    pub fn eq(&self, other: &Value, heap: &Heap, interner: &Rodeo) -> bool {
        match (self.tag, other.tag) {
            (ValueTag::Unit, ValueTag::Unit) => true,
            (ValueTag::Bool, ValueTag::Bool) | (ValueTag::Int, ValueTag::Int) => {
                self.bits == other.bits
            }
            (ValueTag::Float, ValueTag::Float) => {
                let a = f64::from_bits(self.bits);
                let b = f64::from_bits(other.bits);
                a == b
            }
            // Fast path: both interned with same key
            (ValueTag::InternedString, ValueTag::InternedString) if self.bits == other.bits => true,
            // String comparison (any combination of interned/dynamic)
            (
                ValueTag::InternedString | ValueTag::DynamicString,
                ValueTag::InternedString | ValueTag::DynamicString,
            ) => {
                let a = self.get_str(heap, interner);
                let b = other.get_str(heap, interner);
                a == b
            }
            (ValueTag::Closure, ValueTag::Closure) => {
                let a = heap.get_closure(self.bits as u32);
                let b = heap.get_closure(other.bits as u32);
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
                true
            }
            _ => false,
        }
    }

    /// Get string content (works for both interned and dynamic)
    fn get_str<'a>(&self, heap: &'a Heap, interner: &'a Rodeo) -> &'a str {
        match self.tag {
            ValueTag::InternedString => {
                let spur =
                    Spur::try_from_usize(self.bits as usize).expect("invalid interned string spur");
                interner.resolve(&spur)
            }
            ValueTag::DynamicString => heap.get_string(self.bits as u32),
            _ => panic!("not a string"),
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
            ValueTag::InternedString => write!(f, "InternedString(spur={})", self.bits),
            ValueTag::DynamicString => write!(f, "DynamicString(idx={})", self.bits),
            ValueTag::Closure => write!(f, "Closure(idx={})", self.bits),
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
        // Estimate closure size: func_idx (4) + captures (16 * len) + box overhead (16)
        let size = 4 + 16 * captures.len() + 16;
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
        match value.tag() {
            ValueTag::DynamicString => {
                let idx = value.as_dynamic_string_idx().unwrap() as usize;
                if idx < self.string_marks.len() && !self.string_marks[idx] {
                    self.string_marks[idx] = true;
                }
                None
            }
            ValueTag::Closure => {
                let idx = value.as_closure_idx().unwrap() as usize;
                if idx < self.closure_marks.len() && !self.closure_marks[idx] {
                    self.closure_marks[idx] = true;
                    // Return captures to trace
                    if let Some(closure) = &self.closures[idx] {
                        return Some(closure.captures.iter().map(|c| c.get()).collect());
                    }
                }
                None
            }
            _ => None, // Non-heap types
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
                let size = 4 + 16 * closure.captures.len() + 16;
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
