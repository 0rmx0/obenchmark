use chrono::Local;
use crossbeam_channel::{unbounded, Receiver};
use slint::{ModelRc, SharedString, VecModel};

use std::cell::RefCell;
use std::rc::Rc;
use std::time::Duration;

use crate::{
    app::state::AppState,
    benchmarks::suite,
    engines::runner::{run_benchmarks, RunnerEvent},
    model::result::BenchResult,
    util::sysinfo::get_system_info,
};

slint::include_modules!();

pub fn run_gui() -> Result<(), slint::PlatformError> {
    let ui = MainWindow::new()?;

    let state = Rc::new(RefCell::new(AppState::Idle));
    let receiver: Rc<RefCell<Option<Receiver<RunnerEvent>>>> = Rc::new(RefCell::new(None));

    let ui_weak = ui.as_weak();
    let state_tick = state.clone();
    let receiver_tick = receiver.clone();

    let timer = slint::Timer::default();
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
            *state_tick.borrow_mut() = current.clone();
            apply_state_to_ui(&ui, &current);
        },
    );

    let ui_weak_start = ui.as_weak();
    let state_start = state.clone();
    let receiver_start = receiver.clone();
    ui.on_start_clicked(move || {
        let (tx, rx) = unbounded();
        let benches = suite::default_suite();
        let total = benches.len();

        *state_start.borrow_mut() = AppState::Running {
            current_test: String::new(),
            completed: 0,
            total,
        };
        *receiver_start.borrow_mut() = Some(rx);

        if let Some(ui) = ui_weak_start.upgrade() {
            apply_state_to_ui(&ui, &state_start.borrow());
        }

        std::thread::spawn(move || {
            run_benchmarks(benches, tx);
        });
    });

    let state_export = state.clone();
    ui.on_export_clicked(move || {
        if let AppState::Showing(result) = &*state_export.borrow() {
            let json = serde_json::to_string_pretty(result).unwrap();
            let _ = std::fs::write(format!("bench_{}.json", Local::now().timestamp()), json);
        }
    });

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

    apply_state_to_ui(&ui, &state.borrow());
    ui.run()
}

fn apply_event(state: AppState, event: RunnerEvent) -> AppState {
    match event {
        RunnerEvent::BenchStarted(name) => {
            if let AppState::Running {
                completed,
                total,
                ..
            } = state
            {
                AppState::Running {
                    current_test: name,
                    completed,
                    total,
                }
            } else {
                state
            }
        }
        RunnerEvent::BenchFinished(_, _) => {
            if let AppState::Running {
                current_test,
                completed,
                total,
            } = state
            {
                AppState::Running {
                    current_test,
                    completed: completed + 1,
                    total,
                }
            } else {
                state
            }
        }
        RunnerEvent::BenchFinishedWithSamples(_, _) => {
            if let AppState::Running {
                current_test,
                completed,
                total,
            } = state
            {
                AppState::Running {
                    current_test,
                    completed: completed + 1,
                    total,
                }
            } else {
                state
            }
        }
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
        }
        AppState::Running {
            current_test,
            completed,
            total,
        } => {
            ui.set_status(SharedString::from("running"));
            ui.set_current_test(SharedString::from(current_test.as_str()));
            ui.set_progress(if *total > 0 {
                *completed as f32 / *total as f32
            } else {
                0.0
            });
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
            ui.set_gfx_score(SharedString::from(format!(
                "Score GFX : {}",
                result.gfx_score
            )));
            ui.set_scores(build_score_model(result));
            ui.set_system_info(build_system_info_model(result));
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
            value: SharedString::from(format!(
                "{} {}",
                s.raw_score,
                unit_for_bench(&s.name)
            )),
        })
        .collect();

    ModelRc::new(VecModel::from(rows))
}

fn build_system_info_model(result: &BenchResult) -> ModelRc<InfoLine> {
    let mut lines = Vec::new();

    if let Some(si) = &result.system_info {
        lines.push(info_line(format!(
            "CPU Vendor: {}",
            si.cpu.vendor.clone().unwrap_or_else(|| "unknown".to_string())
        )));
        lines.push(info_line(format!(
            "CPU Model: {}",
            si.cpu.model.clone().unwrap_or_else(|| "unknown".to_string())
        )));
        lines.push(info_line(format!("Logical cores: {}", si.cpu.cores_logical)));

        let ram_display = if si.ram.total_mb >= 1024 {
            format!("{:.2} GB", si.ram.total_mb as f64 / 1024.0)
        } else {
            format!("{} MB", si.ram.total_mb)
        };
        lines.push(info_line(format!("RAM Total: {}", ram_display)));
        lines.push(info_line(format!(
            "RAM Type: {}",
            si.ram.ram_type.clone().unwrap_or_else(|| "unknown".to_string())
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

fn human_bytes(mut bytes: f64) -> String {
    let units = ["B", "KB", "MB", "GB", "TB"];
    let mut i = 0;
    while bytes >= 1024.0 && i < units.len() - 1 {
        bytes /= 1024.0;
        i += 1;
    }
    if i == 0 {
        format!("{} {}", bytes as u64, units[i])
    } else {
        format!("{:.2} {}", bytes, units[i])
    }
}

fn unit_for_bench(name: &str) -> &'static str {
    match name {
        "CPU Multi-Core" | "CPU Int Math" | "CPU Float Math" | "CPU Prime Calc" | "CPU SSE Ext"
        | "CPU Physics" | "CPU Sorting" | "CPU UCT Single" => "ops/s",
        "CPU Compression" | "CPU Encryption" => "MB/s",
        "Mem DB Ops" => "ops/s",
        "Mem Cached Read" | "Mem Uncached Read" | "Mem Write" | "Mem Threaded" => "MB/s",
        "Mem Available" => "MB",
        "Mem Latency" => "accès/s",
        "Disk Seq Read" | "Disk Seq Write" => "MB/s",
        "Disk IOPS 32K QD20" | "Disk IOPS 4K QD1" => "IOPS",
        "GFX 2D Raster" => "MPix/s",
        "GFX 3D Raster" => "tris/s",
        _ => "",
    }
}
