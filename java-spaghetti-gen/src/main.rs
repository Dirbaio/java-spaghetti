// must go first because macros.
mod util;

mod config;
mod emit_rust;
mod identifiers;
mod run;

fn main() {
    entry::main();
}

mod entry {
    use std::path::PathBuf;

    use clap::{Parser, Subcommand};

    use crate::config;
    use crate::run::run;

    /// Autogenerate jni-android-sys, glue code for access Android JVM APIs from Rust
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
                let config_file = config::toml::File::from_directory(&cmd.directory).unwrap();
                run(config_file).unwrap();
            }
        }
    }
}
