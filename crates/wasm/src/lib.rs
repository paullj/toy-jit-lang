//! WASM bindings for the toy language playground.

use ast::AstNode;
use serde::Serialize;
use wasm_bindgen::prelude::*;

/// Diagnostic for display in the editor.
#[derive(Serialize)]
pub struct JsDiagnostic {
    pub start_line: u32,
    pub start_col: u32,
    pub end_line: u32,
    pub end_col: u32,
    pub severity: &'static str,
    pub message: String,
}

/// Result of running code.
#[derive(Serialize)]
pub struct RunResult {
    pub success: bool,
    pub value: Option<String>,
    pub value_type: Option<String>,
    pub error: Option<String>,
}

/// Hover info result.
#[derive(Serialize)]
pub struct HoverInfo {
    pub found: bool,
    pub name: Option<String>,
    pub type_info: Option<String>,
}

/// Go-to-definition result.
#[derive(Serialize)]
pub struct GotoDefResult {
    pub found: bool,
    pub line: Option<u32>,
    pub col: Option<u32>,
}

/// Analyze source code and return diagnostics.
#[wasm_bindgen]
pub fn analyze(source: &str) -> JsValue {
    let doc = analyse::Document::new(source.to_string());
    let diagnostics: Vec<JsDiagnostic> = doc
        .diagnostics()
        .iter()
        .map(|d| {
            let (range, severity, _code, message, _help) =
                analyse::to_lsp_fields(d, &doc.line_index);
            JsDiagnostic {
                start_line: range.start.line,
                start_col: range.start.character,
                end_line: range.end.line,
                end_col: range.end.character,
                severity: match severity {
                    analyse::DiagnosticSeverity::Error => "error",
                    analyse::DiagnosticSeverity::Warning => "warning",
                    analyse::DiagnosticSeverity::Info => "info",
                    analyse::DiagnosticSeverity::Hint => "hint",
                },
                message,
            }
        })
        .collect();

    serde_wasm_bindgen::to_value(&diagnostics).unwrap_or(JsValue::NULL)
}

/// Run source code and return the result.
#[wasm_bindgen]
pub fn run(source: &str) -> JsValue {
    let result = run_inner(source);
    serde_wasm_bindgen::to_value(&result).unwrap_or(JsValue::NULL)
}

fn run_inner(source: &str) -> RunResult {
    // Parse
    let (syntax, parse_errors) = parse::parse(source);
    if !parse_errors.is_empty() {
        return RunResult {
            success: false,
            value: None,
            value_type: None,
            error: Some(format!("Parse error: {}", parse_errors[0])),
        };
    }

    // Lower to AST
    let Some(root) = ast::Root::cast(syntax) else {
        return RunResult {
            success: false,
            value: None,
            value_type: None,
            error: Some("Failed to parse AST".into()),
        };
    };

    // Lower to HIR
    let lower_result = hir::lower(root);

    // Type inference
    let infer_result = infer::infer(&lower_result);
    if !infer_result.diagnostics.is_empty() {
        return RunResult {
            success: false,
            value: None,
            value_type: None,
            error: Some(format!("Type error: {}", infer_result.diagnostics[0])),
        };
    }

    // Lower to MIR
    let mir_module = mir::lower(&lower_result, &infer_result);

    // Compile to bytecode
    let compiled = compile::compile(&mir_module);

    // Run
    match vm::run(&compiled) {
        Ok(value) => {
            let type_str = match &value {
                vm::Value::Int(_) => "Int",
                vm::Value::Float(_) => "Float",
                vm::Value::Bool(_) => "Bool",
                vm::Value::String(_) => "String",
                vm::Value::Unit => "Unit",
            };
            RunResult {
                success: true,
                value: Some(value.to_string()),
                value_type: Some(type_str.into()),
                error: None,
            }
        }
        Err(e) => RunResult {
            success: false,
            value: None,
            value_type: None,
            error: Some(format!("Runtime error: {}", e)),
        },
    }
}

/// Get hover info (type) at a position.
#[wasm_bindgen]
pub fn hover(source: &str, line: u32, col: u32) -> JsValue {
    let doc = analyse::Document::new(source.to_string());
    let pos = analyse::Position::new(line, col);

    let result = match doc.symbol_at(pos) {
        Some(symbol) => {
            let type_info = doc.type_of_symbol(symbol).map(|t| format!("{}", t));
            HoverInfo {
                found: true,
                name: Some(symbol.name.clone()),
                type_info,
            }
        }
        None => HoverInfo {
            found: false,
            name: None,
            type_info: None,
        },
    };

    serde_wasm_bindgen::to_value(&result).unwrap_or(JsValue::NULL)
}

/// Get definition location for symbol at position.
#[wasm_bindgen]
pub fn goto_def(source: &str, line: u32, col: u32) -> JsValue {
    let doc = analyse::Document::new(source.to_string());
    let pos = analyse::Position::new(line, col);

    let result = match doc.symbol_at(pos) {
        Some(symbol) => {
            let def_pos = doc.line_index.position(symbol.def_span.start().into());
            GotoDefResult {
                found: true,
                line: Some(def_pos.line),
                col: Some(def_pos.character),
            }
        }
        None => GotoDefResult {
            found: false,
            line: None,
            col: None,
        },
    };

    serde_wasm_bindgen::to_value(&result).unwrap_or(JsValue::NULL)
}
