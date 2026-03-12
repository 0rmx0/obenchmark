mod app;
mod benchmarks;
mod cli;
mod engines;
mod model;
mod util;

use crate::cli::{CliArgs, CliCommand};
use clap::Parser;
use iced::{Application, Settings};

fn main() -> iced::Result {
    let args = CliArgs::parse();
    if let Some(CliCommand::Cli(opts)) = args.command {
        if let Err(err) = cli::run_cli(opts) {
            eprintln!("error: {}", err);
            std::process::exit(1);
        }
        return Ok(());
    }

    let settings = Settings {
        window: iced::window::Settings {
            size: iced::Size::new(960.0, 720.0),
            ..Default::default()
        },
        ..Default::default()
    };

    app::ui::OBenchmarkApp::run(settings)
}
