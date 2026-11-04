use crate::engines::benchmark::bench_meta;
use crate::model::result::BenchResult;
use crate::util::format::human_bytes;

fn escape_html(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

fn score_bar(value: u64, max: u64) -> String {
    let pct = if max == 0 {
        0.0
    } else {
        (value as f64 / max as f64 * 100.0).min(100.0)
    };
    format!(
        "<div class=\"bar\"><div class=\"fill\" style=\"width:{:.1}%\"></div></div>",
        pct
    )
}

/// Render a self-contained HTML report (inline CSS, no external assets).
pub fn render_html(result: &BenchResult) -> String {
    let mut rows = String::new();
    for s in &result.scores {
        let meta = bench_meta(&s.name);
        rows.push_str(&format!(
            "<tr><td>{}</td><td class=\"num\">{} {}</td><td class=\"num\">{}</td></tr>",
            escape_html(&s.name),
            s.raw_score,
            meta.unit,
            s.weight
        ));
    }

    let mut errors = String::new();
    for e in &result.errors {
        errors.push_str(&format!("<li>{}</li>", escape_html(e)));
    }
    let errors_html = if errors.is_empty() {
        String::new()
    } else {
        format!("<h2>Errors</h2><ul class=\"errors\">{}</ul>", errors)
    };

    let mut sys = String::new();
    if let Some(si) = &result.system_info {
        sys.push_str(&format!(
            "<li>CPU: {} {} ({} logical cores{})</li>",
            escape_html(si.cpu.vendor.as_deref().unwrap_or("unknown")),
            escape_html(si.cpu.model.as_deref().unwrap_or("unknown")),
            si.cpu.cores_logical,
            si.cpu
                .cpu_temp_c
                .map(|t| format!(", {:.0} °C", t))
                .unwrap_or_default()
        ));
        if let Some(freq) = si.cpu.frequency_mhz {
            sys.push_str(&format!("<li>Frequency: {} MHz</li>", freq));
        }
        sys.push_str(&format!(
            "<li>RAM: {} MB{}</li>",
            si.ram.total_mb,
            si.ram
                .ram_type
                .as_ref()
                .map(|t| format!(" ({})", escape_html(t)))
                .unwrap_or_default()
        ));
        for d in &si.disks {
            let size = d
                .size_readable
                .clone()
                .unwrap_or_else(|| human_bytes(d.total_bytes.unwrap_or(0) as f64));
            sys.push_str(&format!(
                "<li>Disk: {} {} [{}] — {}</li>",
                escape_html(d.vendor.as_deref().unwrap_or("unknown")),
                escape_html(d.model.as_deref().unwrap_or("unknown")),
                escape_html(d.disk_type.as_deref().unwrap_or("unknown")),
                escape_html(&size)
            ));
        }
    }

    let timestamp = result
        .timestamp
        .clone()
        .unwrap_or_else(|| "unknown".to_string());

    format!(
        r#"<!DOCTYPE html>
<html lang="en">
<head>
<meta charset="utf-8">
<title>OBenchmark report</title>
<style>
body {{ background:#050505; color:#f5f2ea; font-family:monospace; margin:2em auto; max-width:900px; padding:0 1em; }}
h1 {{ letter-spacing:4px; color:#7af9ff; }}
h2 {{ color:#b3afa4; font-size:1em; letter-spacing:2px; border-bottom:1px solid #232323; padding-bottom:4px; }}
.cards {{ display:flex; gap:1em; flex-wrap:wrap; margin:1em 0; }}
.card {{ background:#111; border:1px solid #232323; padding:1em 1.5em; min-width:140px; }}
.card .v {{ font-size:1.6em; color:#7af9ff; }}
.bar {{ background:#111; border:1px solid #232323; height:8px; margin-top:6px; }}
.fill {{ background:#7af9ff; height:100%; }}
table {{ width:100%; border-collapse:collapse; margin:1em 0; }}
td {{ border-bottom:1px solid #1c1c1c; padding:6px 4px; }}
td.num {{ text-align:right; color:#7af9ff; }}
ul {{ list-style:none; padding-left:0; }}
li {{ padding:2px 0; color:#b3afa4; }}
.errors li {{ color:#ff7a92; }}
.meta {{ color:#6e6b64; font-size:0.85em; }}
</style>
</head>
<body>
<h1>OBENCHMARK</h1>
<p class="meta">Report generated {} — final score {}</p>
<div class="cards">
<div class="card"><div>Global</div><div class="v">{}</div>{}</div>
<div class="card"><div>CPU</div><div class="v">{}</div>{}</div>
<div class="card"><div>RAM</div><div class="v">{}</div>{}</div>
<div class="card"><div>Disk</div><div class="v">{}</div>{}</div>
</div>
<h2>Detailed results</h2>
<table>
<tr><td>Benchmark</td><td class="num">Raw</td><td class="num">Weight</td></tr>
{}
</table>
<h2>System information</h2>
<ul>{}</ul>
{}
</body>
</html>"#,
        escape_html(&timestamp),
        result.final_score,
        result.final_score,
        score_bar(result.final_score, 99_999),
        result.cpu_score,
        score_bar(result.cpu_score, 99_999),
        result.mem_score,
        score_bar(result.mem_score, 99_999),
        result.disk_score,
        score_bar(result.disk_score, 99_999),
        rows,
        sys,
        errors_html
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::result::{BenchResult, BenchScore};

    #[test]
    fn escape_html_escapes_specials() {
        assert_eq!(
            escape_html("<a href=\"x\">&"),
            "&lt;a href=&quot;x&quot;&gt;&amp;"
        );
    }

    #[test]
    fn render_html_contains_scores_and_escapes() {
        let result = BenchResult {
            scores: vec![BenchScore {
                name: "<b>CPU</b>".to_string(),
                raw_score: 42,
                weight: 2,
            }],
            final_score: 1000,
            cpu_score: 1000,
            mem_score: 0,
            disk_score: 0,
            errors: vec!["disk <failed>".to_string()],
            timestamp: Some("2025-01-01T00:00:00+00:00".to_string()),
            system_info: None,
        };
        let html = render_html(&result);
        assert!(html.contains("&lt;b&gt;CPU&lt;/b&gt;"));
        assert!(html.contains("disk &lt;failed&gt;"));
        assert!(html.contains("final score 1000"));
        assert!(!html.contains("<b>CPU</b>"));
    }

    #[test]
    fn score_bar_caps_at_100_percent() {
        assert!(score_bar(200_000, 99_999).contains("100.0%"));
        assert!(score_bar(0, 99_999).contains("0.0%"));
        assert!(score_bar(1, 0).contains("0.0%"));
    }
}
