//! NaN-boxed 8-byte runtime value representation.
//!
//! Uses IEEE 754 quiet NaN encoding to pack all value types into 64 bits:
//! - Floats: raw IEEE 754 double
//! - Other types: quiet NaN + 3-bit tag + 48-bit payload

use std::cell::Cell;
use std::collections::HashMap;

use compile::Spur;
use lasso::{Key, Rodeo};

// NaN boxing constants
// Use negative quiet NaN (sign=1, exp=0x7FF, quiet=1) to avoid collisions with real NaNs
// Bits 48-50 are free for tags, bits 0-47 for payload
const QNAN: u64 = 0xFFF8_0000_0000_0000;
const TAG_MASK: u64 = 0x0007_0000_0000_0000; // 3 bits for type tag (bits 48-50)
const PAYLOAD_MASK: u64 = 0x0000_FFFF_FFFF_FFFF; // 48-bit payload

// Tags (bits 48-50) - OR'd with QNAN
// New layout with payload sub-tags for heap types:
//   001 = Integer   (full 48-bit signed payload)
//   010 = Boolean   (1-bit payload)
//   011 = Unit      (no payload)
//   100 = String    (1-bit subtype + 47-bit payload)
//   101 = Closure   (48-bit heap index)
//   110 = Aggregate (2-bit subtype + 46-bit heap index)
//   111 = Object    (reserved for Struct/Enum)
const TAG_INT: u64 = 0x0001_0000_0000_0000; // 001
const TAG_BOOL: u64 = 0x0002_0000_0000_0000; // 010
const TAG_UNIT: u64 = 0x0003_0000_0000_0000; // 011
const TAG_STRING: u64 = 0x0004_0000_0000_0000; // 100
const TAG_CLOSURE: u64 = 0x0005_0000_0000_0000; // 101
const TAG_AGGREGATE: u64 = 0x0006_0000_0000_0000; // 110
#[allow(dead_code)]
const TAG_OBJECT: u64 = 0x0007_0000_0000_0000; // 111 (reserved)

// String subtypes (bit 47 of payload)
const STRING_INTERNED: u64 = 0x0000_0000_0000_0000; // bit 47 = 0
const STRING_DYNAMIC: u64 = 0x0000_8000_0000_0000; // bit 47 = 1
const STRING_SUBTYPE_MASK: u64 = 0x0000_8000_0000_0000;
const STRING_PAYLOAD_MASK: u64 = 0x0000_7FFF_FFFF_FFFF; // 47 bits

// Aggregate subtypes (bits 46-47 of payload)
const AGG_LIST: u64 = 0x0000_0000_0000_0000; // bits 46-47 = 00
const AGG_TUPLE: u64 = 0x0000_4000_0000_0000; // bits 46-47 = 01
const AGG_SUBTYPE_MASK: u64 = 0x0000_C000_0000_0000;
const AGG_PAYLOAD_MASK: u64 = 0x0000_3FFF_FFFF_FFFF; // 46 bits

// Object subtypes (bits 46-47 of payload)
const OBJ_STRUCT: u64 = 0x0000_0000_0000_0000; // bits 46-47 = 00
const OBJ_SUBTYPE_MASK: u64 = 0x0000_C000_0000_0000;
const OBJ_PAYLOAD_MASK: u64 = 0x0000_3FFF_FFFF_FFFF; // 46 bits

// Combined patterns for fast checking
const QNAN_INT: u64 = QNAN | TAG_INT;
const QNAN_BOOL: u64 = QNAN | TAG_BOOL;
const QNAN_UNIT: u64 = QNAN | TAG_UNIT;
const QNAN_STRING: u64 = QNAN | TAG_STRING;
const QNAN_INTERNED: u64 = QNAN | TAG_STRING | STRING_INTERNED;
const QNAN_DYNSTR: u64 = QNAN | TAG_STRING | STRING_DYNAMIC;
const QNAN_CLOSURE: u64 = QNAN | TAG_CLOSURE;
const QNAN_AGGREGATE: u64 = QNAN | TAG_AGGREGATE;
const QNAN_LIST: u64 = QNAN | TAG_AGGREGATE | AGG_LIST;
const QNAN_TUPLE: u64 = QNAN | TAG_AGGREGATE | AGG_TUPLE;
const QNAN_OBJECT: u64 = QNAN | TAG_OBJECT;
const QNAN_STRUCT: u64 = QNAN | TAG_OBJECT | OBJ_STRUCT;

// Mask for type checking: QNAN + TAG
const TYPE_MASK: u64 = QNAN | TAG_MASK;

/// NaN-boxed 8-byte runtime value.
///
/// Encoding:
/// - Float: raw IEEE 754 double (not a quiet NaN)
/// - Int: QNAN | TAG_INT | 48-bit signed payload
/// - Bool: QNAN | TAG_BOOL | 0 or 1
/// - Unit: QNAN | TAG_UNIT
/// - InternedString: QNAN | TAG_STRING | STRING_INTERNED | 47-bit spur index
/// - DynamicString: QNAN | TAG_STRING | STRING_DYNAMIC | 47-bit heap index
/// - Closure: QNAN | TAG_CLOSURE | 48-bit heap index
/// - List: QNAN | TAG_AGGREGATE | AGG_LIST | 46-bit heap index
/// - Tuple: QNAN | TAG_AGGREGATE | AGG_TUPLE | 46-bit heap index
/// - Struct: QNAN | TAG_OBJECT | OBJ_STRUCT | 46-bit heap index
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

    #[inline(always)]
    pub const fn list(idx: u32) -> Self {
        Self(QNAN_LIST | (idx as u64))
    }

    #[inline(always)]
    pub const fn tuple(idx: u32) -> Self {
        Self(QNAN_TUPLE | (idx as u64))
    }

    #[inline(always)]
    pub const fn struct_obj(idx: u32) -> Self {
        Self(QNAN_STRUCT | (idx as u64))
    }

    /// Get raw bits for passing to/from JIT (NaN-boxed representation).
    #[inline(always)]
    pub const fn to_bits(self) -> i64 {
        self.0 as i64
    }

    /// Reconstruct Value from JIT bits (NaN-boxed representation).
    #[inline(always)]
    pub const fn from_bits(raw: i64) -> Self {
        Self(raw as u64)
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

    /// Check if this is any string type
    #[inline(always)]
    pub fn is_string(&self) -> bool {
        (self.0 & TYPE_MASK) == QNAN_STRING
    }

    /// Check if this is an interned string
    #[inline(always)]
    pub fn is_interned_string(&self) -> bool {
        self.is_string() && (self.0 & STRING_SUBTYPE_MASK) == STRING_INTERNED
    }

    /// Check if this is a dynamic string
    #[inline(always)]
    pub fn is_dynamic_string(&self) -> bool {
        self.is_string() && (self.0 & STRING_SUBTYPE_MASK) == STRING_DYNAMIC
    }

    /// Check if this is a closure
    #[inline(always)]
    pub fn is_closure(&self) -> bool {
        (self.0 & TYPE_MASK) == QNAN_CLOSURE
    }

    /// Check if this is any aggregate type (list, tuple)
    #[inline(always)]
    pub fn is_aggregate(&self) -> bool {
        (self.0 & TYPE_MASK) == QNAN_AGGREGATE
    }

    /// Check if this is a list
    #[inline(always)]
    pub fn is_list(&self) -> bool {
        self.is_aggregate() && (self.0 & AGG_SUBTYPE_MASK) == AGG_LIST
    }

    /// Check if this is a tuple
    #[inline(always)]
    pub fn is_tuple(&self) -> bool {
        self.is_aggregate() && (self.0 & AGG_SUBTYPE_MASK) == AGG_TUPLE
    }

    /// Check if this is any object type (struct, enum)
    #[inline(always)]
    pub fn is_object(&self) -> bool {
        (self.0 & TYPE_MASK) == QNAN_OBJECT
    }

    /// Check if this is a struct
    #[inline(always)]
    pub fn is_struct(&self) -> bool {
        self.is_object() && (self.0 & OBJ_SUBTYPE_MASK) == OBJ_STRUCT
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
            let payload = (self.0 & STRING_PAYLOAD_MASK) as usize;
            Spur::try_from_usize(payload)
        } else {
            None
        }
    }

    #[inline(always)]
    pub fn as_dynamic_string_idx(&self) -> Option<u32> {
        if self.is_dynamic_string() {
            Some((self.0 & STRING_PAYLOAD_MASK) as u32)
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

    #[inline(always)]
    pub fn as_list_idx(&self) -> Option<u32> {
        if self.is_list() {
            Some((self.0 & AGG_PAYLOAD_MASK) as u32)
        } else {
            None
        }
    }

    /// Get list index. Panics in debug if wrong type.
    #[inline(always)]
    pub fn as_list_idx_unchecked(&self) -> u32 {
        debug_assert!(self.is_list(), "expected List");
        (self.0 & AGG_PAYLOAD_MASK) as u32
    }

    #[inline(always)]
    pub fn as_tuple_idx(&self) -> Option<u32> {
        if self.is_tuple() {
            Some((self.0 & AGG_PAYLOAD_MASK) as u32)
        } else {
            None
        }
    }

    /// Get tuple index. Panics in debug if wrong type.
    #[inline(always)]
    pub fn as_tuple_idx_unchecked(&self) -> u32 {
        debug_assert!(self.is_tuple(), "expected Tuple");
        (self.0 & AGG_PAYLOAD_MASK) as u32
    }

    #[inline(always)]
    pub fn as_struct_idx(&self) -> Option<u32> {
        if self.is_struct() {
            Some((self.0 & OBJ_PAYLOAD_MASK) as u32)
        } else {
            None
        }
    }

    /// Get struct index. Panics in debug if wrong type.
    #[inline(always)]
    pub fn as_struct_idx_unchecked(&self) -> u32 {
        debug_assert!(self.is_struct(), "expected Struct");
        (self.0 & OBJ_PAYLOAD_MASK) as u32
    }

    pub fn type_name(&self) -> &'static str {
        if self.is_float() {
            "Float"
        } else {
            match self.0 & TYPE_MASK {
                QNAN_INT => "Int",
                QNAN_BOOL => "Bool",
                QNAN_UNIT => "Unit",
                QNAN_STRING => "String",
                QNAN_CLOSURE => "Closure",
                QNAN_AGGREGATE => match self.0 & AGG_SUBTYPE_MASK {
                    AGG_LIST => "List",
                    AGG_TUPLE => "Tuple",
                    _ => "Unknown",
                },
                QNAN_OBJECT => match self.0 & OBJ_SUBTYPE_MASK {
                    OBJ_STRUCT => "Struct",
                    _ => "Unknown",
                },
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
            QNAN_STRING => {
                if self.is_interned_string() {
                    panic!(
                        "cannot display interned string without interner - should be materialized"
                    )
                } else {
                    let idx = (self.0 & STRING_PAYLOAD_MASK) as u32;
                    heap.get_string(idx).to_string()
                }
            }
            QNAN_CLOSURE => {
                let idx = (self.0 & PAYLOAD_MASK) as u32;
                let closure = heap.get_closure(idx);
                format!("<closure fn{}>", closure.func_idx)
            }
            QNAN_AGGREGATE => match self.0 & AGG_SUBTYPE_MASK {
                AGG_LIST => {
                    let idx = (self.0 & AGG_PAYLOAD_MASK) as u32;
                    let list = heap.get_list(idx);
                    let elements: Vec<String> =
                        list.elements.iter().map(|v| v.display(heap)).collect();
                    format!("[{}]", elements.join(", "))
                }
                AGG_TUPLE => {
                    let idx = (self.0 & AGG_PAYLOAD_MASK) as u32;
                    let tuple = heap.get_tuple(idx);
                    let elements: Vec<String> =
                        tuple.elements.iter().map(|v| v.display(heap)).collect();
                    format!("({})", elements.join(", "))
                }
                _ => "<unknown>".to_string(),
            },
            QNAN_OBJECT => match self.0 & OBJ_SUBTYPE_MASK {
                OBJ_STRUCT => {
                    let idx = (self.0 & OBJ_PAYLOAD_MASK) as u32;
                    let s = heap.get_struct(idx);
                    if let Some(meta) = heap.get_struct_meta(s.struct_id) {
                        let fields: Vec<String> = s
                            .fields
                            .iter()
                            .zip(&meta.field_names)
                            .map(|(v, name)| format!("{}: {}", name, v.display(heap)))
                            .collect();
                        format!("{} {{ {} }}", meta.name, fields.join(", "))
                    } else {
                        let fields: Vec<String> =
                            s.fields.iter().map(|v| v.display(heap)).collect();
                        format!("#{} {{ {} }}", s.struct_id, fields.join(", "))
                    }
                }
                _ => "<unknown>".to_string(),
            },
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
            QNAN_STRING => {
                if self.is_interned_string() {
                    let payload = (self.0 & STRING_PAYLOAD_MASK) as usize;
                    let spur = Spur::try_from_usize(payload).expect("invalid interned string spur");
                    interner.resolve(&spur).to_string()
                } else {
                    let idx = (self.0 & STRING_PAYLOAD_MASK) as u32;
                    heap.get_string(idx).to_string()
                }
            }
            QNAN_CLOSURE => {
                let idx = (self.0 & PAYLOAD_MASK) as u32;
                let closure = heap.get_closure(idx);
                format!("<closure fn{}>", closure.func_idx)
            }
            QNAN_AGGREGATE => match self.0 & AGG_SUBTYPE_MASK {
                AGG_LIST => {
                    let idx = (self.0 & AGG_PAYLOAD_MASK) as u32;
                    let list = heap.get_list(idx);
                    let elements: Vec<String> = list
                        .elements
                        .iter()
                        .map(|v| v.display_with_interner(heap, interner))
                        .collect();
                    format!("[{}]", elements.join(", "))
                }
                AGG_TUPLE => {
                    let idx = (self.0 & AGG_PAYLOAD_MASK) as u32;
                    let tuple = heap.get_tuple(idx);
                    let elements: Vec<String> = tuple
                        .elements
                        .iter()
                        .map(|v| v.display_with_interner(heap, interner))
                        .collect();
                    format!("({})", elements.join(", "))
                }
                _ => "<unknown>".to_string(),
            },
            QNAN_OBJECT => match self.0 & OBJ_SUBTYPE_MASK {
                OBJ_STRUCT => {
                    let idx = (self.0 & OBJ_PAYLOAD_MASK) as u32;
                    let s = heap.get_struct(idx);
                    if let Some(meta) = heap.get_struct_meta(s.struct_id) {
                        let fields: Vec<String> = s
                            .fields
                            .iter()
                            .zip(&meta.field_names)
                            .map(|(v, name)| {
                                format!("{}: {}", name, v.display_with_interner(heap, interner))
                            })
                            .collect();
                        format!("{} {{ {} }}", meta.name, fields.join(", "))
                    } else {
                        let fields: Vec<String> = s
                            .fields
                            .iter()
                            .map(|v| v.display_with_interner(heap, interner))
                            .collect();
                        format!("#{} {{ {} }}", s.struct_id, fields.join(", "))
                    }
                }
                _ => "<unknown>".to_string(),
            },
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
        if self_type == QNAN_STRING && other_type == QNAN_STRING {
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

        // Aggregate comparison (list, tuple)
        if self_type == QNAN_AGGREGATE && other_type == QNAN_AGGREGATE {
            let self_subtype = self.0 & AGG_SUBTYPE_MASK;
            let other_subtype = other.0 & AGG_SUBTYPE_MASK;
            if self_subtype != other_subtype {
                return false;
            }
            match self_subtype {
                AGG_LIST => {
                    let a_idx = (self.0 & AGG_PAYLOAD_MASK) as u32;
                    let b_idx = (other.0 & AGG_PAYLOAD_MASK) as u32;
                    let a = heap.get_list(a_idx);
                    let b = heap.get_list(b_idx);
                    if a.elements.len() != b.elements.len() {
                        return false;
                    }
                    for (ea, eb) in a.elements.iter().zip(b.elements.iter()) {
                        if !ea.eq(eb, heap, interner) {
                            return false;
                        }
                    }
                    return true;
                }
                AGG_TUPLE => {
                    let a_idx = (self.0 & AGG_PAYLOAD_MASK) as u32;
                    let b_idx = (other.0 & AGG_PAYLOAD_MASK) as u32;
                    let a = heap.get_tuple(a_idx);
                    let b = heap.get_tuple(b_idx);
                    if a.elements.len() != b.elements.len() {
                        return false;
                    }
                    for (ea, eb) in a.elements.iter().zip(b.elements.iter()) {
                        if !ea.eq(eb, heap, interner) {
                            return false;
                        }
                    }
                    return true;
                }
                _ => return false,
            }
        }

        false
    }

    /// Get string content (works for both interned and dynamic)
    fn get_str<'a>(&self, heap: &'a Heap, interner: &'a Rodeo) -> &'a str {
        debug_assert!(self.is_string(), "expected String");
        if self.is_interned_string() {
            let payload = (self.0 & STRING_PAYLOAD_MASK) as usize;
            let spur = Spur::try_from_usize(payload).expect("invalid interned string spur");
            interner.resolve(&spur)
        } else {
            let idx = (self.0 & STRING_PAYLOAD_MASK) as u32;
            heap.get_string(idx)
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
            QNAN_STRING => {
                if self.is_interned_string() {
                    write!(f, "InternedString(spur={})", self.0 & STRING_PAYLOAD_MASK)
                } else {
                    write!(f, "DynamicString(idx={})", self.0 & STRING_PAYLOAD_MASK)
                }
            }
            QNAN_CLOSURE => write!(f, "Closure(idx={})", self.0 & PAYLOAD_MASK),
            QNAN_AGGREGATE => match self.0 & AGG_SUBTYPE_MASK {
                AGG_LIST => write!(f, "List(idx={})", self.0 & AGG_PAYLOAD_MASK),
                AGG_TUPLE => write!(f, "Tuple(idx={})", self.0 & AGG_PAYLOAD_MASK),
                _ => write!(f, "Aggregate(unknown=0x{:016x})", self.0),
            },
            QNAN_OBJECT => match self.0 & OBJ_SUBTYPE_MASK {
                OBJ_STRUCT => write!(f, "Struct(idx={})", self.0 & OBJ_PAYLOAD_MASK),
                _ => write!(f, "Object(unknown=0x{:016x})", self.0),
            },
            _ => write!(f, "Unknown(0x{:016x})", self.0),
        }
    }
}

/// Closure data stored in heap pool.
pub struct ClosureData {
    pub func_idx: u32,
    pub captures: Box<[Cell<Value>]>,
}

/// List data stored in heap pool.
pub struct ListData {
    pub elements: Vec<Value>,
}

/// Tuple data stored in heap pool.
pub struct TupleData {
    pub elements: Box<[Value]>, // fixed after creation
}

/// Struct data stored in heap pool.
pub struct StructData {
    pub struct_id: u32,       // identifies the struct type
    pub fields: Box<[Value]>, // fixed-size fields
}

/// Struct type metadata for display purposes.
#[derive(Clone)]
pub struct StructMeta {
    pub name: String,
    pub field_names: Vec<String>,
}

/// GC statistics for debugging/profiling.
#[derive(Debug, Clone, Default)]
pub struct GcStats {
    pub collections: u64,
    pub bytes_freed: u64,
    pub strings_freed: u64,
    pub closures_freed: u64,
    pub lists_freed: u64,
    pub tuples_freed: u64,
    pub structs_freed: u64,
}

/// Heap for dynamic strings, closures, lists, tuples, and structs with mark-and-sweep GC.
pub struct Heap {
    // Object storage (None = freed slot)
    strings: Vec<Option<String>>,
    closures: Vec<Option<ClosureData>>,
    lists: Vec<Option<ListData>>,
    tuples: Vec<Option<TupleData>>,
    structs: Vec<Option<StructData>>,

    // Struct type metadata (struct_id -> metadata)
    struct_meta: HashMap<u32, StructMeta>,

    // Free lists for slot reuse
    free_strings: Vec<u32>,
    free_closures: Vec<u32>,
    free_lists: Vec<u32>,
    free_tuples: Vec<u32>,
    free_structs: Vec<u32>,

    // Mark bits (separate for cache efficiency)
    string_marks: Vec<bool>,
    closure_marks: Vec<bool>,
    list_marks: Vec<bool>,
    tuple_marks: Vec<bool>,
    struct_marks: Vec<bool>,

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
            lists: Vec::new(),
            tuples: Vec::new(),
            structs: Vec::new(),
            struct_meta: HashMap::new(),
            free_strings: Vec::new(),
            free_closures: Vec::new(),
            free_lists: Vec::new(),
            free_tuples: Vec::new(),
            free_structs: Vec::new(),
            string_marks: Vec::new(),
            closure_marks: Vec::new(),
            list_marks: Vec::new(),
            tuple_marks: Vec::new(),
            struct_marks: Vec::new(),
            bytes_allocated: 0,
            gc_threshold: INITIAL_GC_THRESHOLD,
            stats: GcStats::default(),
        }
    }

    /// Register struct type metadata for display purposes.
    pub fn register_struct_meta(&mut self, struct_id: u32, name: String, field_names: Vec<String>) {
        self.struct_meta
            .insert(struct_id, StructMeta { name, field_names });
    }

    /// Get struct metadata by struct_id.
    pub fn get_struct_meta(&self, struct_id: u32) -> Option<&StructMeta> {
        self.struct_meta.get(&struct_id)
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

    /// Allocate a list on the heap, returns index.
    /// Pre-fills with unit values for literal initialization via ListSet.
    pub fn alloc_list(&mut self, capacity: usize) -> u32 {
        // Estimate list size: Vec overhead (24) + elements (8 * capacity with NaN boxing)
        let size = 24 + 8 * capacity;
        let data = ListData {
            elements: vec![Value::unit(); capacity],
        };

        let idx = if let Some(free_idx) = self.free_lists.pop() {
            self.lists[free_idx as usize] = Some(data);
            self.list_marks[free_idx as usize] = false;
            free_idx
        } else {
            let idx = self.lists.len() as u32;
            self.lists.push(Some(data));
            self.list_marks.push(false);
            idx
        };
        self.bytes_allocated += size;
        idx
    }

    /// Get list by index. Panics if freed.
    pub fn get_list(&self, idx: u32) -> &ListData {
        self.lists[idx as usize]
            .as_ref()
            .expect("accessing freed list")
    }

    /// Get mutable list by index. Panics if freed.
    pub fn get_list_mut(&mut self, idx: u32) -> &mut ListData {
        self.lists[idx as usize]
            .as_mut()
            .expect("accessing freed list")
    }

    /// Allocate a tuple on the heap, returns index.
    pub fn alloc_tuple(&mut self, elements: Vec<Value>) -> u32 {
        // Estimate tuple size: Box overhead (16) + elements (8 * len with NaN boxing)
        let size = 16 + 8 * elements.len();
        let data = TupleData {
            elements: elements.into_boxed_slice(),
        };

        let idx = if let Some(free_idx) = self.free_tuples.pop() {
            self.tuples[free_idx as usize] = Some(data);
            self.tuple_marks[free_idx as usize] = false;
            free_idx
        } else {
            let idx = self.tuples.len() as u32;
            self.tuples.push(Some(data));
            self.tuple_marks.push(false);
            idx
        };
        self.bytes_allocated += size;
        idx
    }

    /// Get tuple by index. Panics if freed.
    pub fn get_tuple(&self, idx: u32) -> &TupleData {
        self.tuples[idx as usize]
            .as_ref()
            .expect("accessing freed tuple")
    }

    /// Allocate a struct on the heap, returns index.
    pub fn alloc_struct(&mut self, struct_id: u32, fields: Vec<Value>) -> u32 {
        // Estimate struct size: struct_id (4) + Box overhead (16) + fields (8 * len)
        let size = 4 + 16 + 8 * fields.len();
        let data = StructData {
            struct_id,
            fields: fields.into_boxed_slice(),
        };

        let idx = if let Some(free_idx) = self.free_structs.pop() {
            self.structs[free_idx as usize] = Some(data);
            self.struct_marks[free_idx as usize] = false;
            free_idx
        } else {
            let idx = self.structs.len() as u32;
            self.structs.push(Some(data));
            self.struct_marks.push(false);
            idx
        };
        self.bytes_allocated += size;
        idx
    }

    /// Get struct by index. Panics if freed.
    pub fn get_struct(&self, idx: u32) -> &StructData {
        self.structs[idx as usize]
            .as_ref()
            .expect("accessing freed struct")
    }

    /// Get mutable struct by index. Panics if freed.
    pub fn get_struct_mut(&mut self, idx: u32) -> &mut StructData {
        self.structs[idx as usize]
            .as_mut()
            .expect("accessing freed struct")
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

    /// Mark a value as reachable. Returns handles to trace if it's a closure, list, or tuple.
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
        } else if value.is_list() {
            let idx = value.as_list_idx().unwrap() as usize;
            if idx < self.list_marks.len() && !self.list_marks[idx] {
                self.list_marks[idx] = true;
                // Return elements to trace
                if let Some(list) = &self.lists[idx] {
                    return Some(list.elements.clone());
                }
            }
            None
        } else if value.is_tuple() {
            let idx = value.as_tuple_idx().unwrap() as usize;
            if idx < self.tuple_marks.len() && !self.tuple_marks[idx] {
                self.tuple_marks[idx] = true;
                // Return elements to trace
                if let Some(tuple) = &self.tuples[idx] {
                    return Some(tuple.elements.to_vec());
                }
            }
            None
        } else if value.is_struct() {
            let idx = value.as_struct_idx().unwrap() as usize;
            if idx < self.struct_marks.len() && !self.struct_marks[idx] {
                self.struct_marks[idx] = true;
                // Return fields to trace
                if let Some(s) = &self.structs[idx] {
                    return Some(s.fields.to_vec());
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
        let mut lists_freed = 0u64;
        let mut tuples_freed = 0u64;
        let mut structs_freed = 0u64;

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

        // Sweep lists
        for (i, marked) in self.list_marks.iter_mut().enumerate() {
            if !*marked && let Some(list) = self.lists[i].take() {
                let size = 24 + 8 * list.elements.len();
                bytes_freed += size as u64;
                lists_freed += 1;
                self.free_lists.push(i as u32);
            }
            *marked = false;
        }

        // Sweep tuples
        for (i, marked) in self.tuple_marks.iter_mut().enumerate() {
            if !*marked && let Some(tuple) = self.tuples[i].take() {
                let size = 16 + 8 * tuple.elements.len();
                bytes_freed += size as u64;
                tuples_freed += 1;
                self.free_tuples.push(i as u32);
            }
            *marked = false;
        }

        // Sweep structs
        for (i, marked) in self.struct_marks.iter_mut().enumerate() {
            if !*marked && let Some(s) = self.structs[i].take() {
                let size = 4 + 16 + 8 * s.fields.len();
                bytes_freed += size as u64;
                structs_freed += 1;
                self.free_structs.push(i as u32);
            }
            *marked = false;
        }

        self.bytes_allocated = self.bytes_allocated.saturating_sub(bytes_freed as usize);
        self.stats.bytes_freed += bytes_freed;
        self.stats.strings_freed += strings_freed;
        self.stats.closures_freed += closures_freed;
        self.stats.lists_freed += lists_freed;
        self.stats.tuples_freed += tuples_freed;
        self.stats.structs_freed += structs_freed;
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

    #[test]
    fn test_list_basic() {
        let mut heap = Heap::new();

        let idx = heap.alloc_list(3);
        let list = heap.get_list_mut(idx);
        list.elements[0] = Value::int(1);
        list.elements[1] = Value::int(2);
        list.elements[2] = Value::int(3);

        let val = Value::list(idx);
        assert!(val.is_list());
        assert_eq!(val.as_list_idx(), Some(idx));
        assert_eq!(val.display(&heap), "[1, 2, 3]");
    }

    #[test]
    fn test_gc_traces_list_elements() {
        let mut heap = Heap::new();

        // Create a string that's only reachable through a list element
        let str_idx = heap.alloc_string("in list".to_string());
        let list_idx = heap.alloc_list(1);
        heap.get_list_mut(list_idx).elements[0] = Value::dynamic_string(str_idx);

        // Only list is a direct root, but string should survive via element
        let roots = vec![Value::list(list_idx)];
        heap.collect(roots.into_iter());

        // Both should survive
        assert_eq!(heap.get_string(str_idx), "in list");
        assert_eq!(heap.stats().strings_freed, 0);
        assert_eq!(heap.stats().lists_freed, 0);
    }

    #[test]
    fn test_tuple_basic() {
        let mut heap = Heap::new();

        let idx = heap.alloc_tuple(vec![Value::int(1), Value::int(2), Value::int(3)]);

        let val = Value::tuple(idx);
        assert!(val.is_tuple());
        assert!(val.is_aggregate());
        assert!(!val.is_list());
        assert_eq!(val.as_tuple_idx(), Some(idx));
        assert_eq!(val.display(&heap), "(1, 2, 3)");
    }

    #[test]
    fn test_tuple_heterogeneous() {
        let mut heap = Heap::new();
        let interner = Rodeo::default();

        let str_idx = heap.alloc_string("hello".to_string());
        let idx = heap.alloc_tuple(vec![
            Value::int(42),
            Value::bool(true),
            Value::dynamic_string(str_idx),
        ]);

        let val = Value::tuple(idx);
        assert_eq!(
            val.display_with_interner(&heap, &interner),
            "(42, true, hello)"
        );
    }

    #[test]
    fn test_gc_traces_tuple_elements() {
        let mut heap = Heap::new();

        // Create a string that's only reachable through a tuple element
        let str_idx = heap.alloc_string("in tuple".to_string());
        let tuple_idx = heap.alloc_tuple(vec![Value::dynamic_string(str_idx), Value::int(42)]);

        // Only tuple is a direct root, but string should survive via element
        let roots = vec![Value::tuple(tuple_idx)];
        heap.collect(roots.into_iter());

        // Both should survive
        assert_eq!(heap.get_string(str_idx), "in tuple");
        assert_eq!(heap.stats().strings_freed, 0);
        assert_eq!(heap.stats().tuples_freed, 0);
    }

    #[test]
    fn test_gc_reclaims_unreachable_tuples() {
        let mut heap = Heap::new();

        // Allocate tuples
        let idx1 = heap.alloc_tuple(vec![Value::int(1), Value::int(2)]);
        let _idx2 = heap.alloc_tuple(vec![Value::int(3), Value::int(4)]); // unreachable

        // Only idx1 is a root
        let roots = vec![Value::tuple(idx1)];
        heap.collect(roots.into_iter());

        // idx1 should still be accessible
        assert_eq!(heap.get_tuple(idx1).elements.len(), 2);

        // Stats should show one tuple freed
        assert_eq!(heap.stats().tuples_freed, 1);
    }

    #[test]
    fn test_tuple_equality() {
        let mut heap = Heap::new();
        let interner = Rodeo::default();

        let t1 = heap.alloc_tuple(vec![Value::int(1), Value::int(2)]);
        let t2 = heap.alloc_tuple(vec![Value::int(1), Value::int(2)]);
        let t3 = heap.alloc_tuple(vec![Value::int(1), Value::int(3)]);

        assert!(Value::tuple(t1).eq(&Value::tuple(t2), &heap, &interner));
        assert!(!Value::tuple(t1).eq(&Value::tuple(t3), &heap, &interner));
    }
}
