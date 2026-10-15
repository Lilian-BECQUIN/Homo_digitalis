//! Échanges gazeux simplifiés : altitude → PiO₂ → PaO₂ → SpO₂ (Severinghaus).

use crate::safety::clamp01;

/// Résultat quasi statique des échanges gazeux pour une altitude donnée.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct GasExchange {
    pub barometric_mmhg: f64,
    pub inspired_o2_mmhg: f64,
    pub pao2_mmhg: f64,
    pub paco2_mmhg: f64,
    /// Saturation de l'hémoglobine, fraction 0..1.
    pub spo2: f64,
}

/// Pression barométrique (modèle d'atmosphère standard), en mmHg.
pub fn barometric_pressure_mmhg(altitude_m: f64) -> f64 {
    let h = altitude_m.clamp(0.0, 11_000.0);
    760.0 * (1.0 - 2.25577e-5 * h).powf(5.25588)
}

/// Courbe de dissociation de l'oxyhémoglobine (équation de Severinghaus), fraction 0..1.
pub fn spo2_from_po2(po2_mmhg: f64) -> f64 {
    let p = po2_mmhg.clamp(0.0, 1000.0);
    1.0 / (23_400.0 / (p * p * p + 150.0 * p).max(1e-9) + 1.0)
}

/// Calcule l'équilibre gazeux avec réponse ventilatoire hypoxique
/// (l'hyperventilation abaisse la PaCO₂ et protège partiellement la PaO₂).
pub fn compute(altitude_m: f64) -> GasExchange {
    let pb = barometric_pressure_mmhg(altitude_m);
    let pio2 = 0.2095 * (pb - 47.0);
    let mut paco2 = 40.0;
    let mut pao2 = (pio2 - paco2 / 0.8 - 2.0).max(1.0);
    // Point fixe sous-relaxé : la pente de la fonction d'itération dépasse 1 en valeur absolue
    // (≈ -1,9), l'itération brute oscillerait entre deux états. Relaxation 0,4 → contraction.
    for _ in 0..60 {
        let drive = clamp01((90.0 - pao2) / 50.0);
        let ventilation = 1.0 + 2.3 * drive;
        let paco2_target = 40.0 / ventilation;
        paco2 += 0.4 * (paco2_target - paco2);
        pao2 = (pio2 - paco2 / 0.8 - 2.0).max(1.0);
    }
    GasExchange {
        barometric_mmhg: pb,
        inspired_o2_mmhg: pio2,
        pao2_mmhg: pao2,
        paco2_mmhg: paco2,
        spo2: spo2_from_po2(pao2),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sea_level_is_normal() {
        let g = compute(0.0);
        assert!(g.spo2 > 0.96 && g.spo2 < 1.0, "spo2={}", g.spo2);
        assert!((g.paco2_mmhg - 40.0).abs() < 1.0);
    }

    #[test]
    fn everest_pressure_is_about_a_third() {
        let p = barometric_pressure_mmhg(8848.0);
        assert!(p > 220.0 && p < 270.0, "p={p}");
    }

    #[test]
    fn spo2_decreases_monotonically_with_altitude() {
        let mut prev = 2.0;
        for h in (0..=8848).step_by(500) {
            let s = compute(h as f64).spo2;
            assert!(s <= prev + 1e-9);
            prev = s;
        }
    }

    #[test]
    fn severinghaus_p50() {
        assert!((spo2_from_po2(26.8) - 0.5).abs() < 0.01);
    }
}
