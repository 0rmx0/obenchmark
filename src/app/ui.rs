use chrono::Local;
use crossbeam_channel::{unbounded, Receiver};
use slint::{ModelRc, SharedString, VecModel};

use std::cell::{Cell, RefCell};
use std::collections::HashSet;
use std::path::PathBuf;
use std::rc::Rc;
use std::time::Duration;

use crate::{
    app::state::AppState,
    benchmarks::suite::{self, BenchSpec},
    engines::benchmark::bench_meta,
    engines::runner::{run_benchmarks, RunnerEvent},
    model::history,
    model::result::BenchResult,
    util::format::human_bytes,
    util::report::render_html,
    util::sysinfo::get_system_info,
};

slint::include_modules!();

fn results_dir() -> PathBuf {
    let dir = dirs::data_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("obenchmark")
        .join("results");
    let _ = std::fs::create_dir_all(&dir);
    dir
}

pub fn run_gui() -> Result<(), slint::PlatformError> {
    let ui = MainWindow::new()?;

    let state = Rc::new(RefCell::new(AppState::Idle));
    let receiver: Rc<RefCell<Option<Receiver<RunnerEvent>>>> = Rc::new(RefCell::new(None));
    let recorded = Rc::new(Cell::new(false));

    // Benchmark selection (all enabled by default).
    let selected: Rc<RefCell<HashSet<String>>> = Rc::new(RefCell::new(
        suite::AVAILABLE_BENCHMARKS
            .iter()
            .map(|s| s.name.to_string())
            .collect(),
    ));
    ui.set_bench_list(build_bench_list(&selected.borrow(), true));

    let timer = slint::Timer::default();
    {
        let ui_weak = ui.as_weak();
        let state_tick = state.clone();
        let receiver_tick = receiver.clone();
        let recorded_tick = recorded.clone();
        timer.start(
            slint::TimerMode::Repeated,
            Duration::from_millis(40),
            move || {
                let Some(ui) = ui_weak.upgrade() else {
                    return;
                };

                let mut events = Vec::new();
                if let Some(rx) = receiver_tick.borrow().as_ref() {
                    while let Ok(event) = rx.try_recv() {
                        events.push(event);
                    }
                }

                if events.is_empty() {
                    return;
                }

                let mut current = state_tick.borrow().clone();
                for event in events {
                    current = apply_event(current, event);
                }
                if let AppState::Showing(result) = &current {
                    if !recorded_tick.get() {
                        let _ = history::append_history(result);
                        recorded_tick.set(true);
                    }
                }
                *state_tick.borrow_mut() = current.clone();
                apply_state_to_ui(&ui, &current);
            },
        );
    }

    {
        let ui_weak = ui.as_weak();
        let state_start = state.clone();
        let receiver_start = receiver.clone();
        let selected_start = selected.clone();
        let recorded_start = recorded.clone();
        ui.on_start_clicked(move || {
            let sel = selected_start.borrow();
            let specs: Vec<BenchSpec> = if sel.is_empty() {
                suite::default_suite()
            } else {
                suite::AVAILABLE_BENCHMARKS
                    .iter()
                    .copied()
                    .filter(|s| sel.contains(s.name))
                    .collect()
            };
            drop(sel);
            if specs.is_empty() {
                return;
            }
            let total = specs.len();

            *state_start.borrow_mut() = AppState::Running {
                current_test: String::new(),
                completed: 0,
                total,
                current_progress: 0.0,
                failed: Vec::new(),
                last_result: String::new(),
            };
            recorded_start.set(false);
            let (tx, rx) = unbounded();
            *receiver_start.borrow_mut() = Some(rx);

            if let Some(ui) = ui_weak.upgrade() {
                apply_state_to_ui(&ui, &state_start.borrow());
            }

            std::thread::spawn(move || {
                run_benchmarks(specs, tx);
            });
        });
    }

    {
        let selected_toggle = selected.clone();
        ui.on_bench_toggled(move |name, checked| {
            let mut sel = selected_toggle.borrow_mut();
            if checked {
                sel.insert(name.to_string());
            } else {
                sel.remove(name.as_str());
            }
        });
    }

    {
        let ui_weak = ui.as_weak();
        let selected_all = selected.clone();
        ui.on_select_all_clicked(move || {
            let mut sel = selected_all.borrow_mut();
            *sel = suite::AVAILABLE_BENCHMARKS
                .iter()
                .map(|s| s.name.to_string())
                .collect();
            if let Some(ui) = ui_weak.upgrade() {
                ui.set_bench_list(build_bench_list(&sel, true));
            }
        });
    }

    {
        let ui_weak = ui.as_weak();
        let selected_none = selected.clone();
        ui.on_select_none_clicked(move || {
            let mut sel = selected_none.borrow_mut();
            sel.clear();
            if let Some(ui) = ui_weak.upgrade() {
                ui.set_bench_list(build_bench_list(&sel, false));
            }
        });
    }

    {
        let state_export = state.clone();
        let ui_weak_export = ui.as_weak();
        ui.on_export_clicked(move || {
            let Some(ui) = ui_weak_export.upgrade() else {
                return;
            };
            if let AppState::Showing(result) = &*state_export.borrow() {
                let path = results_dir().join(format!(
                    "bench_{}.json",
                    Local::now().format("%Y%m%d_%H%M%S")
                ));
                let res = serde_json::to_string_pretty(result)
                    .map_err(anyhow::Error::from)
                    .and_then(|json| std::fs::write(&path, json).map_err(anyhow::Error::from));
                match res {
                    Ok(()) => {
                        ui.set_export_path(SharedString::from(format!("JSON → {}", path.display())))
                    }
                    Err(e) => {
                        ui.set_export_path(SharedString::from(format!("export failed: {}", e)))
                    }
                }
            }
        });
    }

    {
        let state_html = state.clone();
        let ui_weak_html = ui.as_weak();
        ui.on_export_html_clicked(move || {
            let Some(ui) = ui_weak_html.upgrade() else {
                return;
            };
            if let AppState::Showing(result) = &*state_html.borrow() {
                let path = results_dir().join(format!(
                    "bench_{}.html",
                    Local::now().format("%Y%m%d_%H%M%S")
                ));
                match std::fs::write(&path, render_html(result)) {
                    Ok(()) => {
                        ui.set_export_path(SharedString::from(format!("HTML → {}", path.display())))
                    }
                    Err(e) => {
                        ui.set_export_path(SharedString::from(format!("export failed: {}", e)))
                    }
                }
            }
        });
    }

    {
        let ui_weak_restart = ui.as_weak();
        let state_restart = state.clone();
        let receiver_restart = receiver.clone();
        ui.on_restart_clicked(move || {
            *state_restart.borrow_mut() = AppState::Idle;
            *receiver_restart.borrow_mut() = None;
            if let Some(ui) = ui_weak_restart.upgrade() {
                apply_state_to_ui(&ui, &state_restart.borrow());
            }
        });
    }

    {
        let ui_weak = ui.as_weak();
        ui.on_toggle_locale(move || {
            if let Some(ui) = ui_weak.upgrade() {
                let next = if ui.get_locale().as_str() == "fr" {
                    "en"
                } else {
                    "fr"
                };
                ui.set_locale(SharedString::from(next));
            }
        });
    }

    {
        let ui_weak = ui.as_weak();
        ui.on_toggle_theme(move || {
            if let Some(ui) = ui_weak.upgrade() {
                let dark = ui.get_dark();
                ui.set_dark(!dark);
            }
        });
    }

    apply_state_to_ui(&ui, &state.borrow());
    ui.run()
}

fn build_bench_list(selected: &HashSet<String>, all_checked: bool) -> ModelRc<BenchToggle> {
    let rows: Vec<BenchToggle> = suite::AVAILABLE_BENCHMARKS
        .iter()
        .map(|s| BenchToggle {
            name: SharedString::from(s.name),
            checked: all_checked || selected.contains(s.name),
        })
        .collect();
    ModelRc::new(VecModel::from(rows))
}

fn apply_event(state: AppState, event: RunnerEvent) -> AppState {
    match event {
        RunnerEvent::BenchStarted(name) => match state {
            AppState::Running {
                completed,
                total,
                failed,
                last_result,
                ..
            } => AppState::Running {
                current_test: name,
                completed,
                total,
                current_progress: 0.0,
                failed,
                last_result,
            },
            other => other,
        },
        RunnerEvent::Progress(frac) => match state {
            AppState::Running {
                current_test,
                completed,
                total,
                failed,
                last_result,
                ..
            } => AppState::Running {
                current_test,
                completed,
                total,
                current_progress: frac.clamp(0.0, 1.0),
                failed,
                last_result,
            },
            other => other,
        },
        RunnerEvent::BenchFinished(name, score) => match state {
            AppState::Running {
                current_test,
                completed,
                total,
                failed,
                ..
            } => AppState::Running {
                current_test,
                completed: completed + 1,
                total,
                current_progress: 0.0,
                failed,
                last_result: format!("{} = {} {}", name, score, bench_meta(&name).unit),
            },
            other => other,
        },
        RunnerEvent::BenchFailed(name, msg) => match state {
            AppState::Running {
                current_test,
                completed,
                total,
                current_progress,
                mut failed,
                last_result,
            } => {
                failed.push(format!("{}: {}", name, msg));
                AppState::Running {
                    current_test,
                    completed,
                    total,
                    current_progress,
                    failed,
                    last_result,
                }
            }
            other => other,
        },
        RunnerEvent::Done(result) => AppState::Showing(result),
        RunnerEvent::Error(e) => AppState::Error(e),
    }
}

fn apply_state_to_ui(ui: &MainWindow, state: &AppState) {
    match state {
        AppState::Idle => {
            ui.set_status(SharedString::from("idle"));
            ui.set_current_test(SharedString::default());
            ui.set_progress(0.0);
            ui.set_error_message(SharedString::default());
            ui.set_export_path(SharedString::default());
            ui.set_last_result(SharedString::default());
        }
        AppState::Running {
            current_test,
            completed,
            total,
            current_progress,
            last_result,
            ..
        } => {
            ui.set_status(SharedString::from("running"));
            ui.set_current_test(SharedString::from(current_test.as_str()));
            ui.set_progress(if *total > 0 {
                (*completed as f32 + *current_progress) / *total as f32
            } else {
                0.0
            });
            ui.set_last_result(SharedString::from(last_result.as_str()));
        }
        AppState::Showing(result) => {
            ui.set_status(SharedString::from("showing"));
            ui.set_final_score(SharedString::from(format!(
                "Score global : {}",
                result.final_score
            )));
            ui.set_cpu_score(SharedString::from(format!(
                "Score CPU : {}",
                result.cpu_score
            )));
            ui.set_mem_score(SharedString::from(format!(
                "Score RAM : {}",
                result.mem_score
            )));
            ui.set_disk_score(SharedString::from(format!(
                "Score Disque : {}",
                result.disk_score
            )));
            ui.set_scores(build_score_model(result));
            ui.set_system_info(build_system_info_model(result));
            let err_lines: Vec<SharedString> = result
                .errors
                .iter()
                .map(|e| SharedString::from(e.as_str()))
                .collect();
            ui.set_error_lines(ModelRc::new(VecModel::from(err_lines)));
        }
        AppState::Error(err) => {
            ui.set_status(SharedString::from("error"));
            ui.set_error_message(SharedString::from(err.as_str()));
        }
    }
}

fn build_score_model(result: &BenchResult) -> ModelRc<ScoreRow> {
    let rows: Vec<ScoreRow> = result
        .scores
        .iter()
        .map(|s| ScoreRow {
            name: SharedString::from(s.name.as_str()),
            value: SharedString::from(format!("{} {}", s.raw_score, bench_meta(&s.name).unit)),
        })
        .collect();

    ModelRc::new(VecModel::from(rows))
}

fn build_system_info_model(result: &BenchResult) -> ModelRc<InfoLine> {
    let mut lines = Vec::new();

    if let Some(si) = &result.system_info {
        lines.push(info_line(format!(
            "CPU Vendor: {}",
            si.cpu
                .vendor
                .clone()
                .unwrap_or_else(|| "unknown".to_string())
        )));
        lines.push(info_line(format!(
            "CPU Model: {}",
            si.cpu
                .model
                .clone()
                .unwrap_or_else(|| "unknown".to_string())
        )));
        if let Some(temp) = si.cpu.cpu_temp_c {
            lines.push(info_line(format!("CPU Temp: {:.0} °C", temp)));
        }
        lines.push(info_line(format!(
            "Logical cores: {}",
            si.cpu.cores_logical
        )));

        let ram_display = if si.ram.total_mb >= 1024 {
            format!("{:.2} GB", si.ram.total_mb as f64 / 1024.0)
        } else {
            format!("{} MB", si.ram.total_mb)
        };
        lines.push(info_line(format!("RAM Total: {}", ram_display)));
        lines.push(info_line(format!(
            "RAM Type: {}",
            si.ram
                .ram_type
                .clone()
                .unwrap_or_else(|| "unknown".to_string())
        )));

        for d in &si.disks {
            let size_display = if let Some(b) = d.total_bytes {
                human_bytes(b as f64)
            } else {
                "unknown".to_string()
            };
            lines.push(info_line(format!(
                "Disk: {} {} {} [{}] (size: {}) mount: {:?}",
                d.vendor.clone().unwrap_or_default(),
                d.model.clone().unwrap_or_default(),
                d.name,
                d.disk_type.clone().unwrap_or_else(|| "unknown".to_string()),
                size_display,
                d.mount_point
            )));
        }
    } else {
        let sys = get_system_info();
        let ram_mb = sys.total_memory() / 1024;
        let ram_display = if ram_mb >= 1024 {
            format!("{:.2} GB", ram_mb as f64 / 1024.0)
        } else {
            format!("{} MB", ram_mb)
        };
        lines.push(info_line(format!("CPU: {}", sys.global_cpu_info().brand())));
        lines.push(info_line(format!("Cores: {}", sys.cpus().len())));
        lines.push(info_line(format!("RAM: {}", ram_display)));
    }

    ModelRc::new(VecModel::from(lines))
}

fn info_line(text: String) -> InfoLine {
    InfoLine {
        text: SharedString::from(text.as_str()),
    }
}
