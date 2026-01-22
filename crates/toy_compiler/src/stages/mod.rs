use crate::error::CompilerError;

pub(crate) mod ast;
pub(crate) mod codegen;
pub(crate) mod hir;
pub(crate) mod mir;
pub(crate) mod parse;
pub(crate) mod read;
pub(crate) mod typecheck;

#[salsa::accumulator]
pub struct Diagnostics(#[allow(dead_code)] pub CompilerError);

/// Result of compiling a single file through the full pipeline.
pub struct CompileResult {
    pub executable_bytes: Vec<u8>,
    pub has_type_errors: bool,
}

/// Result of type checking a single file (without code generation).
pub struct CheckResult {
    pub has_type_errors: bool,
}

/// Intermediate result containing parsed AST for module resolution.
#[derive(Clone, Copy)]
pub struct ParsedModule<'db> {
    pub file_path: read::FilePath,
    pub parsed: parse::ParsedText<'db>,
    pub ast: ast::AstFile<'db>,
}

/// Parse a file and return AST for module resolution phase.
pub fn parse_module(db: &dyn crate::db::Db, file_path: read::FilePath) -> ParsedModule<'_> {
    let file_text = read::read_source_file(db, file_path);
    let parsed = parse::parse_text(db, file_text);
    let ast = ast::ast_file(db, parsed);

    ParsedModule {
        file_path,
        parsed,
        ast,
    }
}

/// Compile a file through the full pipeline: read → parse → ast → typecheck → mir → codegen
#[allow(dead_code)]
pub fn compile_module(_db: &dyn crate::db::Db, _file_path: read::FilePath) -> CompileResult {
    // TODO: This function will be refactored to take resolved modules
    // For now, keeping as placeholder
    todo!("Refactor to use resolved modules from module resolution phase");
}
