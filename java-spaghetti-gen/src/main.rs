// this must go first because of macros.
mod util;

mod config;
mod emit;
mod identifiers;
mod parser_util;

use std::fs::File;
use std::io::{self, Read};
use std::path::{Path, PathBuf};

use clap::{Parser, Subcommand};

use crate::config::Config;
use crate::parser_util::JavaClass;

/// The core function of this library: Generate Rust code to access Java APIs.
pub fn run(config: impl Into<Config>) -> Result<(), anyhow::Error> {
    let config: Config = config.into();
    println!("output: {}", config.output.display());

    let mut context = emit::Context::new(&config);
    for file in config.input.iter() {
        gather_file(&mut context, file)?;
    }

    let mut out = Vec::with_capacity(4096);
    context.write(&mut out)?;
    util::write_generated(&context, &config.output, &out[..])?;

    // Generate Java proxy files if proxy_output is specified
    if let Some(proxy_output) = &config.proxy_output {
        emit::java_proxy::write_java_proxy_files(&context, proxy_output)?;
    }

    Ok(())
}

fn gather_file(context: &mut emit::Context, path: &Path) -> Result<(), anyhow::Error> {
    let verbose = context.config.logging_verbose;

    context
        .progress
        .lock()
        .unwrap()
        .update(format!("reading {}...", path.display()).as_str());

    let ext = if let Some(ext) = path.extension() {
        ext
    } else {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "Input files must have an extension",
        ))?;
    };

    match ext.to_string_lossy().to_ascii_lowercase().as_str() {
        "class" => {
            let class = JavaClass::read(std::fs::read(path)?)?;
            context.add_class(class)?;
        }
        "jar" => {
            let mut jar = zip::ZipArchive::new(io::BufReader::new(File::open(path)?))?;
            let n = jar.len();

            for i in 0..n {
                let mut file = jar.by_index(i)?;
                if !file.name().ends_with(".class") {
                    continue;
                }

                if verbose {
                    context
                        .progress
                        .lock()
                        .unwrap()
                        .update(format!("  reading {:3}/{}: {}...", i, n, file.name()).as_str());
                }

                let mut buf = Vec::new();
                file.read_to_end(&mut buf)?;
                let class = JavaClass::read(buf)?;
                context.add_class(class)?;
            }
        }
        unknown => {
            Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                format!("Input files must have a '.class' or '.jar' extension, not a '.{unknown}' extension",),
            ))?;
        }
    }
    Ok(())
}

/// Autogenerate glue code for access Android JVM APIs from Rust
#[derive(Parser, Debug)]
#[command(version, about)]
struct Cli {
    #[command(subcommand)]
    cmd: Cmd,
}

/// Doc comment
#[derive(Subcommand, Debug)]
enum Cmd {
    Generate(GenerateCmd),
}

#[derive(Parser, Debug)]
struct GenerateCmd {
    /// Log in more detail
    #[arg(short, long)]
    verbose: bool,

    /// Sets a custom config file
    #[arg(short, long)]
    config: Option<PathBuf>,
}

pub fn main() {
    let cli = Cli::parse();

    match cli.cmd {
        Cmd::Generate(cmd) => {
            let mut config = if let Some(config_path) = cmd.config {
                // Use specified config file
                config::Config::from_file(&config_path).unwrap()
            } else {
                // Search from current working directory
                config::Config::from_current_directory().unwrap()
            };

            if cmd.verbose {
                config.logging_verbose = true;
            }
            run(config).unwrap();
        }
    }
}
