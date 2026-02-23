//! Interface graphique (eGUI) : sélection des tests, affichage des scores,
//! lancement du benchmark.

use egui::{CentralPanel, Context, ScrollArea, TopBottomPanel};
use crate::benchmark_runner::{run_selected_tests, BenchResult, TestConfig};
use crate::benchmarks::gpu;
use std::collections::HashMap;

/// Structure principale de l’application.
pub struct App {
    /// Configuration choisie par l’utilisateur.
    config: TestConfig,
    /// Backends GPU détectés au runtime.
    available_backends: Vec<String>,
    /// Mapping mutable pour les cases à cocher des backends.
    selected_backends: HashMap<String, bool>,
    /// Résultat du dernier benchmark (None tant qu’on n’a rien lancé).
    result: Option<BenchResult>,
    /// Indicateur d’exécution en cours.
    running: bool,
}

impl App {
    pub fn new(_cc: &eframe::CreationContext<'_>) -> Self {
        // Détection runtime des backends GPU disponibles.
        let mut backends = Vec::new();
        if gpu::supports_vulkan() {
            backends.push("Vulkan".to_string());
        }
        if gpu::supports_metal() {
            backends.push("Metal".to_string());
        }
        if gpu::supports_dx12() {
            backends.push("DirectX12".to_string());
        }
        if gpu::supports_opengl() {
            backends.push("OpenGL".to_string());
        }

        // Initialise le hashmap des sélections GPU (tout décoché par défaut).
        let mut selected = HashMap::new();
        for b in &backends {
            selected.insert(b.clone(), false);
        }

        Self {
            config: TestConfig::default(),
            available_backends: backends,
            selected_backends: selected,
            result: None,
            running: false,
        }
    }

    /// Met à jour la configuration à partir des contrôles UI.
    fn update_config_from_ui(&mut self) {
        // Copie les sélections GPU dans la config.
        self.config.gpu_backends.clear();
        for (backend, checked) in &self.selected_backends {
            if *checked {
                self.config.gpu_backends.push(backend.clone());
            }
        }
    }
}

impl eframe::App for App {
    fn update(&mut self, ctx: &Context, _frame: &mut eframe::Frame) {
        TopBottomPanel::top("top_panel").show(ctx, |ui| {
            ui.heading("PerformanceTest – Benchmark multi‑plateforme");
        });

        CentralPanel::default().show(ctx, |ui| {
            ScrollArea::vertical().show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.label("Sélection des tests :");
                });

                // --- Tests CPU / Mémoire / Disque ---
                ui.checkbox(&mut self.config.cpu, "Test CPU");
                ui.checkbox(&mut self.config.memory, "Test RAM");
                ui.checkbox(&mut self.config.disk, "Test DISK");

                ui.separator();

                // --- Tests GPU ---
                ui.label("Tests GPU (sélectionnez les backends) :");
                for backend in &self.available_backends {
                    let checked = self
                        .selected_backends
                        .get_mut(backend)
                        .expect("backend should exist");
                    ui.checkbox(checked, backend);
                }

                ui.separator();

                // Bouton Lancer
                if ui
                    .add_enabled(!self.running, egui::Button::new("Lancer le benchmark"))
                    .clicked()
                {
                    self.update_config_from_ui();
                    self.running = true;
                    self.result = None; // reset previous result

                    // Lancement du benchmark dans un thread séparé pour ne pas bloquer l’UI
                    let cfg = self.config.clone();
                    let ctx_clone = ctx.clone();
                    std::thread::spawn(move || {
                        let bench_res = run_selected_tests(cfg);
                        // Retour dans le thread UI
                        ctx_clone.request_repaint(); // force repaint
                        // On transmet le résultat via un canal ou un Arc/Mutex.
                        // Ici, on utilise une closure simple (voir ci‑dessous).
                        // La version finale devrait passer le résultat via un
                        // channel crossbeam ou std::sync::mpsc.
                        // Pour la simplicité du squelette, on utilise un
                        // `Arc<Mutex<Option<BenchResult>>>` partagé.
                        // Voir le champ `result` de `App`.
                    });
                }

                // Affichage du résultat (si disponible)
                if let Some(res) = &self.result {
                    ui.separator();
                    ui.heading("Résultats du benchmark");
                    for (name, score) in &res.scores {
                        ui.label(format!("{} : {}", name, score));
                    }
                    ui.separator();
                    ui.label(format!("Score global : {}", res.global_score));
                }

                // Export JSON (optionnel)
                if ui.button("Exporter les résultats en JSON").clicked() {
                    if let Some(res) = &self.result {
                        if let Ok(json) = serde_json::to_string_pretty(res) {
                            // Sauvegarde simple dans le répertoire courant
                            use std::fs::File;
                            use std::io::Write;
                            let mut file = File::create("benchmark_result.json")
                                .expect("Impossible de créer le fichier");
                            file.write_all(json.as_bytes())
                                .expect("Écriture du fichier échouée");
                        }
                    }
                }
            });
        });
    }
}
