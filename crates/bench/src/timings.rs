//! Pipeline timing utilities.

use serde::{Deserialize, Serialize};
use std::ops::Add;
use std::time::Duration;

/// Statistical summary of pipeline timings across multiple iterations.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PipelineStats {
    pub avg: PipelineTimings,
    pub stddev: PipelineTimings,
    pub iterations: usize,
}

impl PipelineStats {
    /// Calculate statistics from a vector of timing samples.
    pub fn from_samples(samples: &[PipelineTimings]) -> Self {
        let n = samples.len();
        if n == 0 {
            return Self {
                avg: PipelineTimings::default(),
                stddev: PipelineTimings::default(),
                iterations: 0,
            };
        }

        // Calculate averages
        let mut sum = PipelineTimings::default();
        for s in samples {
            sum = sum + s.clone();
        }
        let avg = sum.div(n);

        // Calculate standard deviation
        let stddev = Self::calc_stddev(samples, &avg);

        Self {
            avg,
            stddev,
            iterations: n,
        }
    }

    fn calc_stddev(samples: &[PipelineTimings], avg: &PipelineTimings) -> PipelineTimings {
        let n = samples.len() as f64;
        if n <= 1.0 {
            return PipelineTimings::default();
        }

        let mut var_lex = 0.0;
        let mut var_parse = 0.0;
        let mut var_hir = 0.0;
        let mut var_infer = 0.0;
        let mut var_mir = 0.0;
        let mut var_compile = 0.0;
        let mut var_vm_exec = 0.0;
        let mut var_jit_warmup = 0.0;
        let mut var_jit_compile = 0.0;
        let mut var_jit_exec = 0.0;

        let mut vm_exec_count = 0;
        let mut jit_warmup_count = 0;
        let mut jit_compile_count = 0;
        let mut jit_exec_count = 0;

        for s in samples {
            var_lex += (s.lex.as_nanos() as f64 - avg.lex.as_nanos() as f64).powi(2);
            var_parse += (s.parse.as_nanos() as f64 - avg.parse.as_nanos() as f64).powi(2);
            var_hir += (s.hir.as_nanos() as f64 - avg.hir.as_nanos() as f64).powi(2);
            var_infer += (s.infer.as_nanos() as f64 - avg.infer.as_nanos() as f64).powi(2);
            var_mir += (s.mir.as_nanos() as f64 - avg.mir.as_nanos() as f64).powi(2);
            var_compile += (s.compile.as_nanos() as f64 - avg.compile.as_nanos() as f64).powi(2);

            if let (Some(v), Some(a)) = (s.vm_exec, avg.vm_exec) {
                var_vm_exec += (v.as_nanos() as f64 - a.as_nanos() as f64).powi(2);
                vm_exec_count += 1;
            }
            if let (Some(v), Some(a)) = (s.jit_warmup, avg.jit_warmup) {
                var_jit_warmup += (v.as_nanos() as f64 - a.as_nanos() as f64).powi(2);
                jit_warmup_count += 1;
            }
            if let (Some(v), Some(a)) = (s.jit_compile, avg.jit_compile) {
                var_jit_compile += (v.as_nanos() as f64 - a.as_nanos() as f64).powi(2);
                jit_compile_count += 1;
            }
            if let (Some(v), Some(a)) = (s.jit_exec, avg.jit_exec) {
                var_jit_exec += (v.as_nanos() as f64 - a.as_nanos() as f64).powi(2);
                jit_exec_count += 1;
            }
        }

        let stddev_nanos = |var: f64, count: usize| -> u64 {
            if count <= 1 {
                0
            } else {
                (var / (count as f64 - 1.0)).sqrt() as u64
            }
        };

        PipelineTimings {
            lex: Duration::from_nanos(stddev_nanos(var_lex, samples.len())),
            parse: Duration::from_nanos(stddev_nanos(var_parse, samples.len())),
            hir: Duration::from_nanos(stddev_nanos(var_hir, samples.len())),
            infer: Duration::from_nanos(stddev_nanos(var_infer, samples.len())),
            mir: Duration::from_nanos(stddev_nanos(var_mir, samples.len())),
            compile: Duration::from_nanos(stddev_nanos(var_compile, samples.len())),
            vm_exec: if vm_exec_count > 0 {
                Some(Duration::from_nanos(stddev_nanos(
                    var_vm_exec,
                    vm_exec_count,
                )))
            } else {
                None
            },
            jit_warmup: if jit_warmup_count > 0 {
                Some(Duration::from_nanos(stddev_nanos(
                    var_jit_warmup,
                    jit_warmup_count,
                )))
            } else {
                None
            },
            jit_compile: if jit_compile_count > 0 {
                Some(Duration::from_nanos(stddev_nanos(
                    var_jit_compile,
                    jit_compile_count,
                )))
            } else {
                None
            },
            jit_exec: if jit_exec_count > 0 {
                Some(Duration::from_nanos(stddev_nanos(
                    var_jit_exec,
                    jit_exec_count,
                )))
            } else {
                None
            },
        }
    }

    /// Convert to JSON-serializable format with both avg and stddev (microseconds).
    pub fn to_json_map(&self) -> serde_json::Map<String, serde_json::Value> {
        let mut map = serde_json::Map::new();

        // Averages
        map.insert(
            "lex_avg_us".into(),
            (self.avg.lex.as_micros() as u64).into(),
        );
        map.insert(
            "parse_avg_us".into(),
            (self.avg.parse.as_micros() as u64).into(),
        );
        map.insert(
            "hir_avg_us".into(),
            (self.avg.hir.as_micros() as u64).into(),
        );
        map.insert(
            "infer_avg_us".into(),
            (self.avg.infer.as_micros() as u64).into(),
        );
        map.insert(
            "mir_avg_us".into(),
            (self.avg.mir.as_micros() as u64).into(),
        );
        map.insert(
            "compile_avg_us".into(),
            (self.avg.compile.as_micros() as u64).into(),
        );

        if let Some(t) = self.avg.vm_exec {
            map.insert("vm_exec_avg_us".into(), (t.as_micros() as u64).into());
        }
        map.insert(
            "vm_total_avg_us".into(),
            (self.avg.total_vm().as_micros() as u64).into(),
        );

        if let Some(t) = self.avg.jit_warmup {
            map.insert("jit_warmup_avg_us".into(), (t.as_micros() as u64).into());
        }
        if let Some(t) = self.avg.jit_compile {
            map.insert("jit_compile_avg_us".into(), (t.as_micros() as u64).into());
        }
        if let Some(t) = self.avg.jit_exec {
            map.insert("jit_exec_avg_us".into(), (t.as_micros() as u64).into());
            map.insert(
                "jit_total_avg_us".into(),
                (self.avg.total_jit().as_micros() as u64).into(),
            );
        }

        // Standard deviations
        map.insert(
            "lex_stddev_us".into(),
            (self.stddev.lex.as_micros() as u64).into(),
        );
        map.insert(
            "parse_stddev_us".into(),
            (self.stddev.parse.as_micros() as u64).into(),
        );
        map.insert(
            "hir_stddev_us".into(),
            (self.stddev.hir.as_micros() as u64).into(),
        );
        map.insert(
            "infer_stddev_us".into(),
            (self.stddev.infer.as_micros() as u64).into(),
        );
        map.insert(
            "mir_stddev_us".into(),
            (self.stddev.mir.as_micros() as u64).into(),
        );
        map.insert(
            "compile_stddev_us".into(),
            (self.stddev.compile.as_micros() as u64).into(),
        );

        if let Some(t) = self.stddev.vm_exec {
            map.insert("vm_exec_stddev_us".into(), (t.as_micros() as u64).into());
        }
        if let Some(t) = self.stddev.jit_warmup {
            map.insert("jit_warmup_stddev_us".into(), (t.as_micros() as u64).into());
        }
        if let Some(t) = self.stddev.jit_compile {
            map.insert(
                "jit_compile_stddev_us".into(),
                (t.as_micros() as u64).into(),
            );
        }
        if let Some(t) = self.stddev.jit_exec {
            map.insert("jit_exec_stddev_us".into(), (t.as_micros() as u64).into());
        }

        map
    }
}

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
