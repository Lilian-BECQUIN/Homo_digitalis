//! Paramètres du modèle hémodynamique (unités : mL, mmHg, s ; résistances en mmHg·s/mL,
//! compliances en mL/mmHg, élastances en mmHg/mL).

#[derive(Clone, Copy, Debug)]
pub struct HemodynamicParams {
    // --- Ventricule gauche (élastance variable de Suga) ---
    pub e_max_rest: f64,
    pub e_min: f64,
    /// Volume non contraint du VG (mL).
    pub v0_lv: f64,
    // --- Windkessel à 3 éléments + retour veineux ---
    pub c_art: f64,
    pub c_ven: f64,
    /// Impédance caractéristique aortique (Rc).
    pub r_char: f64,
    /// Résistance de la valve mitrale.
    pub r_mitral: f64,
    /// Résistance périphérique de repos (Rp).
    pub r_periph_rest: f64,
    // --- Système nerveux autonome (baroréflexe) ---
    pub symp_rest: f64,
    pub para_rest: f64,
    pub baro_setpoint_symp_mmhg: f64,
    pub baro_setpoint_para_mmhg: f64,
    pub baro_width_mmhg: f64,
    pub tau_symp_s: f64,
    pub tau_para_s: f64,
    pub tau_map_s: f64,
    // --- Couplages ---
    pub hr_rest_bpm: f64,
    pub hr_gain_symp: f64,
    pub hr_gain_para: f64,
    pub contractility_gain: f64,
    pub resistance_radius_gain: f64,
    pub effort_vasodilation: f64,
    pub hypoxic_vasodilation: f64,
    pub venoconstriction_ml: f64,
    pub muscle_pump_ml: f64,
    pub adrenaline_sympathomimetic: f64,
    pub effort_symp_drive: f64,
    pub chemo_symp_drive: f64,
    pub chemo_para_drive: f64,
    /// Constante de temps d'élimination de l'adrénaline (s) ; demi-vie ≈ 2 min.
    pub tau_adrenaline_s: f64,
    // --- Garde-fous ---
    /// PAM filtrée sous laquelle le décompte d'arrêt circulatoire démarre (mmHg).
    pub arrest_map_threshold_mmhg: f64,
    /// Durée sous le seuil avant arrêt (s).
    pub arrest_delay_s: f64,
}

impl Default for HemodynamicParams {
    fn default() -> Self {
        Self {
            e_max_rest: 2.0,
            e_min: 0.05,
            v0_lv: 10.0,
            c_art: 1.4,
            c_ven: 60.0,
            r_char: 0.05,
            r_mitral: 0.01,
            r_periph_rest: 1.05,
            symp_rest: 0.3,
            para_rest: 0.6,
            baro_setpoint_symp_mmhg: 82.8,
            baro_setpoint_para_mmhg: 88.0,
            baro_width_mmhg: 12.0,
            tau_symp_s: 4.0,
            tau_para_s: 1.0,
            tau_map_s: 2.0,
            hr_rest_bpm: 70.0,
            hr_gain_symp: 110.0,
            hr_gain_para: 45.0,
            contractility_gain: 0.8,
            resistance_radius_gain: 0.12,
            effort_vasodilation: 0.75,
            hypoxic_vasodilation: 0.4,
            venoconstriction_ml: 400.0,
            muscle_pump_ml: 250.0,
            adrenaline_sympathomimetic: 0.5,
            effort_symp_drive: 0.7,
            chemo_symp_drive: 0.6,
            chemo_para_drive: 0.3,
            tau_adrenaline_s: 170.0,
            arrest_map_threshold_mmhg: 20.0,
            arrest_delay_s: 15.0,
        }
    }
}
