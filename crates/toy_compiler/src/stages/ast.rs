//! AST stage: Extract granular items from parsed CST for incremental memoization.
//!
//! Key insight: Parse always runs on file change, but by tracking AST items individually,
//! only changed functions/definitions re-lower to HIR in subsequent stages.

use toy_ast::{AstNode, Item, Root};
use toy_cst::{ResolvedNode, SyntaxKind};

use super::parse::ParsedText;
use crate::db::Db;

/// A single tracked AST item with its CST subtree.
///
/// Each item is tracked independently for fine-grained memoization.
/// Uses `SyntaxKind` directly for quick dispatch - no duplicate enum needed.
#[salsa::tracked]
pub struct AstItem<'db> {
    /// SyntaxKind of this item for quick dispatch.
    pub kind: SyntaxKind,

    /// The CST subtree for this item.
    #[returns(ref)]
    pub syntax: ResolvedNode,
}

/// Tracked collection of all items in a file.
///
/// The Vec contains tracked AstItems, enabling salsa to detect
/// which specific items changed between parses.
#[salsa::tracked]
pub struct AstFile<'db> {
    #[returns(ref)]
    pub items: Vec<AstItem<'db>>,
}

/// Extract granular AST items from parsed CST.
///
/// This query creates individually-tracked AstItems from the parsed tree,
/// enabling fine-grained memoization in downstream stages.
#[salsa::tracked]
pub fn ast_file<'db>(db: &'db dyn Db, parsed: ParsedText<'db>) -> AstFile<'db> {
    let green = parsed.green(db);

    let Some(root) = Root::cast(green) else {
        // Empty/invalid parse - return empty file
        return AstFile::new(db, Vec::new());
    };

    let mut items = Vec::new();
    for item in root.items() {
        let syntax = item.syntax();
        let kind = syntax.kind();
        // Item::can_cast validates this is a valid item kind
        if Item::can_cast(kind) {
            items.push(AstItem::new(db, kind, syntax.clone()));
        }
    }

    AstFile::new(db, items)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::Database;
    use crate::stages::parse::parse_text;
    use crate::stages::read::{SourceText, source_to_file_text};

    impl<'db> AstItem<'db> {
        /// Attempt to cast this AstItem back to a typed AST Item.
        ///
        /// Returns None if the kind does not correspond to a valid Item.
        #[allow(clippy::wrong_self_convention)]
        pub fn to_item(&self, db: &'db dyn Db) -> Option<Item> {
            let syntax = self.syntax(db);
            Item::cast(syntax)
        }
    }

    fn parse_and_extract<'db>(db: &'db Database, source: &str) -> AstFile<'db> {
        let source_text = SourceText::new(db, source.to_string());
        let file_text = source_to_file_text(db, source_text);
        let parsed = parse_text(db, file_text);
        ast_file(db, parsed)
    }

    #[test]
    fn empty_source() {
        let db = Database::default();
        let ast = parse_and_extract(&db, "");
        assert!(ast.items(&db).is_empty());
    }

    #[test]
    fn single_variable_definition() {
        let db = Database::default();
        let ast = parse_and_extract(&db, "x := 42");

        let items = ast.items(&db);
        assert_eq!(items.len(), 1);
        assert_eq!(items[0].kind(&db), SyntaxKind::VariableDefinition);
    }

    #[test]
    fn single_function_definition() {
        let db = Database::default();
        let ast = parse_and_extract(&db, "fn add(a: int, b: int): int { a + b }");

        let items = ast.items(&db);
        assert_eq!(items.len(), 1);
        assert_eq!(items[0].kind(&db), SyntaxKind::FunctionDefinition);
    }

    #[test]
    fn multiple_items() {
        let db = Database::default();
        let ast = parse_and_extract(
            &db,
            r#"
x := 1
y := 2
fn foo() { }
z := x + y
"#,
        );

        let items = ast.items(&db);
        assert_eq!(items.len(), 4);
        assert_eq!(items[0].kind(&db), SyntaxKind::VariableDefinition);
        assert_eq!(items[1].kind(&db), SyntaxKind::VariableDefinition);
        assert_eq!(items[2].kind(&db), SyntaxKind::FunctionDefinition);
        assert_eq!(items[3].kind(&db), SyntaxKind::VariableDefinition);
    }

    #[test]
    fn expression_item() {
        let db = Database::default();
        let ast = parse_and_extract(&db, "1 + 2");

        let items = ast.items(&db);
        assert_eq!(items.len(), 1);
        // Expression items store their specific SyntaxKind (InfixExpression here)
        assert_eq!(items[0].kind(&db), SyntaxKind::InfixExpression);
    }

    #[test]
    fn variable_assignment() {
        let db = Database::default();
        let ast = parse_and_extract(
            &db,
            r#"
x := 1
x = 2
"#,
        );

        let items = ast.items(&db);
        assert_eq!(items.len(), 2);
        assert_eq!(items[0].kind(&db), SyntaxKind::VariableDefinition);
        assert_eq!(items[1].kind(&db), SyntaxKind::VariableAssignment);
    }

    #[test]
    fn control_flow_statements() {
        let db = Database::default();
        let ast = parse_and_extract(
            &db,
            r#"
fn test() {
    return 42
}
echo "hello"
"#,
        );

        let items = ast.items(&db);
        assert_eq!(items.len(), 2);
        assert_eq!(items[0].kind(&db), SyntaxKind::FunctionDefinition);
        assert_eq!(items[1].kind(&db), SyntaxKind::EchoStatement);
    }

    #[test]
    fn to_item_roundtrip() {
        let db = Database::default();
        let ast = parse_and_extract(&db, "x := 42");

        let items = ast.items(&db);
        let ast_item = &items[0];

        // Should successfully cast back to typed AST
        let typed = ast_item.to_item(&db);
        assert!(typed.is_some());
        assert!(matches!(typed.unwrap(), Item::VariableDefinition(_)));
    }

    #[test]
    fn items_preserve_syntax() {
        let db = Database::default();
        let ast = parse_and_extract(&db, "x := 42");

        let items = ast.items(&db);
        let syntax = items[0].syntax(&db);

        // Syntax node should have correct kind
        assert_eq!(syntax.kind(), SyntaxKind::VariableDefinition);
    }

    // ========================================
    // Memoization tests using TrackedDatabase
    // ========================================

    mod memoization {
        use super::*;
        use crate::db::test_utils::TrackedDatabase;

        #[test]
        fn same_input_no_reexecution() {
            let db = TrackedDatabase::new();

            // First call - should execute
            let source = SourceText::new(&db, "x := 42".to_string());
            let file_text = source_to_file_text(&db, source);
            let _parsed = parse_text(&db, file_text);

            let exec_after_first = db.counters.will_execute();
            assert!(exec_after_first > 0, "should execute on first call");

            db.counters.reset();

            // Second call with SAME input (same source, file_text) - should be memoized
            let _parsed2 = parse_text(&db, file_text);

            let exec_after_second = db.counters.will_execute();
            assert_eq!(
                exec_after_second, 0,
                "should not re-execute with same input"
            );
            // Note: DidValidateMemoizedValue may not fire if value returned directly from cache
        }

        #[test]
        fn different_input_reexecutes() {
            let db = TrackedDatabase::new();

            // First source
            let source1 = SourceText::new(&db, "x := 1".to_string());
            let file_text1 = source_to_file_text(&db, source1);
            let _parsed1 = parse_text(&db, file_text1);

            db.counters.reset();

            // Different source - should re-execute
            let source2 = SourceText::new(&db, "y := 2".to_string());
            let file_text2 = source_to_file_text(&db, source2);
            let _parsed2 = parse_text(&db, file_text2);

            assert!(
                db.counters.will_execute() > 0,
                "should re-execute with different input"
            );
        }

        #[test]
        fn ast_file_memoized_on_same_parsed() {
            let db = TrackedDatabase::new();

            // Create inputs and run pipeline
            let source = SourceText::new(&db, "fn foo() { 42 }".to_string());
            let file_text = source_to_file_text(&db, source);
            let parsed = parse_text(&db, file_text);
            let _ast1 = ast_file(&db, parsed);

            let exec_after_first = db.counters.will_execute();
            assert!(exec_after_first > 0);

            db.counters.reset();

            // Call ast_file again with SAME parsed result - should be memoized
            let _ast2 = ast_file(&db, parsed);

            assert_eq!(
                db.counters.will_execute(),
                0,
                "ast_file should be memoized when called with same parsed input"
            );
        }

        #[test]
        fn full_pipeline_memoized_on_same_input() {
            let db = TrackedDatabase::new();

            // Create input ONCE and run pipeline
            let source = SourceText::new(&db, "x := 1\ny := 2\nz := 3".to_string());
            let file_text = source_to_file_text(&db, source);
            let parsed = parse_text(&db, file_text);
            let _ast = ast_file(&db, parsed);

            let first_exec_count = db.counters.will_execute();
            assert!(first_exec_count > 0, "should execute on first run");

            db.counters.reset();

            // Run entire pipeline again with SAME inputs - all should be memoized
            let file_text2 = source_to_file_text(&db, source);
            let parsed2 = parse_text(&db, file_text2);
            let _ast2 = ast_file(&db, parsed2);

            let second_exec_count = db.counters.will_execute();
            assert_eq!(
                second_exec_count, 0,
                "no re-execution on second run with same inputs"
            );
        }

        #[test]
        fn new_input_with_same_content_reexecutes() {
            let db = TrackedDatabase::new();

            // First input
            let source1 = SourceText::new(&db, "x := 1".to_string());
            let file_text1 = source_to_file_text(&db, source1);
            let _parsed1 = parse_text(&db, file_text1);

            db.counters.reset();

            // NEW input with same CONTENT - this is a different input ID, so re-executes
            let source2 = SourceText::new(&db, "x := 1".to_string());
            let file_text2 = source_to_file_text(&db, source2);
            let _parsed2 = parse_text(&db, file_text2);

            // This demonstrates inputs are identity-based, not content-based
            assert!(
                db.counters.will_execute() > 0,
                "new input (even with same content) triggers re-execution"
            );
        }

        #[test]
        fn modified_source_invalidates_downstream() {
            let db = TrackedDatabase::new();

            // Initial parse
            let source1 = SourceText::new(&db, "x := 1".to_string());
            let file_text1 = source_to_file_text(&db, source1);
            let parsed1 = parse_text(&db, file_text1);
            let _ast1 = ast_file(&db, parsed1);

            db.counters.reset();

            // Modified source - should re-execute parse and ast_file
            let source2 = SourceText::new(&db, "x := 1\ny := 2".to_string());
            let file_text2 = source_to_file_text(&db, source2);
            let parsed2 = parse_text(&db, file_text2);
            let _ast2 = ast_file(&db, parsed2);

            assert!(
                db.counters.will_execute() > 0,
                "modified source should trigger re-execution"
            );
        }
    }
}
