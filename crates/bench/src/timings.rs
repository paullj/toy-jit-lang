//! Pipeline timing utilities.

use serde::{Deserialize, Serialize};
use std::ops::Add;
use std::time::Duration;

/// Timing measurements for each pipeline stage.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PipelineTimings {
    pub lex: Duration,
    pub parse: Duration,
    pub hir: Duration,
    pub infer: Duration,
    pub mir: Duration,
    pub compile: Duration,
    pub vm_exec: Option<Duration>,
    pub jit_warmup: Option<Duration>,
    pub jit_compile: Option<Duration>,
    pub jit_exec: Option<Duration>,
}

impl PipelineTimings {
    /// Total time spent in frontend stages (lex + parse + hir + infer).
    pub fn total_frontend(&self) -> Duration {
        self.lex + self.parse + self.hir + self.infer
    }

    /// Total time spent in backend stages (mir + compile).
    pub fn total_backend(&self) -> Duration {
        self.mir + self.compile
    }

    /// Total time for VM execution path.
    pub fn total_vm(&self) -> Duration {
        self.total_frontend() + self.total_backend() + self.vm_exec.unwrap_or_default()
    }

    /// Total time for JIT execution path.
    pub fn total_jit(&self) -> Duration {
        self.total_frontend()
            + self.mir
            + self.jit_compile.unwrap_or_default()
            + self.jit_exec.unwrap_or_default()
    }

    /// Total time for JIT including warmup.
    pub fn total_jit_with_warmup(&self) -> Duration {
        self.total_frontend()
            + self.mir
            + self.jit_warmup.unwrap_or_default()
            + self.jit_compile.unwrap_or_default()
            + self.jit_exec.unwrap_or_default()
    }

    /// Element-wise addition with another PipelineTimings.
    pub fn add(&self, other: &PipelineTimings) -> PipelineTimings {
        PipelineTimings {
            lex: self.lex + other.lex,
            parse: self.parse + other.parse,
            hir: self.hir + other.hir,
            infer: self.infer + other.infer,
            mir: self.mir + other.mir,
            compile: self.compile + other.compile,
            vm_exec: match (self.vm_exec, other.vm_exec) {
                (Some(a), Some(b)) => Some(a + b),
                (Some(a), None) | (None, Some(a)) => Some(a),
                (None, None) => None,
            },
            jit_warmup: match (self.jit_warmup, other.jit_warmup) {
                (Some(a), Some(b)) => Some(a + b),
                (Some(a), None) | (None, Some(a)) => Some(a),
                (None, None) => None,
            },
            jit_compile: match (self.jit_compile, other.jit_compile) {
                (Some(a), Some(b)) => Some(a + b),
                (Some(a), None) | (None, Some(a)) => Some(a),
                (None, None) => None,
            },
            jit_exec: match (self.jit_exec, other.jit_exec) {
                (Some(a), Some(b)) => Some(a + b),
                (Some(a), None) | (None, Some(a)) => Some(a),
                (None, None) => None,
            },
        }
    }

    /// Divide all durations by a scalar.
    pub fn div(&self, n: usize) -> PipelineTimings {
        let n = n as u32;
        PipelineTimings {
            lex: self.lex / n,
            parse: self.parse / n,
            hir: self.hir / n,
            infer: self.infer / n,
            mir: self.mir / n,
            compile: self.compile / n,
            vm_exec: self.vm_exec.map(|d| d / n),
            jit_warmup: self.jit_warmup.map(|d| d / n),
            jit_compile: self.jit_compile.map(|d| d / n),
            jit_exec: self.jit_exec.map(|d| d / n),
        }
    }

    /// Format timings for display.
    pub fn display(&self) -> String {
        let mut s = String::new();
        s.push_str(&format!(
            "  lex:         {:>12}\n",
            format_duration(self.lex)
        ));
        s.push_str(&format!(
            "  parse:       {:>12}\n",
            format_duration(self.parse)
        ));
        s.push_str(&format!(
            "  hir:         {:>12}\n",
            format_duration(self.hir)
        ));
        s.push_str(&format!(
            "  infer:       {:>12}\n",
            format_duration(self.infer)
        ));
        s.push_str(&format!(
            "  mir:         {:>12}\n",
            format_duration(self.mir)
        ));
        s.push_str(&format!(
            "  compile:     {:>12}\n",
            format_duration(self.compile)
        ));
        if let Some(t) = self.vm_exec {
            s.push_str(&format!("  vm_exec:     {:>12}\n", format_duration(t)));
        }
        if let Some(t) = self.jit_warmup {
            s.push_str(&format!("  jit_warmup:  {:>12}\n", format_duration(t)));
        }
        if let Some(t) = self.jit_compile {
            s.push_str(&format!("  jit_compile: {:>12}\n", format_duration(t)));
        }
        if let Some(t) = self.jit_exec {
            s.push_str(&format!("  jit_exec:    {:>12}\n", format_duration(t)));
        }
        s.push_str("  ─────────────────────────\n");
        s.push_str(&format!(
            "  frontend:    {:>12}\n",
            format_duration(self.total_frontend())
        ));
        s.push_str(&format!(
            "  backend:     {:>12}\n",
            format_duration(self.total_backend())
        ));
        if self.vm_exec.is_some() {
            s.push_str(&format!(
                "  total (vm):  {:>12}\n",
                format_duration(self.total_vm())
            ));
        }
        if self.jit_exec.is_some() {
            s.push_str(&format!(
                "  total (jit): {:>12}\n",
                format_duration(self.total_jit())
            ));
            if self.jit_warmup.is_some() {
                s.push_str(&format!(
                    "  total+warm:  {:>12}\n",
                    format_duration(self.total_jit_with_warmup())
                ));
            }
        }
        s
    }

    /// Convert to JSON-serializable format (microseconds).
    pub fn to_json_map(&self) -> serde_json::Map<String, serde_json::Value> {
        let mut map = serde_json::Map::new();
        map.insert("lex_us".to_string(), (self.lex.as_micros() as u64).into());
        map.insert(
            "parse_us".to_string(),
            (self.parse.as_micros() as u64).into(),
        );
        map.insert("hir_us".to_string(), (self.hir.as_micros() as u64).into());
        map.insert(
            "infer_us".to_string(),
            (self.infer.as_micros() as u64).into(),
        );
        map.insert("mir_us".to_string(), (self.mir.as_micros() as u64).into());
        map.insert(
            "compile_us".to_string(),
            (self.compile.as_micros() as u64).into(),
        );
        if let Some(t) = self.vm_exec {
            map.insert("vm_exec_us".to_string(), (t.as_micros() as u64).into());
        }
        map.insert(
            "vm_total_us".to_string(),
            (self.total_vm().as_micros() as u64).into(),
        );
        if let Some(t) = self.jit_warmup {
            map.insert("jit_warmup_us".to_string(), (t.as_micros() as u64).into());
        }
        if let Some(t) = self.jit_compile {
            map.insert("jit_compile_us".to_string(), (t.as_micros() as u64).into());
        }
        if let Some(t) = self.jit_exec {
            map.insert("jit_exec_us".to_string(), (t.as_micros() as u64).into());
            map.insert(
                "jit_total_us".to_string(),
                (self.total_jit().as_micros() as u64).into(),
            );
        }
        map
    }
}

impl Add for PipelineTimings {
    type Output = PipelineTimings;

    fn add(self, other: PipelineTimings) -> PipelineTimings {
        PipelineTimings::add(&self, &other)
    }
}

/// Code size metrics.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CodeSizeMetrics {
    /// Number of bytecode instructions.
    pub bytecode_instructions: usize,
    /// Number of functions/chunks.
    pub function_count: usize,
    /// Total constants in constant pool.
    pub constant_count: usize,
}

impl CodeSizeMetrics {
    pub fn display(&self) -> String {
        format!(
            "  bytecode:    {} instructions\n  functions:   {}\n  constants:   {}\n",
            self.bytecode_instructions, self.function_count, self.constant_count
        )
    }
}

/// Format a duration for human display.
pub fn format_duration(d: Duration) -> String {
    let nanos = d.as_nanos();
    if nanos < 1_000 {
        format!("{}ns", nanos)
    } else if nanos < 1_000_000 {
        format!("{:.2}us", nanos as f64 / 1_000.0)
    } else if nanos < 1_000_000_000 {
        format!("{:.2}ms", nanos as f64 / 1_000_000.0)
    } else {
        format!("{:.2}s", nanos as f64 / 1_000_000_000.0)
    }
}
