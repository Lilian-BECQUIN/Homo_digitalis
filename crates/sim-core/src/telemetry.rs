//! Données de télémétrie publiées par le noyau vers le rendu / l'interface (lecture seule).

/// État global du modèle.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Status {
    Alive,
    /// Arrêt circulatoire (PAM filtrée trop basse trop longtemps) : état d'arrêt biologique du modèle.
    CardiacArrest,
    /// Divergence numérique détectée et rattrapée : le modèle est figé dans son dernier état sain.
    NumericalFault,
}

/// Instantané complet des grandeurs utiles à l'affichage (f32 : destiné au GPU / UI).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Telemetry {
    pub status: Status,
    pub time_s: f64,
    pub heart_rate_bpm: f32,
    /// Phase du cycle cardiaque 0..1 (pour animer le cœur).
    pub cardiac_phase: f32,
    /// Activation ventriculaire 0..1 (pilote le morph target « contraction »).
    pub ventricular_activation: f32,
    pub lv_volume_ml: f32,
    pub lv_pressure_mmhg: f32,
    pub arterial_pressure_mmhg: f32,
    pub venous_pressure_mmhg: f32,
    pub mean_arterial_pressure_mmhg: f32,
    pub systolic_mmhg: f32,
    pub diastolic_mmhg: f32,
    pub aortic_flow_ml_s: f32,
    pub stroke_volume_ml: f32,
    pub cardiac_output_l_min: f32,
    pub peripheral_resistance: f32,
    pub spo2_percent: f32,
    pub pao2_mmhg: f32,
    pub paco2_mmhg: f32,
    pub sympathetic: f32,
    pub parasympathetic: f32,
    pub adrenaline: f32,
    pub blood_volume_ml: f32,
    pub blood_lost_ml: f32,
}

impl Default for Telemetry {
    fn default() -> Self {
        Self {
            status: Status::Alive,
            time_s: 0.0,
            heart_rate_bpm: 70.0,
            cardiac_phase: 0.0,
            ventricular_activation: 0.0,
            lv_volume_ml: 120.0,
            lv_pressure_mmhg: 5.0,
            arterial_pressure_mmhg: 85.0,
            venous_pressure_mmhg: 6.0,
            mean_arterial_pressure_mmhg: 93.0,
            systolic_mmhg: 120.0,
            diastolic_mmhg: 80.0,
            aortic_flow_ml_s: 0.0,
            stroke_volume_ml: 70.0,
            cardiac_output_l_min: 4.9,
            peripheral_resistance: 1.05,
            spo2_percent: 98.0,
            pao2_mmhg: 100.0,
            paco2_mmhg: 40.0,
            sympathetic: 0.3,
            parasympathetic: 0.6,
            adrenaline: 0.0,
            blood_volume_ml: 5000.0,
            blood_lost_ml: 0.0,
        }
    }
}

/// Échantillon de forme d'onde pour l'inspecteur télémétrique (oscilloscope).
#[derive(Clone, Copy, Debug, Default)]
pub struct WaveSample {
    pub time_s: f64,
    pub arterial_mmhg: f32,
    pub lv_mmhg: f32,
    pub aortic_flow_ml_s: f32,
    pub ecg_mv: f32,
}

#[inline]
fn gaussian(x: f64, mu: f64, sigma: f64, amp: f64) -> f64 {
    let z = (x - mu) / sigma;
    amp * (-0.5 * z * z).exp()
}

/// ECG synthétique (dérivation II) : somme de gaussiennes P, Q, R, S, T.
/// `t_beat` = temps écoulé depuis le début du battement (s).
pub fn ecg_lead_ii(t_beat: f64, period_s: f64, systole_s: f64) -> f64 {
    let t_prev = t_beat - period_s; // onde P du battement suivant
    gaussian(t_prev, -0.17, 0.025, 0.15)
        + gaussian(t_beat, -0.01, 0.008, -0.12)
        + gaussian(t_beat, 0.01, 0.010, 1.0)
        + gaussian(t_beat, 0.035, 0.010, -0.25)
        + gaussian(t_beat, 0.8 * systole_s, 0.04, 0.30)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ecg_r_wave_dominates() {
        let r = ecg_lead_ii(0.01, 0.857, 0.34);
        let baseline = ecg_lead_ii(0.45, 0.857, 0.34);
        assert!(r > 0.9 && baseline.abs() < 0.05);
    }
}
