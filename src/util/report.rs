use crate::engines::benchmark::bench_meta;
use crate::model::result::BenchResult;
use crate::util::format::human_bytes;

// Police HelpMe embarquée : le rapport HTML reste autonome (aucun asset externe).
const HELPME_TTF: &[u8] = include_bytes!("../../ui/fonts/HelpMe.ttf");

// Couche cachée : ligne base64 affichée telle quelle, jamais décodée à l'écran.
const HIDDEN_LAYER: &[u8] = b"OBM::RMX::CRT::0A0F0A::33FF66::TERMINAL-V2";

fn escape_html(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

fn base64_encode(data: &[u8]) -> String {
    const ALPHABET: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::with_capacity(data.len().div_ceil(3) * 4);
    for chunk in data.chunks(3) {
        let b0 = u32::from(chunk[0]);
        let b1 = u32::from(*chunk.get(1).unwrap_or(&0));
        let b2 = u32::from(*chunk.get(2).unwrap_or(&0));
        let n = (b0 << 16) | (b1 << 8) | b2;
        out.push(ALPHABET[(n >> 18 & 63) as usize] as char);
        out.push(ALPHABET[(n >> 12 & 63) as usize] as char);
        out.push(if chunk.len() > 1 {
            ALPHABET[(n >> 6 & 63) as usize] as char
        } else {
            '='
        });
        out.push(if chunk.len() > 2 {
            ALPHABET[(n & 63) as usize] as char
        } else {
            '='
        });
    }
    out
}

fn score_bar(value: u64, max: u64) -> String {
    let pct = if max == 0 {
        0.0
    } else {
        (value as f64 / max as f64 * 100.0).min(100.0)
    };
    let filled = ((pct / 100.0) * 20.0).round() as usize;
    format!(
        "[{}{}] {:.1}%",
        "█".repeat(filled),
        "░".repeat(20 - filled),
        pct
    )
}

/// Render a self-contained HTML report as an R_M_X terminal screen
/// (charte Terminal v2 : un seul <pre> plein écran, CSS inline, aucun JS).
pub fn render_html(result: &BenchResult) -> String {
    let sep = "─".repeat(56);

    let mut rows = String::new();
    for s in &result.scores {
        let meta = bench_meta(&s.name);
        rows.push_str(&format!(
            "{:<34}{:>12} {:<14}(poids {})\n",
            escape_html(&s.name),
            s.raw_score,
            escape_html(meta.unit),
            s.weight
        ));
    }

    let mut errors = String::new();
    for e in &result.errors {
        errors.push_str(&format!("! {}\n", escape_html(e)));
    }
    let errors_section = if errors.is_empty() {
        String::new()
    } else {
        format!("\n=== ERREURS ===\n{}", errors)
    };

    let mut sys = String::new();
    if let Some(si) = &result.system_info {
        sys.push_str(&format!(
            "> CPU : {} {} ({} cœurs{})\n",
            escape_html(si.cpu.vendor.as_deref().unwrap_or("inconnu")),
            escape_html(si.cpu.model.as_deref().unwrap_or("inconnu")),
            si.cpu.cores_logical,
            si.cpu
                .cpu_temp_c
                .map(|t| format!(", {:.0} °C", t))
                .unwrap_or_default()
        ));
        if let Some(freq) = si.cpu.frequency_mhz {
            sys.push_str(&format!("> Fréquence : {} MHz\n", freq));
        }
        sys.push_str(&format!(
            "> RAM : {} MB{}\n",
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
                "> Disque : {} {} [{}] — {}\n",
                escape_html(d.vendor.as_deref().unwrap_or("inconnu")),
                escape_html(d.model.as_deref().unwrap_or("inconnu")),
                escape_html(d.disk_type.as_deref().unwrap_or("inconnu")),
                escape_html(&size)
            ));
        }
    }

    let timestamp = result
        .timestamp
        .clone()
        .unwrap_or_else(|| "inconnu".to_string());

    let body = format!(
        "=== OBENCHMARK ===\n\
         {}\n\
         \n\
         Rapport généré le : {}\n\
         Score final : {}/99999 {}\n\
         \n\
         === SCORES ===\n\
         Score global : {} {}\n\
         Score CPU    : {} {}\n\
         Score RAM    : {} {}\n\
         Score Disque : {} {}\n\
         \n\
         === RÉSULTATS DÉTAILLÉS ===\n\
         {:<34}{:>12} {:<14}(poids)\n\
         {rows}\n\
         \n\
         === INFORMATIONS SYSTÈME ===\n\
         {sys}{errors_section}\n\
         {}\n",
        sep,
        escape_html(&timestamp),
        result.final_score,
        score_bar(result.final_score, 99_999),
        result.final_score,
        score_bar(result.final_score, 99_999),
        result.cpu_score,
        score_bar(result.cpu_score, 99_999),
        result.mem_score,
        score_bar(result.mem_score, 99_999),
        result.disk_score,
        score_bar(result.disk_score, 99_999),
        "BENCHMARK",
        "RAW",
        "UNITÉ",
        base64_encode(HIDDEN_LAYER)
    );

    format!(
        r#"<!DOCTYPE html>
<html lang="fr">
<head>
<meta charset="utf-8">
<title>OBENCHMARK — RAPPORT</title>
<style>
@font-face {{ font-family: 'HelpMe'; src: url(data:font/ttf;base64,{font_b64}) format('truetype'); font-weight: normal; font-style: normal; }}
:root {{ --fond: #0a0f0a; --texte: #33ff66; --halo: 0 0 6px rgba(51, 255, 102, 0.55); --taille: clamp(9px, 1.55vw, 16px); }}
* {{ box-sizing: border-box; }}
html, body {{ margin: 0; padding: 0; height: 100%; background: var(--fond); color: var(--texte); font-family: 'HelpMe', 'Courier New', monospace; }}
#terminal {{ position: fixed; inset: 0; margin: 0; padding: 1.2em; overflow-y: auto; overflow-x: hidden; white-space: pre-wrap; word-break: break-word; font-size: var(--taille); line-height: 1.25; text-shadow: var(--halo); outline: none; cursor: text; scrollbar-width: thin; scrollbar-color: var(--texte) var(--fond); }}
#terminal::after {{ content: '\2588'; animation: rmx-blink 1s steps(1) infinite; }}
@keyframes rmx-blink {{ 50% {{ opacity: 0; }} }}
</style>
</head>
<body>
<pre id="terminal">{body}</pre>
</body>
</html>"#,
        font_b64 = base64_encode(HELPME_TTF),
        body = body
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
    fn base64_encode_matches_rfc4648() {
        assert_eq!(base64_encode(b""), "");
        assert_eq!(base64_encode(b"Man"), "TWFu");
        assert_eq!(base64_encode(b"R_M_X"), "Ul9NX1g=");
    }

    #[test]
    fn render_html_is_a_terminal_screen() {
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
        assert!(html.contains("<html lang=\"fr\">"));
        assert!(html.contains("<pre id=\"terminal\">"));
        assert!(html.contains("#0a0f0a"));
        assert!(html.contains("#33ff66"));
        assert!(html.contains("data:font/ttf;base64,"));
        assert!(html.contains("Score final : 1000"));
        assert!(html.contains("&lt;b&gt;CPU&lt;/b&gt;"));
        assert!(html.contains("disk &lt;failed&gt;"));
        assert!(!html.contains("<b>CPU</b>"));
        assert!(!html.contains("<script"));
        assert!(html.contains(&base64_encode(HIDDEN_LAYER)));
    }

    #[test]
    fn score_bar_caps_at_100_percent() {
        assert!(score_bar(200_000, 99_999).contains("100.0%"));
        assert!(score_bar(0, 99_999).contains("0.0%"));
        assert!(score_bar(1, 0).contains("0.0%"));
    }
}
