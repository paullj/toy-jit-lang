use std::{path::PathBuf, process::ExitCode};

use clap::{
    Parser, Subcommand,
    builder::styling::{AnsiColor, Color, Style},
};
use lex::lex;
use parse::parse;

/// a simple interpreter
#[derive(Parser, Debug)]
#[command(version, about, long_about = None, styles=get_styles())]
struct Arguments {
    /// Path of file to run, if not provided will run the REPL
    #[arg(value_name = "PATH", index = 1)]
    path: Option<PathBuf>,

    #[cfg(debug_assertions)]
    #[command(subcommand)]
    cmd: Option<Commands>,
}

#[derive(Subcommand, Debug, Clone)]
enum Commands {
    /// Lex source into tokens
    #[command(name = "lex")]
    Lex { source: String },
    /// Parse source into an AST
    #[command(name = "parse")]
    Parse { source: String },
}

fn main() -> ExitCode {
    let args = Arguments::parse();

    // Commands for testing lexing and parsing
    #[cfg(debug_assertions)]
    {
        println!("Running in debug mode");
        if let Some(cmd) = &args.cmd {
            match cmd {
                Commands::Lex { source } => {
                    let contents = get_source_contents(source);

                    for result in lex(contents.as_str()).iter() {
                        match result {
                            Ok(token) => println!("{}", token),
                            Err(err) => eprintln!("error: {:?}", err),
                        }
                    }
                }
                Commands::Parse { source } => {
                    let contents = get_source_contents(source);
                    let (tree, _errors) = parse(contents.as_str());
                    println!("{:#?}", tree);
                }
            }
            return ExitCode::SUCCESS;
        }
    }

    match args.path {
        Some(path) => {
            eprintln!("Run {:?}", path)
        }
        None => {
            eprintln!("Run REPL")
        }
    }
    return ExitCode::SUCCESS;
}

fn get_source_contents(source: &str) -> String {
    if PathBuf::from(source).exists() {
        std::fs::read_to_string(source).unwrap()
    } else {
        source.to_string()
    }
}

fn get_styles() -> clap::builder::Styles {
    clap::builder::Styles::styled()
        .usage(title(AnsiColor::BrightWhite))
        .header(title(AnsiColor::BrightWhite))
        .literal(colored(AnsiColor::Cyan))
        .invalid(bold(AnsiColor::Red))
        .error(bold(AnsiColor::Red))
        .valid(title(AnsiColor::Green))
        .placeholder(colored(AnsiColor::White))
}

fn title(color: AnsiColor) -> Style {
    Style::new()
        .bold()
        .underline()
        .fg_color(Some(Color::Ansi(color)))
}

fn bold(color: AnsiColor) -> Style {
    Style::new().bold().fg_color(Some(Color::Ansi(color)))
}

fn colored(color: AnsiColor) -> Style {
    Style::new().fg_color(Some(Color::Ansi(color)))
}
