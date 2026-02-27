mod app;
mod engines;
mod benchmarks;
mod model;
mod util;

use app::ui::OBenchmarkApp;

fn main() -> eframe::Result<()> {
    let options = eframe::NativeOptions::default();
    eframe::run_native("OBenchmark", options, Box::new(|cc| Box::new(OBenchmarkApp::new(cc))))
}