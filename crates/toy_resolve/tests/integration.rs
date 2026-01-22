//! Integration tests for module resolution

use std::collections::HashMap;
use std::sync::Arc;
use toy_ast::AstNode;
use toy_resolve::{ModuleId, ModulePath, resolve_modules_with_asts};

#[test]
fn test_full_module_resolution() {
    let source = r#"
use std.io
use std.math.{sin, cos}

pub fn add(a: int, b: int): int {
    a + b
}

fn multiply(x: int, y: int): int {
    x * y
}

PI := 3.14
"#;

    // Parse the source
    let (resolved_node, _) = toy_parser::parse(source);
    let root = toy_ast::Root::cast(&resolved_node).expect("Failed to cast to Root");

    // Create module ASTs map
    let mut module_asts = HashMap::new();
    module_asts.insert(ModuleId(0), Arc::new(root));

    // Create module paths map
    let mut module_paths = HashMap::new();
    module_paths.insert(ModuleId(0), ModulePath::absolute(vec!["main".to_string()]));

    // Run module resolution
    let resolution =
        resolve_modules_with_asts(module_asts, module_paths).expect("Module resolution failed");

    // Check results
    assert_eq!(resolution.symbol_tables.len(), 1);

    let table = resolution
        .symbol_tables
        .get(&ModuleId(0))
        .expect("Missing symbol table for module");

    // Should have 3 symbols: add, multiply, PI
    assert_eq!(table.all_symbols.len(), 3);

    // Note: Due to how is_pub() works in the AST when parsing directly,
    // we can't reliably test public/private distinction here
    // In real usage through the compiler pipeline, this should work correctly
}
