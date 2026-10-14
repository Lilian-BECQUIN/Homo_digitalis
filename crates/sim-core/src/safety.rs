//! Garde-fous numériques : aucune valeur extrême ne doit produire NaN/Infinity.

/// Sigmoïde logistique `1 / (1 + e^x)` (décroissante en x), protégée contre l'overflow.
#[inline]
pub fn logistic_decreasing(x: f64) -> f64 {
    1.0 / (1.0 + x.clamp(-50.0, 50.0).exp())
}

/// Sigmoïde logistique croissante `1 / (1 + e^-x)`.
#[inline]
pub fn logistic_increasing(x: f64) -> f64 {
    logistic_decreasing(-x)
}

/// Remplace NaN/±Inf par `fallback`, puis borne dans `[lo, hi]`.
#[inline]
pub fn sanitize(x: f64, lo: f64, hi: f64, fallback: f64) -> f64 {
    if x.is_finite() {
        x.clamp(lo, hi)
    } else {
        fallback.clamp(lo, hi)
    }
}

#[inline]
pub fn clamp01(x: f64) -> f64 {
    x.clamp(0.0, 1.0)
}

/// Vrai si toutes les composantes sont finies.
pub fn all_finite<const N: usize>(y: &[f64; N]) -> bool {
    y.iter().all(|v| v.is_finite())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn logistic_never_overflows() {
        for x in [-1e300, -1e6, -50.0, 0.0, 50.0, 1e6, 1e300] {
            let v = logistic_decreasing(x);
            assert!(v.is_finite() && (0.0..=1.0).contains(&v));
        }
    }

    #[test]
    fn sanitize_handles_non_finite() {
        assert_eq!(sanitize(f64::NAN, 0.0, 1.0, 0.5), 0.5);
        assert_eq!(sanitize(f64::INFINITY, 0.0, 1.0, 0.5), 0.5);
        assert_eq!(sanitize(5.0, 0.0, 1.0, 0.5), 1.0);
    }
}
