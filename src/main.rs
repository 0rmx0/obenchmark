mod app;
mod benchmarks;
mod cli;
mod engines;
mod model;
mod util;

use crate::cli::{CliArgs, CliCommand};
use clap::Parser;

fn main() {
    let args = CliArgs::parse();
    if let Some(CliCommand::Cli(opts)) = args.command {
        if let Err(err) = cli::run_cli(opts) {
            eprintln!("error: {}", err);
            std::process::exit(1);
        }
        return;
    }

    if let Err(err) = app::ui::run_gui() {
        eprintln!("error: {}", err);
        std::process::exit(1);
    }
}
