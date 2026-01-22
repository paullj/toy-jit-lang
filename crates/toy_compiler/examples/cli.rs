use std::path::PathBuf;

use clap::{Parser, ValueEnum};
use miette::Result;
use toy_compiler::{CompileOptions, compile, options::CompileProfile};

#[derive(Parser, Debug)]
#[command(name = "toyc", version, about = "toy language compiler")]
struct Args {
    #[arg(
        value_name = "PATH",
        help = "Input path to compile, can be a .toy file, a directory, or project manifest"
    )]
    path: PathBuf,
    #[arg(
        value_enum,
        long,
        default_value = "debug",
        help = "Compilation profile"
    )]
    profile: Profile,
    #[arg(
        value_enum,
        long,
        help = "Compilation mode, auto-detects if not specified"
    )]
    mode: Option<Mode>,
    #[arg(
        value_enum,
        long,
        help = "Target platform for the compilation, auto-detects if not specified"
    )]
    target: Option<Target>,
}

#[derive(Copy, Clone, ValueEnum, Debug)]
enum Profile {
    Debug,
    Release,
}

#[derive(Copy, Clone, ValueEnum, Debug)]
enum Mode {
    Script,
    Project,
}

#[derive(Copy, Clone, ValueEnum, Debug)]
enum Target {
    Linux,
    Windows,
    MacOS,
    Wasm,
}

impl From<Profile> for CompileProfile {
    fn from(profile: Profile) -> Self {
        match profile {
            Profile::Debug => CompileProfile::Debug,
            Profile::Release => CompileProfile::Release,
        }
    }
}

fn main() -> Result<()> {
    toy_logger::init(log::LevelFilter::Trace);
    let args = Args::parse();
    log::debug!("Parsed arguments: {:#?}", args);
    let options = CompileOptions::new(args.path).with_profile(args.profile.into());

    let results = compile(options)?;
    log::info!("Compiled {} file(s)", results.len());
    Ok(())
}
