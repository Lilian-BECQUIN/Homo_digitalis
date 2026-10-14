//! Modèle hémodynamique en boucle fermée 0D :
//!
//! ```text
//!   VG (élastance variable) ──Rc──▶ Windkessel artériel (C_art) ──Rp──▶ réservoir veineux (C_ven) ──Rmv──▶ VG
//! ```
//!
//! Vecteur d'état (8 composantes, intégré en RK4) :
//! `[V_lv, V_art, V_ven, phase, S, Vg, PAM_filtrée, adrénaline]`.
//!
//! Les volumes sont des volumes *contraints* (stressed) : la conservation de la masse
//! sanguine est exacte en l'absence d'hémorragie.

use crate::gas::{self, GasExchange};
use crate::inputs::Inputs;
use crate::params::HemodynamicParams;
use crate::safety::{clamp01, logistic_decreasing, logistic_increasing};
use crate::solver::OdeSystem;
use crate::vessel::resistance_scale;

pub const V_LV: usize = 0;
pub const V_ART: usize = 1;
pub const V_VEN: usize = 2;
/// Phase du cycle cardiaque ∈ [0, 1).
pub const PHASE: usize = 3;
/// Activité sympathique ∈ [0, 1].
pub const SYMP: usize = 4;
/// Activité parasympathique (vagale) ∈ [0, 1].
pub const PARA: usize = 5;
/// Pression artérielle moyenne filtrée (capteur baroréflexe), mmHg.
pub const MAP_FILT: usize = 6;
/// Adrénaline plasmatique, unités normalisées (effet demi-maximal à 1,0).
pub const ADREN: usize = 7;
pub const N_STATES: usize = 8;

pub type State = [f64; N_STATES];

/// État initial proche de l'équilibre de repos (convergence en < 15 s simulées).
pub fn initial_state() -> State {
    [120.0, 1.4 * 85.0, 60.0 * 6.0, 0.0, 0.3, 0.6, 93.0, 0.0]
}

/// Bornes de sécurité de l'état (saturation dure en dernier recours).
pub fn clamp_state(y: &mut State) {
    y[V_LV] = y[V_LV].clamp(1.0, 800.0);
    y[V_ART] = y[V_ART].clamp(0.0, 1000.0);
    y[V_VEN] = y[V_VEN].clamp(-3000.0, 6000.0);
    y[SYMP] = clamp01(y[SYMP]);
    y[PARA] = clamp01(y[PARA]);
    y[MAP_FILT] = y[MAP_FILT].clamp(0.0, 800.0);
    y[ADREN] = y[ADREN].clamp(0.0, 1000.0);
}

/// Grandeurs algébriques dérivées de l'état (pression, débits, commandes).
#[derive(Clone, Copy, Debug, Default)]
pub struct Algebraic {
    pub heart_rate_bpm: f64,
    pub contractility_e_max: f64,
    /// Activation ventriculaire e(t) ∈ [0, 1].
    pub activation: f64,
    pub p_lv: f64,
    pub p_art: f64,
    pub p_ven: f64,
    pub q_in: f64,
    pub q_out: f64,
    pub q_periph: f64,
    pub r_periph: f64,
    pub bleed_ml_s: f64,
    pub sympathetic_effective: f64,
    pub hypoxia: f64,
    /// Durée de la systole ventriculaire (s).
    pub systole_s: f64,
}

#[derive(Clone, Debug)]
pub struct HemodynamicsModel {
    pub params: HemodynamicParams,
    inputs: Inputs,
    gas: GasExchange,
    /// Arrêt circulatoire : fréquence cardiaque nulle, le système relaxe passivement.
    pub arrested: bool,
}

impl Default for HemodynamicsModel {
    fn default() -> Self {
        Self::new(HemodynamicParams::default())
    }
}

impl HemodynamicsModel {
    pub fn new(params: HemodynamicParams) -> Self {
        let inputs = Inputs::default();
        Self {
            params,
            inputs,
            gas: gas::compute(inputs.altitude_m),
            arrested: false,
        }
    }

    pub fn inputs(&self) -> Inputs {
        self.inputs
    }

    pub fn gas(&self) -> GasExchange {
        self.gas
    }

    pub fn set_inputs(&mut self, inputs: Inputs) {
        let inputs = inputs.sanitized();
        if inputs.altitude_m != self.inputs.altitude_m {
            self.gas = gas::compute(inputs.altitude_m);
        }
        self.inputs = inputs;
    }

    /// Calcule toutes les grandeurs algébriques pour un état donné.
    pub fn algebraic(&self, y: &State) -> Algebraic {
        let p = &self.params;
        let inp = &self.inputs;
        let effort = inp.effort;

        let adren = y[ADREN].max(0.0);
        let s_eff = clamp01(y[SYMP] + p.adrenaline_sympathomimetic * adren / (adren + 1.0));
        let hypoxia = clamp01((95.0 - 100.0 * self.gas.spo2) / 35.0);

        // Nœud sinusal : fréquence modulée par sympathique (+) / parasympathique (−).
        let mut hr = (p.hr_rest_bpm + p.hr_gain_symp * (s_eff - p.symp_rest)
            - p.hr_gain_para * (y[PARA] - p.para_rest))
            .clamp(30.0, 220.0);
        if self.arrested {
            hr = 0.0;
        }

        let e_max = p.e_max_rest * (1.0 + p.contractility_gain * (s_eff - p.symp_rest));

        // Vasomotricité : r diminue avec le tonus sympathique → R ∝ η / r⁴ (Poiseuille).
        let radius_ratio =
            (1.0 - p.resistance_radius_gain * (s_eff - p.symp_rest)).clamp(0.5, 1.5);
        let r_periph = (p.r_periph_rest
            * resistance_scale(inp.viscosity_factor, radius_ratio)
            * (1.0 - p.effort_vasodilation * effort)
            * (1.0 - p.hypoxic_vasodilation * hypoxia)
            * inp.vasodilation_factor)
            .clamp(0.05, 20.0);

        // Retour veineux : veinoconstriction sympathique + pompe musculaire.
        let v_shift = p.venoconstriction_ml * (s_eff - p.symp_rest) + p.muscle_pump_ml * effort;
        let p_ven = ((y[V_VEN] + v_shift) / p.c_ven).max(0.0);
        let p_art = y[V_ART] / p.c_art;

        // Activation ventriculaire (demi-sinus sur la durée de systole).
        let (activation, systole_s) = if hr > 0.0 {
            let period = 60.0 / hr;
            let systole = (0.37 * period.sqrt()).min(0.8 * period);
            let t_beat = y[PHASE] * period;
            let e = if t_beat < systole {
                (core::f64::consts::PI * t_beat / systole).sin()
            } else {
                0.0
            };
            (e, systole)
        } else {
            (0.0, 0.0)
        };

        let elastance = p.e_min + (e_max - p.e_min) * activation;
        let p_lv = (elastance * (y[V_LV] - p.v0_lv)).max(0.0);

        // Valves = diodes.
        let q_in = (p_ven - p_lv).max(0.0) / p.r_mitral;
        let q_out = (p_lv - p_art).max(0.0) / p.r_char;
        let q_periph = (p_art - p_ven).max(0.0) / r_periph;
        let bleed = inp.hemorrhage_rate_ml_s * clamp01(p_art / 60.0);

        Algebraic {
            heart_rate_bpm: hr,
            contractility_e_max: e_max,
            activation,
            p_lv,
            p_art,
            p_ven,
            q_in,
            q_out,
            q_periph,
            r_periph,
            bleed_ml_s: bleed,
            sympathetic_effective: s_eff,
            hypoxia,
            systole_s,
        }
    }
}

impl OdeSystem<N_STATES> for HemodynamicsModel {
    fn derivatives(&self, _t: f64, y: &State) -> State {
        let p = &self.params;
        let a = self.algebraic(y);
        let effort = self.inputs.effort;

        // Commande baroréflexe (cibles) + chémoréflexe + effort.
        let symp_target = clamp01(
            logistic_decreasing((y[MAP_FILT] - p.baro_setpoint_symp_mmhg) / p.baro_width_mmhg)
                + p.effort_symp_drive * effort
                + p.chemo_symp_drive * a.hypoxia,
        );
        let para_target = clamp01(
            logistic_increasing((y[MAP_FILT] - p.baro_setpoint_para_mmhg) / p.baro_width_mmhg)
                * (1.0 - effort)
                - p.chemo_para_drive * a.hypoxia,
        );

        let mut d = [0.0; N_STATES];
        d[V_LV] = a.q_in - a.q_out;
        d[V_ART] = a.q_out - a.q_periph - a.bleed_ml_s;
        d[V_VEN] = a.q_periph - a.q_in;
        d[PHASE] = a.heart_rate_bpm / 60.0;
        d[SYMP] = (symp_target - y[SYMP]) / p.tau_symp_s;
        d[PARA] = (para_target - y[PARA]) / p.tau_para_s;
        d[MAP_FILT] = (a.p_art - y[MAP_FILT]) / p.tau_map_s;
        d[ADREN] = -y[ADREN] / p.tau_adrenaline_s;
        d
    }
}
