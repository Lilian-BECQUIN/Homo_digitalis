//! Loi de Poiseuille : R = 8·η·L / (π·r⁴).

use core::f64::consts::PI;

/// Viscosité sanguine de référence (Pa·s) ≈ 3,5 mPa·s.
pub const BLOOD_VISCOSITY_PA_S: f64 = 3.5e-3;

/// Conversion Pa·s/m³ → mmHg·s/mL (1 Pa = 7,50062e-3 mmHg ; 1 m³ = 1e6 mL).
pub const SI_TO_MMHG_S_PER_ML: f64 = 7.50062e-9;

/// Résistance hydraulique d'un segment cylindrique, en Pa·s/m³.
pub fn poiseuille_resistance_si(viscosity_pa_s: f64, length_m: f64, radius_m: f64) -> f64 {
    let r = radius_m.max(1e-6);
    8.0 * viscosity_pa_s * length_m / (PI * r.powi(4))
}

/// Même résistance, exprimée en mmHg·s/mL (unités du solveur).
pub fn poiseuille_resistance(viscosity_pa_s: f64, length_m: f64, radius_m: f64) -> f64 {
    poiseuille_resistance_si(viscosity_pa_s, length_m, radius_m) * SI_TO_MMHG_S_PER_ML
}

/// Facteur multiplicatif de résistance par rapport à l'état de référence :
/// `η/η₀ · (r₀/r)⁴`. Le rayon relatif est borné pour éviter toute singularité.
pub fn resistance_scale(viscosity_ratio: f64, radius_ratio: f64) -> f64 {
    viscosity_ratio * radius_ratio.clamp(0.05, 3.0).powi(-4)
}

/// Segment artériel (composant ECS ; sera relié au graphe 1D en phase 2b).
#[derive(Clone, Copy, Debug)]
pub struct VesselSegment {
    pub length_m: f64,
    pub rest_radius_m: f64,
    /// Rayon relatif courant (1 = repos ; <1 vasoconstriction ou sténose).
    pub radius_ratio: f64,
}

impl VesselSegment {
    pub fn resistance_mmhg_s_ml(&self, viscosity_pa_s: f64) -> f64 {
        poiseuille_resistance(
            viscosity_pa_s,
            self.length_m,
            self.rest_radius_m * self.radius_ratio.clamp(0.05, 3.0),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn halving_radius_multiplies_resistance_by_16() {
        let a = poiseuille_resistance_si(BLOOD_VISCOSITY_PA_S, 0.1, 2e-3);
        let b = poiseuille_resistance_si(BLOOD_VISCOSITY_PA_S, 0.1, 1e-3);
        assert!((b / a - 16.0).abs() < 1e-9);
    }

    #[test]
    fn resistance_proportional_to_viscosity() {
        let a = poiseuille_resistance_si(3.5e-3, 0.1, 1e-3);
        let b = poiseuille_resistance_si(7.0e-3, 0.1, 1e-3);
        assert!((b / a - 2.0).abs() < 1e-9);
    }

    #[test]
    fn scale_is_finite_for_zero_radius() {
        assert!(resistance_scale(1.0, 0.0).is_finite());
    }
}
