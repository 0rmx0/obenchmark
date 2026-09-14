//! Utilitaires de mesure reproductible pour les benchmarks.
//!
//! Un benchmark crédible doit :
//! - chauffer les caches / le turbo (warmup) ;
//! - répéter la mesure plusieurs fois ;
//! - agréger par la médiane (robuste aux pics ponctuels) plutôt que la moyenne.

use std::time::Duration;

/// Durée cible d'une passe de mesure (courte pour rester réactif).
const SAMPLE_DURATION_MS: u64 = 300;

/// Effectue `passes` mesures d'une fonction de benchmark et renvoie la médiane
/// du résultat (par exemple ops/s ou Mo/s).
///
/// `sample` est appelé avec un budget de temps d'environ `SAMPLE_DURATION_MS` ;
/// il doit renvoyer une grandeur normalisée par seconde (débit) pour que les
/// passes soient comparables malgré des durées légèrement différentes.
pub fn median_of_passes<F>(warmup_passes: usize, measured_passes: usize, mut sample: F) -> u64
where
    F: FnMut(Duration) -> u64,
{
    // Warmup : on ignore le résultat, l'objectif est de stabiliser caches + turbo.
    let warmup_budget = Duration::from_millis(SAMPLE_DURATION_MS);
    for _ in 0..warmup_passes {
        let _ = sample(warmup_budget);
    }

    let mut results: Vec<u64> = Vec::with_capacity(measured_passes);
    let budget = Duration::from_millis(SAMPLE_DURATION_MS);
    for _ in 0..measured_passes {
        results.push(sample(budget));
    }

    results.sort_unstable();
    let mid = results.len() / 2;
    if results.is_empty() {
        0
    } else if results.len().is_multiple_of(2) {
        (results[mid - 1] + results[mid]) / 2
    } else {
        results[mid]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn median_of_constant() {
        let v = median_of_passes(1, 3, |_| 1000);
        assert_eq!(v, 1000);
    }

    #[test]
    fn median_odd_picks_middle() {
        let vals = [3u64, 1, 2];
        let mut i = 0usize;
        let v = median_of_passes(0, 3, |_| {
            let x = vals[i];
            i += 1;
            x
        });
        assert_eq!(v, 2);
    }

    #[test]
    fn median_even_averages_middle_two() {
        let vals = [4u64, 1, 3, 2];
        let mut i = 0usize;
        let v = median_of_passes(0, 4, |_| {
            let x = vals[i];
            i += 1;
            x
        });
        assert_eq!(v, (2 + 3) / 2);
    }
}
