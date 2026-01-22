//! Basic compilation test

use std::env;
use std::fs;
use toy_compiler::options::{CompileProfile, CompileSource};
use toy_compiler::{CompileOptions, compile};

#[test]
fn test_basic_compile() {
    // Initialize logger for tests
    let _ = env_logger::builder()
        .filter_level(log::LevelFilter::Debug)
        .is_test(true)
        .try_init();

    // Create a temporary test file
    let test_dir = env::temp_dir().join("toy_test_basic");
    fs::create_dir_all(&test_dir).unwrap();

    let test_file = test_dir.join("test.toy");
    fs::write(&test_file, "x := 42").unwrap();

    // Compile the file
    let options = CompileOptions {
        source: CompileSource::Script(test_file),
        profile: CompileProfile::Debug,
    };

    let result = compile(options);

    // Check that compilation doesn't panic
    // The result will be empty since we haven't implemented the full pipeline
    assert!(result.is_ok(), "Compilation should not fail");

    // Clean up
    fs::remove_dir_all(&test_dir).ok();
}
