use std::path::PathBuf;

use crate::db::Db;

#[salsa::input]
pub struct FilePath {
    #[returns(ref)]
    pub path: PathBuf,
}

/// Direct source text input - used for REPL, tests, and inline compilation.
#[salsa::input]
pub struct SourceText {
    #[returns(ref)]
    pub text: String,
}

#[salsa::tracked(debug)]
pub struct FileText<'db> {
    #[tracked]
    #[returns(ref)]
    pub text: String,
}

#[salsa::tracked]
pub fn read_source_file<'db>(db: &'db dyn Db, input: FilePath) -> FileText<'db> {
    let path = input.path(db);
    let text = match std::fs::read_to_string(path) {
        Ok(text) => text,
        Err(err) => {
            log::error!("Failed to read source file {:?}: {}", path, err);
            String::new()
        }
    };
    FileText::new(db, text)
}

/// Create FileText from direct source text input.
#[salsa::tracked]
pub fn source_to_file_text<'db>(db: &'db dyn Db, input: SourceText) -> FileText<'db> {
    FileText::new(db, input.text(db).clone())
}
