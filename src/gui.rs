use crate::benchmarks::{cpu::CpuBenchmark, memory::MemoryBenchmark, disk::DiskBenchmark, Benchmark};
use crate::runner::{run_async, BenchResult};
use eframe::egui;
use std::sync::mpsc::Receiver;

pub struct OBenchmarkApp {
    running: bool,
    progress: f32,
    receiver: Option<Receiver<anyhow::Result<BenchResult>>>,
    result: Option<BenchResult>,
}

impl Default for OBenchmarkApp {
    fn default() -> Self {
        Self {
            running: false,
            progress: 0.0,
            receiver: None,
            result: None,
        }
    }
}

impl eframe::App for OBenchmarkApp {
    fn update(&mut self, ctx: &egui::Context, _: &mut eframe::Frame) {

        egui::TopBottomPanel::top("top_panel").show(ctx, |ui| {
            ui.heading("🚀 OBenchmark Professional");
        });

        egui::CentralPanel::default().show(ctx, |ui| {

            ui.add_space(20.0);

            if !self.running {
                if ui
                    .add_sized([200.0, 50.0], egui::Button::new("Start Benchmark"))
                    .clicked()
                {
                    let benches: Vec<Box<dyn Benchmark + Send>> = vec![
                        Box::new(CpuBenchmark::new()),
                        Box::new(MemoryBenchmark),
                        Box::new(DiskBenchmark),
                    ];

                    self.receiver = Some(run_async(benches));
                    self.running = true;
                    self.progress = 0.0;
                    self.result = None;
                }
            } else {
                ui.label("Benchmark running...");
                self.progress += 0.002;
                if self.progress > 1.0 {
                    self.progress = 1.0;
                }

                ui.add(egui::ProgressBar::new(self.progress).animate(true));
            }

            if let Some(rx) = &self.receiver {
                if let Ok(res) = rx.try_recv() {
                    self.running = false;
                    self.progress = 1.0;

                    match res {
                        Ok(r) => self.result = Some(r),
                        Err(e) => eprintln!("Error: {:?}", e),
                    }
                }
            }

            ui.add_space(30.0);

            if let Some(result) = &self.result {
                ui.separator();
                ui.heading("Results");

                for (name, score) in &result.scores {
                    ui.horizontal(|ui| {
                        ui.label(name);
                        ui.label(format!("{}", score));
                    });
                }

                ui.separator();
                ui.heading(format!("Global Score: {}", result.global_score));
            }
        });

        ctx.request_repaint();
    }
}