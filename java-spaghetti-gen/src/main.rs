// this must go first because of macros.
mod util;

mod config;
mod emit_rust;
mod identifiers;
mod parser_util;
mod run;

fn main() {
    entry::main();
}

mod entry {
    use std::path::PathBuf;

    use clap::{Parser, Subcommand};

    use crate::config;
    use crate::run::run;

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

        /// Sets a custom directory
        #[arg(short, long, default_value = ".")]
        directory: PathBuf,
    }

    pub fn main() {
        let cli = Cli::parse();

        match cli.cmd {
            Cmd::Generate(cmd) => {
                let mut config = config::Config::from_directory(&cmd.directory).unwrap();
                if cmd.verbose {
                    config.logging_verbose = true;
                }
                run(config).unwrap();
            }
        }
    }
}
