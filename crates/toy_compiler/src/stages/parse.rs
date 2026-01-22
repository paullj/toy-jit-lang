use toy_cst::ResolvedNode;
use toy_parser::ParseError;

use crate::error::CompilerError;

use super::Diagnostics;
use super::read::FileText;

use salsa::Accumulator;

#[salsa::tracked]
pub struct ParsedText<'db> {
    #[returns(ref)]
    pub green: ResolvedNode,
    #[returns(ref)]
    pub errors: Vec<ParseError>,
}

#[salsa::tracked()]
pub fn parse_text<'db>(db: &'db dyn crate::db::Db, file_text: FileText<'db>) -> ParsedText<'db> {
    let (resolved_node, errors) = toy_parser::parse(file_text.text(db));
    for err in &errors {
        Diagnostics(CompilerError::from(err.clone())).accumulate(db);
    }

    ParsedText::new(db, resolved_node, errors)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::test_utils::TrackedDatabase;
    use crate::stages::read::{SourceText, source_to_file_text};

    mod memoization {
        use super::*;

        #[test]
        fn parse_memoized_on_same_file_text() {
            let db = TrackedDatabase::new();

            // Create input and parse
            let source = SourceText::new(&db, "x := 1 + 2".to_string());
            let file_text = source_to_file_text(&db, source);
            let _parsed1 = parse_text(&db, file_text);

            let exec_after_first = db.counters.will_execute();
            assert!(exec_after_first > 0, "should execute on first parse");

            db.counters.reset();

            // Parse again with SAME FileText - should be memoized
            let _parsed2 = parse_text(&db, file_text);

            assert_eq!(
                db.counters.will_execute(),
                0,
                "parse should be memoized with same FileText"
            );
            // Note: DidValidateMemoizedValue may not fire if value returned directly from cache
        }

        #[test]
        fn parse_reexecutes_on_different_source() {
            let db = TrackedDatabase::new();

            // First parse
            let source1 = SourceText::new(&db, "x := 1".to_string());
            let file_text1 = source_to_file_text(&db, source1);
            let _parsed1 = parse_text(&db, file_text1);

            db.counters.reset();

            // Different source text - should re-execute
            let source2 = SourceText::new(&db, "y := 2".to_string());
            let file_text2 = source_to_file_text(&db, source2);
            let _parsed2 = parse_text(&db, file_text2);

            assert!(
                db.counters.will_execute() > 0,
                "parse should re-execute with different source"
            );
        }

        #[test]
        fn source_to_file_text_memoized() {
            let db = TrackedDatabase::new();

            // Create source and convert to file text
            let source = SourceText::new(&db, "fn test() { }".to_string());
            let _file_text1 = source_to_file_text(&db, source);

            let exec_after_first = db.counters.will_execute();
            assert!(exec_after_first > 0);

            db.counters.reset();

            // Call again with same SourceText - should be memoized
            let _file_text2 = source_to_file_text(&db, source);

            assert_eq!(
                db.counters.will_execute(),
                0,
                "source_to_file_text should be memoized"
            );
        }

        #[test]
        fn parse_produces_correct_result_after_memoization() {
            let db = TrackedDatabase::new();

            let source = SourceText::new(&db, "x := 42\ny := 10".to_string());
            let file_text = source_to_file_text(&db, source);

            // First parse
            let parsed1 = parse_text(&db, file_text);
            let errors1 = parsed1.errors(&db);

            // Second parse (memoized)
            let parsed2 = parse_text(&db, file_text);
            let errors2 = parsed2.errors(&db);

            // Results should be identical
            assert_eq!(errors1.len(), errors2.len());
            assert!(errors1.is_empty(), "valid code should have no errors");
        }

        #[test]
        fn parse_with_errors_also_memoized() {
            let db = TrackedDatabase::new();

            // Invalid syntax
            let source = SourceText::new(&db, "x := ".to_string());
            let file_text = source_to_file_text(&db, source);

            // First parse
            let parsed1 = parse_text(&db, file_text);
            assert!(!parsed1.errors(&db).is_empty(), "should have parse errors");

            db.counters.reset();

            // Second parse - even errors should be memoized
            let _parsed2 = parse_text(&db, file_text);

            assert_eq!(
                db.counters.will_execute(),
                0,
                "parse errors should also be memoized"
            );
        }
    }
}
