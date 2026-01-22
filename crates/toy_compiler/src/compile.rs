use crate::CompileOptions;
use crate::db::Database;
use crate::options::CompileSource;
use crate::stages::read::FilePath;
use crate::stages::{
    self, CheckResult, CompileResult, ParsedModule, ast, parse, parse_module, read,
};
use miette::Result;
use std::collections::HashMap;
use std::path::PathBuf;
use std::process::Command;
use std::sync::Arc;
use std::sync::mpsc::channel;
use std::time::Instant;
use threadpool::ThreadPool;
use toy_resolve::{ModuleId, ModulePath, Resolution};

/// Result of type checking phase (shared between check and compile)
struct TypeCheckPhaseResult<'db> {
    parsed_modules: Vec<ParsedModule<'db>>,
    resolution: Resolution,
    typed_modules: HashMap<toy_hir::ModuleId, Arc<stages::typecheck::TypedModule>>,
    has_type_errors: bool,
}

pub fn compile(options: CompileOptions) -> Result<Vec<CompileResult>> {
    #[allow(clippy::arc_with_non_send_sync)]
    let db = Arc::new(Database::default());
    let file_paths = file_paths_from_source(&*db, options.source);

    let workers = match std::thread::available_parallelism() {
        Ok(n) => {
            log::debug!("Using {} workers for parallel execution", n);
            n.get()
        }
        Err(e) => {
            log::error!(
                "Failed to get available parallelism: {}. Defaulting to 1 worker.",
                e
            );
            1
        }
    };
    let pool = ThreadPool::with_name("toy-compiler-worker".to_string(), workers);

    // Phases 1-4: Parse, resolve, lower to HIR, and type check
    let phase_result = run_typecheck_phases(&db, file_paths, pool.clone())?;
    let _parsed_modules = phase_result.parsed_modules;
    let resolution = phase_result.resolution;
    let typed_modules = phase_result.typed_modules;

    // Phase 5: Lower HIR to MIR using TypedModule
    log::debug!("Starting MIR lowering");
    let mir_start = Instant::now();

    let mut mir_modules: HashMap<toy_hir::ModuleId, Arc<stages::mir::MirResult>> = HashMap::new();

    // Compute SCCs for MIR lowering (following type checking order)
    let sccs = resolution.graph.find_sccs();

    // Process SCCs for MIR lowering
    for scc in &sccs {
        if scc.len() == 1 {
            // Single module - can be lowered independently
            let module_id = toy_hir::ModuleId(scc[0].0);
            log::debug!("Lowering single module to MIR: {:?}", module_id);

            if let Some(typed_module) = typed_modules.get(&module_id) {
                log::debug!(
                    "Module {:?} has {} typed items",
                    module_id,
                    typed_module.items.len()
                );
                let mir = stages::mir::lower_module(&*db, module_id, Arc::clone(typed_module));
                mir_modules.insert(module_id, mir);
            }
        } else {
            // Multiple mutually recursive modules - need to lower together
            log::debug!("Lowering SCC with {} modules to MIR", scc.len());

            let scc_module_ids: Vec<_> = scc.iter().map(|id| toy_hir::ModuleId(id.0)).collect();
            let scc_typed_modules: Vec<_> = scc_module_ids
                .iter()
                .filter_map(|id| typed_modules.get(id).map(Arc::clone))
                .collect();

            if scc_typed_modules.len() == scc_module_ids.len() {
                let mir_scc = stages::mir::lower_scc(&*db, &scc_module_ids, &scc_typed_modules);
                for (module_id, mir_module) in scc_module_ids.iter().zip(mir_scc) {
                    mir_modules.insert(*module_id, mir_module);
                }
            } else {
                log::error!("Missing typed modules for SCC MIR lowering");
            }
        }
    }

    log::debug!(
        "Completed MIR lowering in {:.2}ms",
        mir_start.elapsed().as_secs_f64() * 1000.0
    );

    // Debug: Check what's in the MIR modules
    for (module_id, mir_result) in &mir_modules {
        log::debug!(
            "Module {:?} has {} functions",
            module_id,
            mir_result.module.functions.len()
        );
    }

    // Phase 6: Code generation
    log::debug!("Starting code generation");
    let codegen_start = Instant::now();

    // Convert HashMap to Vec for codegen_modules
    let mir_modules_vec: Vec<(toy_hir::ModuleId, Arc<stages::mir::MirResult>)> =
        mir_modules.into_iter().collect();

    let codegen_results = match stages::codegen::codegen_modules(mir_modules_vec) {
        Ok(results) => {
            log::debug!(
                "Completed code generation in {:.2}ms",
                codegen_start.elapsed().as_secs_f64() * 1000.0
            );
            results
        }
        Err(e) => {
            log::error!("Code generation failed: {}", e);
            return Err(miette::miette!("Code generation failed: {}", e));
        }
    };

    // Check if any modules had codegen errors (should have failed fast already)
    for result in &codegen_results {
        if let Some(err) = &result.error {
            log::error!("Module {:?} codegen error: {}", result.module_id, err);
            return Err(miette::miette!(
                "Code generation error in module {:?}: {}",
                result.module_id,
                err
            ));
        }
    }

    // Phase 7: Linking
    log::debug!("Starting linking");
    let link_start = Instant::now();

    // Link all objects into executable
    let executable = link_modules(codegen_results)?;

    log::debug!(
        "Completed linking in {:.2}ms",
        link_start.elapsed().as_secs_f64() * 1000.0
    );
    Ok(vec![executable])
}

/// Type check a script up to and including type checking, without code generation
pub fn check(options: CompileOptions) -> Result<Vec<CheckResult>> {
    #[allow(clippy::arc_with_non_send_sync)]
    let db = Arc::new(Database::default());
    let file_paths = file_paths_from_source(&*db, options.source);

    let workers = match std::thread::available_parallelism() {
        Ok(n) => {
            log::debug!("Using {} workers for parallel execution", n);
            n.get()
        }
        Err(e) => {
            log::error!(
                "Failed to get available parallelism: {}. Defaulting to 1 worker.",
                e
            );
            1
        }
    };
    let pool = ThreadPool::with_name("toy-compiler-worker".to_string(), workers);

    // Phases 1-4: Parse, resolve, lower to HIR, and type check
    let phase_result = run_typecheck_phases(&db, file_paths, pool)?;

    log::info!("Check complete");
    Ok(vec![CheckResult {
        has_type_errors: phase_result.has_type_errors,
    }])
}

/// Link compiled objects into a single executable
fn link_modules(codegen_results: Vec<stages::codegen::CodegenResult>) -> Result<CompileResult> {
    // Create temp directory for object files
    let temp_dir = std::env::temp_dir().join(format!("toy_build_{}", std::process::id()));
    std::fs::create_dir_all(&temp_dir)
        .map_err(|e| miette::miette!("Failed to create temp directory: {}", e))?;

    // Write object files to temp directory
    let mut object_paths = Vec::new();
    for result in &codegen_results {
        if let Some(obj) = &result.object {
            let obj_path = temp_dir.join(format!("module_{}.o", result.module_id.0));
            std::fs::write(&obj_path, &obj.bytes)
                .map_err(|e| miette::miette!("Failed to write object file: {}", e))?;
            object_paths.push(obj_path);
        }
    }

    if object_paths.is_empty() {
        // Clean up temp dir
        let _ = std::fs::remove_dir_all(&temp_dir);
        return Err(miette::miette!("No object files to link"));
    }

    // Find runtime library
    let runtime_lib = find_runtime_lib()?;

    // Create output path in temp dir
    let exe_path = temp_dir.join("toy_output");

    // Link with system linker
    let mut cmd = Command::new("cc");
    cmd.arg("-o").arg(&exe_path);

    // Add all object files
    for obj_path in &object_paths {
        cmd.arg(obj_path);
    }

    // Add runtime library
    cmd.arg(&runtime_lib);

    // Platform-specific optimizations
    #[cfg(target_os = "macos")]
    cmd.arg("-Wl,-dead_strip");
    #[cfg(target_os = "linux")]
    cmd.args(["-Wl,--gc-sections", "-Wl,--as-needed"]);

    let output = cmd
        .output()
        .map_err(|e| miette::miette!("Failed to run linker: {}", e))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        // Clean up temp dir
        let _ = std::fs::remove_dir_all(&temp_dir);
        return Err(miette::miette!("Linking failed: {}", stderr));
    }

    // Read executable bytes
    let executable_bytes = std::fs::read(&exe_path)
        .map_err(|e| miette::miette!("Failed to read linked executable: {}", e))?;

    // Clean up temp directory
    let _ = std::fs::remove_dir_all(&temp_dir);

    // Return CompileResult with executable bytes
    Ok(CompileResult {
        executable_bytes,
        has_type_errors: false, // Already checked in earlier phases
    })
}

/// Find the runtime library for linking
fn find_runtime_lib() -> Result<PathBuf> {
    // Try relative to current exe (dev mode)
    if let Ok(exe) = std::env::current_exe() {
        let dev_path = exe
            .parent()
            .map(|p| p.join("libtoy_runtime.a"))
            .filter(|p| p.exists());
        if let Some(path) = dev_path {
            return Ok(path);
        }
    }

    // Try cargo target directory - prefer release for smaller binaries
    let cargo_paths = [
        "target/release/libtoy_runtime.a",
        "target/debug/libtoy_runtime.a",
    ];
    for path in cargo_paths {
        let p = PathBuf::from(path);
        if p.exists() {
            return Ok(p);
        }
    }

    Err(miette::miette!(
        "Runtime library not found (libtoy_runtime.a). Build it with: cargo build -p toy_runtime"
    ))
}

/// Run phases 1-4: parse, module resolution, HIR lowering, and type checking
fn run_typecheck_phases<'db>(
    db: &'db Arc<Database>,
    file_paths: Vec<read::FilePath>,
    pool: ThreadPool,
) -> Result<TypeCheckPhaseResult<'db>> {
    let (tx, rx) = channel();

    // Phase 1: Parse all modules in parallel to get ASTs
    log::debug!("Starting parse");
    let parse_start = Instant::now();

    let parsed_modules: Vec<ParsedModule> = match file_paths[..] {
        [] => panic!("No source files found to compile."),
        [file_path] => {
            log::debug!("  Single file, parsing synchronously");
            vec![parse_module(&**db, file_path)]
        }
        [..] => {
            // Extract file paths outside of threads (Salsa queries can't be run in threads)
            let file_path_data: Vec<_> = file_paths
                .iter()
                .map(|fp| (*fp, fp.path(&**db).clone()))
                .collect();

            let n_jobs = file_path_data.len();

            for (file_path, path_buf) in file_path_data {
                let tx = tx.clone();
                pool.execute(move || {
                    let thread_id = std::thread::current().id();
                    let start = Instant::now();

                    // Get file name for logging
                    let file_name = path_buf
                        .file_name()
                        .and_then(|n| n.to_str())
                        .unwrap_or("unknown");

                    // Read file directly without Salsa
                    let text = match std::fs::read_to_string(&path_buf) {
                        Ok(text) => text,
                        Err(err) => {
                            log::error!("Failed to read source file {:?}: {}", path_buf, err);
                            String::new()
                        }
                    };

                    // Parse directly without Salsa
                    let (green, errors) = toy_parser::parse(&text);

                    // Extract AST items directly
                    use toy_ast::AstNode;
                    use toy_cst::SyntaxKind;
                    let root = toy_ast::Root::cast(&green);
                    let items: Vec<(SyntaxKind, toy_cst::ResolvedNode)> = if let Some(root) = &root
                    {
                        root.items()
                            .map(|item| {
                                let syntax = item.syntax().clone();
                                let kind = syntax.kind();
                                (kind, syntax)
                            })
                            .collect()
                    } else {
                        vec![]
                    };

                    let elapsed = start.elapsed().as_secs_f64() * 1000.0;
                    log::debug!(
                        "  Completed parsing {} ({:?}) in {:.2}ms",
                        file_name,
                        thread_id,
                        elapsed
                    );

                    let result = (file_path, text, green, errors, items);
                    tx.send(result).expect("Failed to send parsed module");
                });
            }

            // Collect results and reconstruct ParsedModule structures
            let results: Vec<_> = rx.iter().take(n_jobs).collect();

            // Now create the Salsa tracked structures back on the main thread
            results
                .into_iter()
                .map(|(file_path, text, green, errors, items)| {
                    // Create FileText
                    let _file_text = read::FileText::new(&**db, text);

                    // Create ParsedText with green node and errors
                    let parsed_text = parse::ParsedText::new(&**db, green, errors);

                    // Create AstFile with the extracted items
                    let ast_items: Vec<_> = items
                        .into_iter()
                        .map(|(kind, syntax)| ast::AstItem::new(&**db, kind, syntax))
                        .collect();
                    let ast_file = ast::AstFile::new(&**db, ast_items);

                    ParsedModule {
                        file_path,
                        parsed: parsed_text,
                        ast: ast_file,
                    }
                })
                .collect()
        }
    };
    log::debug!(
        "Completed parse in {:.2}ms",
        parse_start.elapsed().as_secs_f64() * 1000.0
    );

    // Phase 2: Module Resolution
    log::debug!("Starting module resolution");
    let resolution_start = Instant::now();

    // Create module IDs, paths, and ASTs for toy_resolve
    let mut module_paths = HashMap::new();
    let mut module_asts = HashMap::new();

    for (idx, parsed_module) in parsed_modules.iter().enumerate() {
        let module_id = ModuleId(idx as u32);

        // Extract file path and convert to ModulePath
        let file_path = parsed_module.file_path.path(&**db);

        // For now, treat all modules as absolute paths based on file name
        // TODO: Implement proper module path logic based on project structure
        let module_name = file_path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("unknown");
        module_paths.insert(
            module_id,
            ModulePath::absolute(vec![module_name.to_string()]),
        );

        // Extract the Root AST from ParsedText
        use toy_ast::AstNode;
        let green = parsed_module.parsed.green(&**db);
        if let Some(root) = toy_ast::Root::cast(green) {
            module_asts.insert(module_id, Arc::new(root));
        } else {
            log::warn!("Failed to extract Root AST for module {:?}", module_id);
        }
    }

    // Run module resolution with the simpler API
    let resolution: Resolution =
        match toy_resolve::resolve_modules_with_asts(module_asts, module_paths) {
            Ok(res) => res,
            Err(e) => {
                log::error!("Module resolution failed: {:?}", e);
                // For now, create empty resolution to continue
                // TODO: Proper error handling
                Resolution {
                    graph: toy_resolve::DependencyGraph::new(),
                    symbol_tables: HashMap::new(),
                    diagnostics: vec![],
                }
            }
        };

    log::debug!(
        "Completed module resolution in {:.2}ms",
        resolution_start.elapsed().as_secs_f64() * 1000.0
    );

    // Log diagnostics from resolution
    for diagnostic in &resolution.diagnostics {
        log::warn!("Resolution diagnostic: {}", diagnostic.message);
    }

    // Phase 3: Lower to HIR with resolved symbols
    log::debug!("Starting HIR lowering");
    let hir_start = Instant::now();

    let _hir_modules: Vec<_> = if parsed_modules.len() == 1 {
        // Single module - no parallelism needed
        let module = &parsed_modules[0];
        let module_id = toy_hir::ModuleId(0);

        log::debug!("  Single module, lowering synchronously");

        // Get the AST Root
        use toy_ast::AstNode;
        let green = module.parsed.green(&**db);

        if let Some(root) = toy_ast::Root::cast(green) {
            let hir = toy_hir::lower_with_resolution(
                root,
                module_id,
                resolution
                    .symbol_tables
                    .iter()
                    .map(|(id, table)| (toy_hir::ModuleId(id.0), table.clone()))
                    .collect(),
                HashMap::new(), // module_paths not used yet
            );
            vec![(module_id, hir)]
        } else {
            log::error!("Failed to get Root AST for HIR lowering");
            vec![]
        }
    } else {
        // Multiple modules - extract data outside threads then parallelize
        let mut hir_tasks = Vec::new();

        for (idx, module) in parsed_modules.iter().enumerate() {
            let module_id = toy_hir::ModuleId(idx as u32);
            let symbol_tables: HashMap<toy_hir::ModuleId, toy_resolve::SymbolTable> = resolution
                .symbol_tables
                .iter()
                .map(|(id, table)| (toy_hir::ModuleId(id.0), table.clone()))
                .collect();

            // Extract green node outside of thread
            use toy_ast::AstNode;
            let green = module.parsed.green(&**db);

            if let Some(root) = toy_ast::Root::cast(green) {
                // Store the underlying syntax node which is cloneable
                let root_syntax = root.syntax().clone();
                hir_tasks.push((module_id, root_syntax, symbol_tables));
            } else {
                log::error!("Failed to get Root AST for module {:?}", module_id);
            }
        }

        // Now parallelize the HIR lowering
        let (tx, rx) = channel();
        let n_jobs = hir_tasks.len();

        for (module_id, root_syntax, symbol_tables) in hir_tasks {
            let tx = tx.clone();
            pool.execute(move || {
                let thread_id = std::thread::current().id();
                let start = Instant::now();

                // Reconstruct the Root from the syntax node
                use toy_ast::AstNode;
                if let Some(root) = toy_ast::Root::cast(&root_syntax) {
                    let hir = toy_hir::lower_with_resolution(
                        root,
                        module_id,
                        symbol_tables,
                        HashMap::new(), // module_paths not used yet
                    );

                    let elapsed = start.elapsed().as_secs_f64() * 1000.0;
                    log::debug!(
                        "  Completed HIR lowering for module {} ({:?}) in {:.2}ms",
                        module_id.0,
                        thread_id,
                        elapsed
                    );

                    tx.send((module_id, hir))
                        .expect("Failed to send HIR module");
                } else {
                    log::error!("Failed to reconstruct Root AST in thread");
                }
            });
        }

        rx.iter().take(n_jobs).collect()
    };

    log::debug!(
        "Completed HIR lowering in {:.2}ms",
        hir_start.elapsed().as_secs_f64() * 1000.0
    );

    // Phase 4: Type checking across modules
    log::debug!("Starting type checking");
    let typecheck_start = Instant::now();

    // Build a map from module ID to parsed module for AST access
    let mut module_ast_map = HashMap::new();
    for (idx, parsed_module) in parsed_modules.iter().enumerate() {
        let module_id = toy_hir::ModuleId(idx as u32);
        module_ast_map.insert(module_id, parsed_module.ast);
    }

    // Compute strongly connected components (SCCs) for type checking order
    let sccs = resolution.graph.find_sccs();
    log::debug!("Found {} SCC(s) in dependency graph", sccs.len());

    // Type check modules in topological order
    let mut typed_modules: HashMap<toy_hir::ModuleId, Arc<stages::typecheck::TypedModule>> =
        HashMap::new();

    for scc in &sccs {
        if scc.len() == 1 {
            // Single module - can be type checked independently
            let module_id = toy_hir::ModuleId(scc[0].0);
            log::debug!("Type checking single module: {:?}", module_id);

            if let Some(ast_file) = module_ast_map.get(&module_id) {
                // For single modules, we can parallelize with other single modules
                // But for now, keeping sequential for simplicity
                let typed = stages::typecheck::typecheck_module(&**db, module_id, *ast_file);
                typed_modules.insert(module_id, typed);
            }
        } else {
            // Multiple mutually recursive modules - need to type check together
            log::debug!("Type checking SCC with {} modules", scc.len());

            let scc_module_ids: Vec<_> = scc.iter().map(|id| toy_hir::ModuleId(id.0)).collect();
            let scc_ast_files: Vec<_> = scc_module_ids
                .iter()
                .filter_map(|id| module_ast_map.get(id).copied())
                .collect();

            if scc_ast_files.len() == scc_module_ids.len() {
                let typed_scc =
                    stages::typecheck::typecheck_scc(&**db, &scc_module_ids, &scc_ast_files);
                for (module_id, typed_module) in scc_module_ids.iter().zip(typed_scc) {
                    typed_modules.insert(*module_id, typed_module);
                }
            } else {
                log::error!("Missing AST files for SCC modules");
            }
        }
    }

    log::debug!(
        "Completed type checking in {:.2}ms",
        typecheck_start.elapsed().as_secs_f64() * 1000.0
    );

    // Check for type errors
    let has_type_errors = typed_modules.values().any(|m| m.has_errors);
    if has_type_errors {
        log::warn!("Type errors detected in one or more modules");
    }

    Ok(TypeCheckPhaseResult {
        parsed_modules,
        resolution,
        typed_modules,
        has_type_errors,
    })
}

fn file_paths_from_source(db: &dyn crate::Db, source: CompileSource) -> Vec<read::FilePath> {
    match source {
        CompileSource::Script(p) => {
            if p.is_dir() {
                todo!("Expected a script file path, but got a directory: {:?}", p);
            }
            if p.extension().and_then(|s| s.to_str()) != Some("toy") {
                todo!("Expected a .toy file for a script, but got: {:?}", p);
            }
            vec![FilePath::new(db, p)]
        }
        CompileSource::Project(p) => {
            todo!("Project source handling not implemented yet: {:?}", p)
        }
    }
}
