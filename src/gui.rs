use crate::benchmarks::{cpu::CpuBenchmark, memory::MemoryBenchmark, disk::DiskBenchmark, Benchmark};
use crate::runner::{run_async, BenchResult, ProgressUpdate};
use eframe::egui;
use std::sync::mpsc::{self, Receiver};

pub struct OBenchmarkApp {
    running: bool,
    receiver: Option<Receiver<anyhow::Result<BenchResult>>>,
    result: Option<BenchResult>,

    // Console
    console_output: String,
    console_rx: Option<Receiver<String>>,

    // Progression
    global_progress: f32,
    step_progress: f32,
    current_step: String,
    progress_rx: Option<Receiver<ProgressUpdate>>,
}

impl Default for OBenchmarkApp {
    fn default() -> Self {
        Self {
            running: false,
            receiver: None,
            result: None,

            console_output: String::new(),
            console_rx: None,

            global_progress: 0.0,
            step_progress: 0.0,
            current_step: "Aucun".into(),
            progress_rx: None,
        }
    }
}

impl eframe::App for OBenchmarkApp {
    fn update(&mut self, ctx: &egui::Context, _: &mut eframe::Frame) {
        egui::TopBottomPanel::top("top_panel").show(ctx, |ui| {
            ui.heading("🚀 OBenchmark Professional");
        });

        egui::CentralPanel::default().show(ctx, |ui| {
            ui.add_space(10.0);

            ui.horizontal(|ui| {
                if !self.running {
                    if ui.button("▶ Start Benchmark").clicked() {
                        // Préparer les benches
                        let benches: Vec<Box<dyn Benchmark + Send>> = vec![
                            Box::new(CpuBenchmark::new()),
                            Box::new(MemoryBenchmark),
                            Box::new(DiskBenchmark),
                        ];

                        // Console
                        let (log_tx, log_rx) = mpsc::channel::<String>();
                        self.console_output.clear();
                        self.console_rx = Some(log_rx);

                        // Progress
                        let (progress_tx, progress_rx) = mpsc::channel::<ProgressUpdate>();
                        self.progress_rx = Some(progress_rx);
                        self.global_progress = 0.0;
                        self.step_progress = 0.0;
                        self.current_step = "Démarrage".into();

                        // Lancer
                        self.receiver = Some(run_async(benches, log_tx, progress_tx));
                        self.running = true;
                        self.result = None;
                    }
                }

                if ui.button("🧹 Clear console").clicked() {
                    self.console_output.clear();
                }
            });

            ui.add_space(10.0);

            // Barres de progression
            ui.label(format!("Étape en cours : {}", self.current_step));
            ui.add(egui::ProgressBar::new(self.step_progress).animate(true));

            ui.label("Progression globale :");
            ui.add(egui::ProgressBar::new(self.global_progress).animate(true));

            // Récupérer progression
            if let Some(rx) = &self.progress_rx {
                while let Ok(msg) = rx.try_recv() {
                    match msg {
                        ProgressUpdate::Global(v) => self.global_progress = v,
                        ProgressUpdate::Step(v, name) => {
                            self.step_progress = v;
                            self.current_step = name;
                        }
                        ProgressUpdate::StepStart(name) => {
                            self.current_step = name;
                            self.step_progress = 0.0;
                        }
                        ProgressUpdate::StepEnd(_) => {
                            self.step_progress = 1.0;
                        }
                    }
                }
            }

            // Logs
            if let Some(log_rx) = &self.console_rx {
                while let Ok(line) = log_rx.try_recv() {
                    self.console_output.push_str(&line);
                    self.console_output.push('\n');
                }
            }

            ui.separator();
            ui.heading("Console Output");

            egui::ScrollArea::vertical()
                .stick_to_bottom(true)
                .max_height(200.0)
                .show(ui, |ui| {
                    ui.monospace(&self.console_output);
                });

            // Résultats finaux
            if let Some(r) = &self.result {
                ui.separator();
                ui.heading("Résultats :");
                for (name, score) in &r.scores {
                    ui.label(format!("{} : {}", name, score));
                }
                ui.separator();
                ui.heading(format!("Score global : {}", r.global_score));
            }

            // Réception du résultat final
            if let Some(rx) = &self.receiver {
                if let Ok(res) = rx.try_recv() {
                    self.running = false;
                    self.step_progress = 1.0;
                    self.global_progress = 1.0;

                    match res {
                        Ok(r) => self.result = Some(r),
                        Err(e) => {
                            self.console_output
                                .push_str(&format!("Erreur : {:?}\n", e));
                        }
                    }
                }
            }
        });

        ctx.request_repaint();
    }
}