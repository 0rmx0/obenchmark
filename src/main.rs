mod runner;
mod benchmarks;
mod report;
mod gui;

fn main() -> eframe::Result<()> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([900.0, 600.0])
            .with_resizable(true),
        ..Default::default()
    };

    eframe::run_native(
        "OBenchmark Professional",
        options,
        Box::new(|_cc| Box::new(gui::OBenchmarkApp::default())),
    )
}