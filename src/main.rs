mod app;
mod benchmarks;
mod cli;
mod engines;
mod model;
mod util;

use crate::cli::{CliArgs, CliCommand};
use clap::Parser;

fn main() {
    #[cfg(unix)]
    util::cleanup::install_signal_handlers();
    util::cleanup::cleanup_stale_temp_files();

    let args = CliArgs::parse();
    match args.command {
        Some(CliCommand::Cli(opts)) => {
            if let Err(err) = cli::run_cli(opts) {
                eprintln!("error: {}", err);
                std::process::exit(1);
            }
        }
        Some(CliCommand::History) => {
            if let Err(err) = cli::run_history() {
                eprintln!("error: {}", err);
                std::process::exit(1);
            }
        }
        Some(CliCommand::Compare { file_a, file_b }) => {
            if let Err(err) = cli::run_compare(&file_a, &file_b) {
                eprintln!("error: {}", err);
                std::process::exit(1);
            }
        }
        None => {
            if let Err(err) = app::ui::run_gui() {
                eprintln!("error: {}", err);
                std::process::exit(1);
            }
        }
    }
}
