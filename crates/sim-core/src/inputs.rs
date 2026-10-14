//! Entrées utilisateur du Module A (curseurs du HUD). Toute valeur passe par [`Inputs::sanitized`].

use crate::safety::sanitize;

/// Perturbations continues appliquées au système cardiovasculaire.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Inputs {
    /// Intensité de l'effort physique global, 0 (repos) → 1 (effort maximal).
    pub effort: f64,
    /// Débit d'hémorragie en mL/s (0 = aucune).
    pub hemorrhage_rate_ml_s: f64,
    /// Altitude en mètres (0 → 8848 m), pilote la pression partielle d'O₂.
    pub altitude_m: f64,
    /// Facteur multiplicatif de la résistance périphérique (1 = normal ;
    /// ≈0,3 = choc anaphylactique/vasoplégie ; >1 = vasoconstriction pharmacologique).
    pub vasodilation_factor: f64,
    /// Viscosité sanguine relative η/η₀ (1 = normal ; <1 hémodilution/anémie ;
    /// >1 déshydratation/polyglobulie).
    pub viscosity_factor: f64,
}

impl Default for Inputs {
    fn default() -> Self {
        Self {
            effort: 0.0,
            hemorrhage_rate_ml_s: 0.0,
            altitude_m: 0.0,
            vasodilation_factor: 1.0,
            viscosity_factor: 1.0,
        }
    }
}

impl Inputs {
    pub const MAX_ALTITUDE_M: f64 = 8848.0;
    pub const MAX_HEMORRHAGE_ML_S: f64 = 200.0;

    /// Renvoie des entrées finies et bornées dans leur domaine physiologique.
    pub fn sanitized(self) -> Self {
        Self {
            effort: sanitize(self.effort, 0.0, 1.0, 0.0),
            hemorrhage_rate_ml_s: sanitize(
                self.hemorrhage_rate_ml_s,
                0.0,
                Self::MAX_HEMORRHAGE_ML_S,
                0.0,
            ),
            altitude_m: sanitize(self.altitude_m, 0.0, Self::MAX_ALTITUDE_M, 0.0),
            vasodilation_factor: sanitize(self.vasodilation_factor, 0.05, 3.0, 1.0),
            viscosity_factor: sanitize(self.viscosity_factor, 0.4, 4.0, 1.0),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extreme_inputs_are_clamped() {
        let i = Inputs {
            effort: 1e9,
            hemorrhage_rate_ml_s: f64::NAN,
            altitude_m: f64::INFINITY,
            vasodilation_factor: -4.0,
            viscosity_factor: 0.0,
        }
        .sanitized();
        assert_eq!(i.effort, 1.0);
        assert_eq!(i.hemorrhage_rate_ml_s, 0.0);
        assert_eq!(i.altitude_m, 0.0);
        assert_eq!(i.vasodilation_factor, 0.05);
        assert_eq!(i.viscosity_factor, 0.4);
    }
}
