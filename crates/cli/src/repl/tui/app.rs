use std::io;

use ast::AstNode;
use crossterm::ExecutableCommand;
use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyModifiers};
use crossterm::terminal::{disable_raw_mode, enable_raw_mode};
use ratatui::style::Color;

use super::input::InputState;
use crate::repl::highlight::highlight;
use crate::repl::session::ReplSession;
use analyse::Document;

/// Main REPL application state.
struct ReplApp {
    session: ReplSession,
    input: InputState,
    analysis: Option<Document>,
    should_quit: bool,
    /// Index into history for up/down navigation (None = current input)
    history_index: Option<usize>,
    /// Saved current input when navigating history
    saved_input: String,
}

impl ReplApp {
    fn new() -> Self {
        Self {
            session: ReplSession::new(),
            input: InputState::new(),
            analysis: None,
            should_quit: false,
            history_index: None,
            saved_input: String::new(),
        }
    }

    fn history_up(&mut self) {
        if self.session.history.is_empty() {
            return;
        }

        match self.history_index {
            None => {
                // Save current input and go to most recent history
                self.saved_input = self.input.text.clone();
                self.history_index = Some(self.session.history.len() - 1);
            }
            Some(idx) if idx > 0 => {
                self.history_index = Some(idx - 1);
            }
            _ => return,
        }

        // Load history entry into input
        if let Some(idx) = self.history_index {
            self.input.text = self.session.history[idx].input.clone();
            self.input.cursor = self.input.text.len();
            self.on_input_change();
        }
    }

    fn history_down(&mut self) {
        match self.history_index {
            Some(idx) if idx + 1 < self.session.history.len() => {
                self.history_index = Some(idx + 1);
                self.input.text = self.session.history[idx + 1].input.clone();
                self.input.cursor = self.input.text.len();
                self.on_input_change();
            }
            Some(_) => {
                // Return to saved input
                self.history_index = None;
                self.input.text = std::mem::take(&mut self.saved_input);
                self.input.cursor = self.input.text.len();
                self.on_input_change();
            }
            None => {}
        }
    }

    fn on_input_change(&mut self) {
        // Re-analyze on every keystroke
        if !self.input.text.is_empty() {
            self.analysis = Some(Document::new(self.input.text.clone()));
        } else {
            self.analysis = None;
        }
    }

    fn execute(&mut self) {
        let text = self.input.take();
        self.history_index = None;
        self.saved_input.clear();

        if text.trim().is_empty() {
            return;
        }

        // Parse and type check with session state
        let (syntax, parse_errors) = parse::parse(&text);

        if !parse_errors.is_empty() {
            let error_msg = format_parse_errors(&text, &parse_errors);
            self.session.add_history(text, Some(error_msg), true);
            self.analysis = None;
            return;
        }

        let Some(root) = ast::Root::cast(syntax) else {
            self.session
                .add_history(text.clone(), Some("failed to parse".into()), true);
            self.analysis = None;
            return;
        };

        let lower = hir::lower(root);
        let infer_result = infer::infer_with_state(&lower, self.session.infer_state.clone());

        if infer_result.result.has_errors() {
            let error_msg = format_infer_errors(&text, &infer_result.result.diagnostics);
            self.session.add_history(text, Some(error_msg), true);
            self.analysis = None;
            return;
        }

        // Update session type state
        self.session.infer_state = infer_result.state;

        // Execute with JIT
        match self.execute_jit(&lower, &infer_result.result) {
            Ok(result) => {
                self.session.add_history(text, result, false);
            }
            Err(e) => {
                self.session.add_history(text, Some(e), true);
            }
        }

        self.analysis = None;
    }

    fn execute_jit(
        &mut self,
        lower: &hir::LowerResult,
        infer: &infer::InferenceResult,
    ) -> Result<Option<String>, String> {
        let mir = mir::lower(lower, infer);
        let mut jit_compiler = jit::Jit::new();

        // Determine return type from last item
        let result_type = lower
            .items
            .last()
            .map(|item| get_item_type(item, lower, infer));

        jit_compiler
            .compile_module(&mir)
            .map_err(|e| format!("JIT error: {:?}", e))?;

        let func_ptr = jit_compiler
            .get_main(mir.main_id)
            .ok_or("Main function not compiled")?;

        // Create runtime context and execute
        let mut context = jit::RuntimeContext::new(lasso::Rodeo::default());
        let raw_result: i64 = unsafe {
            let func: fn(*mut jit::RuntimeContext) -> i64 = std::mem::transmute(func_ptr);
            func(&mut context as *mut jit::RuntimeContext)
        };

        // Execute and format result
        let result_str = if let Some(last_item) = lower.items.last() {
            let result = format_result(raw_result, result_type.as_ref(), &context.heap);

            match last_item {
                hir::Item::Expression(_) => Some(result),
                hir::Item::Definition(hir::Definition::Variable { name, .. }) => {
                    Some(format!("{} = {}", lower.resolve(*name), result))
                }
                hir::Item::Definition(hir::Definition::Function { name, .. }) => {
                    Some(format!("{} = <function>", lower.resolve(*name)))
                }
                hir::Item::Assignment { name, .. } => {
                    Some(format!("{} = {}", lower.resolve(*name), result))
                }
                hir::Item::IndexAssignment { .. } => None,
            }
        } else {
            None
        };

        Ok(result_str)
    }

    fn handle_key(&mut self, key: KeyEvent) {
        match (key.modifiers, key.code) {
            // Quit
            (KeyModifiers::CONTROL, KeyCode::Char('c'))
            | (KeyModifiers::CONTROL, KeyCode::Char('d')) => {
                self.should_quit = true;
            }

            // Execute
            (KeyModifiers::NONE, KeyCode::Enter) => {
                self.execute();
            }

            // Multi-line (Shift+Enter)
            (KeyModifiers::SHIFT, KeyCode::Enter) => {
                self.input.insert_newline();
                self.on_input_change();
            }

            // Backspace
            (_, KeyCode::Backspace) => {
                self.input.backspace();
                self.on_input_change();
            }

            // Delete
            (_, KeyCode::Delete) => {
                self.input.delete();
                self.on_input_change();
            }

            // Navigation
            (_, KeyCode::Left) => self.input.move_left(),
            (_, KeyCode::Right) => self.input.move_right(),
            (_, KeyCode::Home) | (KeyModifiers::CONTROL, KeyCode::Char('a')) => {
                self.input.move_home()
            }
            (_, KeyCode::End) | (KeyModifiers::CONTROL, KeyCode::Char('e')) => {
                self.input.move_end()
            }

            // History navigation
            (_, KeyCode::Up) => self.history_up(),
            (_, KeyCode::Down) => self.history_down(),

            // Character input
            (KeyModifiers::NONE | KeyModifiers::SHIFT, KeyCode::Char(c)) => {
                self.input.insert(c);
                self.on_input_change();
            }

            _ => {}
        }
    }
}

fn get_item_type(
    item: &hir::Item,
    hir: &hir::LowerResult,
    infer: &infer::InferenceResult,
) -> infer::Type {
    match item {
        hir::Item::Definition(hir::Definition::Variable { name, .. }) => infer
            .variable_types
            .get(hir.resolve(*name))
            .cloned()
            .unwrap_or(infer::Type::Integer),
        hir::Item::Definition(hir::Definition::Function { .. }) => infer::Type::Unit,
        hir::Item::Assignment { name, .. } => infer
            .variable_types
            .get(hir.resolve(*name))
            .cloned()
            .unwrap_or(infer::Type::Integer),
        hir::Item::IndexAssignment { .. } => infer::Type::Unit,
        hir::Item::Expression(expr) => get_expr_type(expr, hir, infer),
    }
}

fn get_expr_type(
    expr: &hir::Expression,
    hir: &hir::LowerResult,
    infer: &infer::InferenceResult,
) -> infer::Type {
    match expr {
        hir::Expression::Missing => infer::Type::Integer,
        hir::Expression::Literal(lit) => match lit {
            hir::Literal::Integer(_) => infer::Type::Integer,
            hir::Literal::Float(_) => infer::Type::Float,
            hir::Literal::Boolean(_) => infer::Type::Boolean,
            hir::Literal::String(_) => infer::Type::String,
        },
        hir::Expression::Infix { op, .. } => match op {
            hir::InfixOp::Add
            | hir::InfixOp::Sub
            | hir::InfixOp::Mul
            | hir::InfixOp::Div
            | hir::InfixOp::Mod => infer::Type::Integer,
            hir::InfixOp::AddFloat
            | hir::InfixOp::SubFloat
            | hir::InfixOp::MulFloat
            | hir::InfixOp::DivFloat => infer::Type::Float,
            _ => infer::Type::Boolean,
        },
        hir::Expression::Prefix { op, .. } => match op {
            hir::PrefixOp::Neg => infer::Type::Integer,
            hir::PrefixOp::Not => infer::Type::Boolean,
        },
        hir::Expression::VariableRef { name } => infer
            .variable_types
            .get(hir.resolve(*name))
            .cloned()
            .unwrap_or(infer::Type::Integer),
        hir::Expression::Block { tail, .. } => match tail {
            Some(idx) => infer
                .expression_types
                .get(*idx)
                .cloned()
                .unwrap_or(infer::Type::Unit),
            None => infer::Type::Unit,
        },
        hir::Expression::If {
            else_branch,
            then_branch,
            ..
        } => match else_branch {
            Some(_) => infer
                .expression_types
                .get(*then_branch)
                .cloned()
                .unwrap_or(infer::Type::Unit),
            None => infer::Type::Unit,
        },
        hir::Expression::Function { .. } => infer::Type::Unit,
        hir::Expression::Call { .. } => infer::Type::Unit,
        hir::Expression::Return { .. } => infer::Type::Unit,
        hir::Expression::Echo { .. } => infer::Type::Unit,
        hir::Expression::Loop { .. } => infer::Type::Unit,
        hir::Expression::While { .. } => infer::Type::Unit,
        hir::Expression::For { .. } => infer::Type::Unit,
        hir::Expression::Range { .. } => infer::Type::Integer,
        hir::Expression::Break { .. } => infer::Type::Unit,
        hir::Expression::Continue { .. } => infer::Type::Unit,
        // List expressions - type comes from inference
        hir::Expression::List { .. }
        | hir::Expression::Index { .. }
        | hir::Expression::Slice { .. } => infer::Type::Unit,
        // Tuple expressions - type comes from inference
        hir::Expression::Tuple { .. } | hir::Expression::TupleAccess { .. } => infer::Type::Unit,
    }
}

fn format_result(raw: i64, ty: Option<&infer::Type>, heap: &vm::Heap) -> String {
    match ty {
        Some(infer::Type::Float) => {
            let f = f64::from_bits(raw as u64);
            format!("{}", f)
        }
        Some(infer::Type::Boolean) => {
            format!("{}", raw != 0)
        }
        _ => {
            // Check if NaN-boxed (list or other heap value)
            let is_nan_boxed = (raw as u64 & 0xFFF8_0000_0000_0000) == 0xFFF8_0000_0000_0000;
            if is_nan_boxed {
                let value = vm::Value::from_bits(raw);
                value.display(heap).to_string()
            } else {
                format!("{}", raw)
            }
        }
    }
}

fn format_parse_errors(source: &str, errors: &[parse::ParseError]) -> String {
    use miette::{GraphicalReportHandler, GraphicalTheme, NamedSource, Report};

    let line_index = analyse::LineIndex::new(source);
    let handler = GraphicalReportHandler::new_themed(GraphicalTheme::unicode());
    let mut output = String::new();

    for error in errors {
        let (range, _severity, _code, message, _help) = analyse::to_lsp_fields(
            &analyse::AnalyseDiagnostic::Parse(error.clone()),
            &line_index,
        );
        let start = range.start.character as usize;
        let end = range.end.character as usize;
        let len = end.saturating_sub(start).max(1);

        let report: Report = miette::miette!(
            labels = vec![miette::LabeledSpan::at(start..start + len, &message)],
            "{}",
            message
        )
        .with_source_code(NamedSource::new("repl", source.to_string()));

        let _ = handler.render_report(&mut output, report.as_ref());
    }

    output
}

fn format_infer_errors(source: &str, diagnostics: &[infer::InferDiagnostic]) -> String {
    use miette::{GraphicalReportHandler, GraphicalTheme, NamedSource, Report};

    let line_index = analyse::LineIndex::new(source);
    let handler = GraphicalReportHandler::new_themed(GraphicalTheme::unicode());
    let mut output = String::new();

    for diag in diagnostics {
        let (range, _severity, _code, message, _help) = analyse::to_lsp_fields(
            &analyse::AnalyseDiagnostic::Infer(diag.clone()),
            &line_index,
        );
        let start = range.start.character as usize;
        let end = range.end.character as usize;
        let len = end.saturating_sub(start).max(1);

        let report: Report = miette::miette!(
            labels = vec![miette::LabeledSpan::at(start..start + len, &message)],
            "{}",
            message
        )
        .with_source_code(NamedSource::new("repl", source.to_string()));

        let _ = handler.render_report(&mut output, report.as_ref());
    }

    output
}

/// Run the REPL TUI.
pub fn run() -> io::Result<()> {
    use crossterm::cursor;
    use crossterm::terminal::{Clear, ClearType};
    use std::io::Write;

    let mut app = ReplApp::new();
    let mut stdout = io::stdout();

    enable_raw_mode()?;

    // Main loop
    loop {
        // Clear current line and render input
        stdout.execute(cursor::MoveToColumn(0))?;
        stdout.execute(Clear(ClearType::FromCursorDown))?;

        // Build and print input line with highlighting
        let prompt = if app.input.multi_line { "| " } else { "> " };
        print!("\x1b[34m{}\x1b[0m", prompt); // Blue prompt

        // Print highlighted input
        for span in highlight(&app.input.text) {
            let color = match span.style.fg {
                Some(Color::Yellow) => "\x1b[33m",
                Some(Color::Green) => "\x1b[32m",
                Some(Color::Magenta) => "\x1b[35m",
                Some(Color::Cyan) => "\x1b[36m",
                Some(Color::DarkGray) => "\x1b[90m",
                Some(Color::White) => "\x1b[37m",
                _ => "\x1b[0m",
            };
            print!("{}{}\x1b[0m", color, span.content);
        }

        // Always print diagnostic line (keeps input stable)
        // In raw mode, need explicit \r\n and cursor control
        print!("\r\n");
        stdout.execute(cursor::MoveToColumn(0))?;
        if let Some(ref analysis) = app.analysis {
            let diagnostics = analysis.diagnostics();
            if !diagnostics.is_empty() {
                // Print underlines - start with prompt offset
                let mut diag_chars: Vec<char> = " ".repeat(prompt.len()).chars().collect();
                let line_index = &analysis.line_index;
                for diag in &diagnostics {
                    let (range, _severity, _code, _message, _help) =
                        analyse::to_lsp_fields(diag, line_index);
                    let start = range.start.character as usize;
                    let end = range.end.character as usize;
                    // Extend if needed
                    while diag_chars.len() < prompt.len() + end {
                        diag_chars.push(' ');
                    }
                    // Fill underlines at correct positions
                    for i in start..end {
                        diag_chars[prompt.len() + i] = '^';
                    }
                }
                let diag_line: String = diag_chars.into_iter().collect();
                print!("\x1b[31m{}\x1b[0m", diag_line);

                if let Some(diag) = diagnostics.first() {
                    print!(" {}", diag);
                }
            }
        }
        // Move cursor back up for input
        stdout.execute(cursor::MoveUp(1))?;

        // Position cursor
        let cursor_pos = prompt.len() + app.input.cursor;
        stdout.execute(cursor::MoveToColumn(cursor_pos as u16))?;
        stdout.flush()?;

        // Wait for input
        if let Event::Key(key) = event::read()? {
            let was_execute = matches!(
                (key.modifiers, key.code),
                (KeyModifiers::NONE, KeyCode::Enter)
            );

            app.handle_key(key);

            // After execution, print the result to terminal history
            if was_execute && !app.session.history.is_empty() {
                // Clear the input line first
                stdout.execute(cursor::MoveToColumn(0))?;
                stdout.execute(Clear(ClearType::FromCursorDown))?;

                // Disable raw mode for clean multi-line output
                disable_raw_mode()?;

                let entry = app.session.history.last().unwrap();

                // Print the executed input
                print!("\x1b[34m> \x1b[0m");
                for span in highlight(&entry.input) {
                    let color = match span.style.fg {
                        Some(Color::Yellow) => "\x1b[33m",
                        Some(Color::Green) => "\x1b[32m",
                        Some(Color::Magenta) => "\x1b[35m",
                        Some(Color::Cyan) => "\x1b[36m",
                        Some(Color::DarkGray) => "\x1b[90m",
                        Some(Color::White) => "\x1b[37m",
                        _ => "\x1b[0m",
                    };
                    print!("{}{}\x1b[0m", color, span.content);
                }
                println!();

                // Print output
                if let Some(ref output) = entry.output {
                    if entry.is_error {
                        print!("{}", output); // miette already has colors
                    } else {
                        println!("\x1b[32m{}\x1b[0m", output); // Green for results
                    }
                }

                // Re-enable raw mode for input
                enable_raw_mode()?;
            }
        }

        if app.should_quit {
            // Clear input line before exit
            stdout.execute(cursor::MoveToColumn(0))?;
            stdout.execute(Clear(ClearType::CurrentLine))?;
            break;
        }
    }

    disable_raw_mode()?;
    println!(); // Final newline

    Ok(())
}
