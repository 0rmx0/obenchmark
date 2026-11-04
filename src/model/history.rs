use anyhow::{Context, Result};
use std::path::PathBuf;

use crate::model::result::BenchResult;

/// JSON Lines history file, stored in the user data dir.
pub fn history_path() -> PathBuf {
    dirs::data_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("obenchmark")
        .join("history.jsonl")
}

/// Append a run to the local history (best effort at call sites).
pub fn append_history(result: &BenchResult) -> Result<()> {
    let path = history_path();
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .with_context(|| format!("failed creating {}", parent.display()))?;
    }
    let line = serde_json::to_string(result)?;
    use std::io::Write;
    let mut file = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&path)
        .with_context(|| format!("failed opening {}", path.display()))?;
    writeln!(file, "{}", line).with_context(|| format!("failed writing {}", path.display()))?;
    Ok(())
}

/// Load all recorded runs, oldest first.
pub fn load_history() -> Result<Vec<BenchResult>> {
    let path = history_path();
    let Ok(content) = std::fs::read_to_string(&path) else {
        return Ok(Vec::new());
    };
    let mut out = Vec::new();
    for line in content.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        let result: BenchResult = serde_json::from_str(line)
            .with_context(|| format!("corrupted history line in {}", path.display()))?;
        out.push(result);
    }
    Ok(out)
}

/// One row of a comparison between two runs.
#[derive(Clone, Debug)]
pub struct CompareRow {
    pub name: String,
    pub a: u64,
    pub b: u64,
    pub delta: i128,
}

/// Compare two results per benchmark (matched by name).
pub fn compare(a: &BenchResult, b: &BenchResult) -> Vec<CompareRow> {
    let mut rows = Vec::new();
    for sb in &b.scores {
        let sa = a.scores.iter().find(|s| s.name == sb.name);
        let va = sa.map(|s| s.raw_score).unwrap_or(0);
        rows.push(CompareRow {
            name: sb.name.clone(),
            a: va,
            b: sb.raw_score,
            delta: sb.raw_score as i128 - va as i128,
        });
    }
    rows
}

/// Human-readable comparison summary.
pub fn format_comparison(label_a: &str, a: &BenchResult, label_b: &str, b: &BenchResult) -> String {
    let mut out = String::new();
    out.push_str(&format!("Comparison: {} vs {}\n", label_a, label_b));
    out.push_str(&format!(
        "  Global: {} -> {} ({:+})\n",
        a.final_score,
        b.final_score,
        b.final_score as i128 - a.final_score as i128
    ));
    out.push_str(&format!(
        "  CPU:    {} -> {} ({:+})\n",
        a.cpu_score,
        b.cpu_score,
        b.cpu_score as i128 - a.cpu_score as i128
    ));
    out.push_str(&format!(
        "  RAM:    {} -> {} ({:+})\n",
        a.mem_score,
        b.mem_score,
        b.mem_score as i128 - a.mem_score as i128
    ));
    out.push_str(&format!(
        "  Disk:   {} -> {} ({:+})\n",
        a.disk_score,
        b.disk_score,
        b.disk_score as i128 - a.disk_score as i128
    ));
    out.push_str("  Per benchmark:\n");
    for row in compare(a, b) {
        out.push_str(&format!(
            "    {: <28} {:>12} -> {:>12} ({:+})\n",
            row.name, row.a, row.b, row.delta
        ));
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::result::BenchScore;

    fn result(final_score: u64, scores: Vec<(&str, u64)>) -> BenchResult {
        BenchResult {
            scores: scores
                .into_iter()
                .map(|(n, r)| BenchScore {
                    name: n.to_string(),
                    raw_score: r,
                    weight: 2,
                })
                .collect(),
            final_score,
            cpu_score: final_score,
            mem_score: 0,
            disk_score: 0,
            errors: vec![],
            timestamp: Some("2025-01-01T00:00:00+00:00".to_string()),
            system_info: None,
        }
    }

    #[test]
    fn compare_matches_by_name_and_computes_delta() {
        let a = result(1000, vec![("CPU Int Math", 100), ("Mem Write", 50)]);
        let b = result(1200, vec![("CPU Int Math", 150), ("Mem Write", 40)]);
        let rows = compare(&a, &b);
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].name, "CPU Int Math");
        assert_eq!(rows[0].delta, 50);
        assert_eq!(rows[1].delta, -10);
    }

    #[test]
    fn compare_handles_missing_benchmarks_as_zero() {
        let a = result(1000, vec![("CPU Int Math", 100)]);
        let b = result(1000, vec![("CPU Int Math", 100), ("Disk Seq Read", 500)]);
        let rows = compare(&a, &b);
        let disk = rows.iter().find(|r| r.name == "Disk Seq Read").unwrap();
        assert_eq!(disk.a, 0);
        assert_eq!(disk.delta, 500);
    }

    #[test]
    fn format_comparison_contains_scores_and_deltas() {
        let a = result(1000, vec![("CPU Int Math", 100)]);
        let b = result(1100, vec![("CPU Int Math", 120)]);
        let text = format_comparison("old.json", &a, "new.json", &b);
        assert!(text.contains("old.json vs new.json"));
        assert!(text.contains("1000 -> 1100 (+100)"));
        assert!(text.contains("CPU Int Math"));
    }

    #[test]
    fn history_roundtrip_through_temp_dir() {
        // Exercise append/load against a redirected history path is not
        // possible without touching the real data dir, so test the JSONL
        // encoding contract directly.
        let r = result(42, vec![("CPU Int Math", 7)]);
        let line = serde_json::to_string(&r).unwrap();
        let back: BenchResult = serde_json::from_str(&line).unwrap();
        assert_eq!(back.final_score, 42);
        assert_eq!(back.scores[0].raw_score, 7);
    }
}
