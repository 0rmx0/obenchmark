use egui::RichText;
use crossbeam_channel::{unbounded, Receiver};
use chrono::Local;

use crate::{
    engines::runner::{run_benchmarks, RunnerEvent},
    benchmarks::{cpu::CpuBenchmark, memory::MemoryBenchmark, disk::DiskBenchmark},
    util::sysinfo::get_system_info,
    app::state::AppState,
};

pub struct OBenchmarkApp {
    state: AppState,
    receiver: Option<Receiver<RunnerEvent>>,
}

impl OBenchmarkApp {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        crate::app::theme::apply_ui_theme(&cc.egui_ctx);

        Self { state: AppState::Idle, receiver: None }
    }
}

impl eframe::App for OBenchmarkApp {
    fn update(&mut self, ctx: &egui::Context, _: &mut eframe::Frame) {
        let mut should_restart = false;

        if let Some(rx) = &self.receiver {
            while let Ok(event) = rx.try_recv() {
                match event {
                    RunnerEvent::BenchStarted(name) => {
                        let completed = match &self.state {
                            AppState::Running { completed, .. } => *completed,
                            _ => 0,
                        };
                        self.state = AppState::Running {
                            current_test: name.clone(),
                            completed,
                            total: 3,
                        };
                    }
                    RunnerEvent::BenchFinished(_, _) => {
                        if let AppState::Running { completed, .. } = &self.state {
                            let new_completed = completed + 1;
                            if let AppState::Running { current_test, total, .. } = &self.state {
                                self.state = AppState::Running {
                                    current_test: current_test.clone(),
                                    completed: new_completed,
                                    total: *total,
                                };
                            }
                        }
                    }
                    RunnerEvent::Done(result) => {
                        self.state = AppState::Showing(result.clone());
                    }
                    RunnerEvent::Error(e) => {
                        self.state = AppState::Error(e);
                    }
                }
            }
        }

        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("OBenchmark");
            ui.separator();

            match &self.state {
                AppState::Idle => {
                    if ui.button("Start Benchmark").clicked() {
                        let (tx, rx) = unbounded();

                        run_benchmarks(
                            vec![
                                Box::new(CpuBenchmark),
                                Box::new(MemoryBenchmark),
                                Box::new(DiskBenchmark),
                            ],
                            tx,
                        );

                        self.receiver = Some(rx);
                    }
                }

                AppState::Running { current_test, completed, total } => {
                    ui.label(RichText::new(format!("Test en cours: {}", current_test)).size(18.0).strong());
                    ui.separator();
                    
                    // Barre de progression du test actuel
                    ui.label("Progression du test:");
                    ui.add(egui::ProgressBar::new(0.5).show_percentage());
                    
                    ui.separator();
                    
                    // Barre de progression globale
                    let global_progress = *completed as f32 / *total as f32;
                    ui.label(format!("Tests: {}/{}", completed, total));
                    ui.add(egui::ProgressBar::new(global_progress).show_percentage());
                }

                AppState::Showing(result) => {
                    ui.label(RichText::new(format!("Score final: {}", result.final_score)).size(32.0).strong());
                    ui.separator();

                    ui.label(RichText::new("Détail des scores:").size(18.0).strong());
                    for score in &result.scores {
                        ui.horizontal(|ui| {
                            ui.label(format!("{}:", score.name));
                            ui.label(RichText::new(format!("{}", score.raw_score)).strong());
                        });
                    }

                    ui.separator();
                    ui.label("System Info");

                    let sys = get_system_info();

                    ui.label(format!("CPU: {}", sys.global_cpu_info().brand()));
                    ui.label(format!("Cores: {}", sys.cpus().len()));
                    ui.label(format!("RAM: {} MB", sys.total_memory() / 1024));
                    ui.label(format!("OS: {:?}", sysinfo::System::name()));

                    ui.horizontal(|ui| {
                        if ui.button("Export Result JSON").clicked() {
                            let json = serde_json::to_string_pretty(&result).unwrap();
                            std::fs::write(format!("bench_{}.json", Local::now().timestamp()), json).ok();
                        }

                        if ui.button("🔄 New Analysis").clicked() {
                            should_restart = true;
                        }
                    });
                }

                AppState::Error(err) => {
                    ui.colored_label(egui::Color32::RED, err);
                }
            }
        });

        if should_restart {
            let (tx, rx) = unbounded();

            run_benchmarks(
                vec![
                    Box::new(CpuBenchmark),
                    Box::new(MemoryBenchmark),
                    Box::new(DiskBenchmark),
                ],
                tx,
            );

            self.receiver = Some(rx);
            self.state = AppState::Idle;
        }

        ctx.request_repaint_after(std::time::Duration::from_millis(16));
    }
}