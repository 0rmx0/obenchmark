//! Point d’entrée de l’application.
//! Initialise le logger, crée la fenêtre eGUI et démarre l’interface.

use eframe::{egui, NativeOptions};

mod gui;
mod benchmark_runner;
mod benchmarks;
mod utils;

fn main() -> Result<(), eframe::Error> {
    // Initialise le logger (utile pour le debug GPU)
    env_logger::init();

    let native_options = NativeOptions {
        initial_window_size: Some(egui::vec2(800.0, 600.0)),
        ..Default::default()
    };

    eframe::run_native(
        "PerformanceTest – Benchmark multi‑plateforme",
        native_options,
        Box::new(|cc| Box::new(gui::App::new(cc))),
    )
}
