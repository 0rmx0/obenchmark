//! Fonctions d’aide pour normaliser ou pondérer les scores.
//! Vous pouvez étendre ce module selon vos besoins.

/// Exemple de pondération simple (poids = 1 pour tous les tests).
pub fn weighted_average(scores: &[(String, u64)], weights: &[f64]) -> f64 {
    assert_eq!(scores.len(), weights.len());
    let weighted_sum: f64 = scores
        .iter()
        .zip(weights.iter())
        .map(|((_, s), w)| (*s as f64) * w)
        .sum();
    let weight_total: f64 = weights.iter().sum();
    weighted_sum / weight_total
}
