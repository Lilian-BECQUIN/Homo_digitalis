//! Boucle de simulation à pas fixe, garde-fous numériques, télémétrie et snapshots (rewind).

use std::collections::VecDeque;

use crate::hemodynamics::{
    self, clamp_state, initial_state, HemodynamicsModel, State, ADREN, MAP_FILT, PARA, PHASE, SYMP,
    V_LV,
};
use crate::inputs::Inputs;
use crate::safety::all_finite;
use crate::solver::rk4_step;
use crate::telemetry::{ecg_lead_ii, Status, Telemetry, WaveSample};

/// Volume sanguin total de référence (mL).
pub const NOMINAL_BLOOD_VOLUME_ML: f64 = 5000.0;
const MAX_PENDING_SAMPLES: usize = 4096;

/// Pas de temps recommandé selon le facteur d'accélération (RK4 reste stable jusqu'à ~10 ms ici).
pub fn recommended_dt(time_scale: f64) -> f64 {
    if time_scale <= 60.0 {
        0.001
    } else {
        0.002
    }
}

/// Copie complète de l'état mémoire de la simulation (base du Rewind).
#[derive(Clone, Debug)]
pub struct Snapshot {
    pub time_s: f64,
    state: State,
    status: Status,
    lost_ml: f64,
    inputs: Inputs,
}

/// Tampon circulaire de snapshots pris toutes les `interval_s` secondes simulées.
#[derive(Clone, Debug)]
pub struct SnapshotRing {
    interval_s: f64,
    capacity: usize,
    next_time_s: f64,
    items: VecDeque<Snapshot>,
}

impl SnapshotRing {
    pub fn new(interval_s: f64, capacity: usize) -> Self {
        Self {
            interval_s: interval_s.max(0.01),
            capacity: capacity.max(1),
            next_time_s: 0.0,
            items: VecDeque::with_capacity(capacity.max(1)),
        }
    }
    pub fn len(&self) -> usize {
        self.items.len()
    }
    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }
}

#[derive(Clone, Debug)]
pub struct Simulation {
    model: HemodynamicsModel,
    y: State,
    time_s: f64,
    accumulator_s: f64,
    status: Status,
    lost_ml: f64,
    low_map_s: f64,
    // Statistiques du battement en cours / précédent.
    beat_max: f64,
    beat_min: f64,
    beat_sv: f64,
    last_sbp: f64,
    last_dbp: f64,
    last_sv: f64,
    // Échantillons de forme d'onde (drainés par le runner).
    samples: Vec<WaveSample>,
    sample_acc_s: f64,
    sample_interval_s: f64,
    snapshots: SnapshotRing,
}

impl Default for Simulation {
    fn default() -> Self {
        Self::new()
    }
}

impl Simulation {
    pub fn new() -> Self {
        let y = initial_state();
        let p_art = y[hemodynamics::V_ART] / 1.4;
        Self {
            model: HemodynamicsModel::default(),
            y,
            time_s: 0.0,
            accumulator_s: 0.0,
            status: Status::Alive,
            lost_ml: 0.0,
            low_map_s: 0.0,
            beat_max: f64::MIN,
            beat_min: f64::MAX,
            beat_sv: 0.0,
            last_sbp: p_art,
            last_dbp: p_art,
            last_sv: 0.0,
            samples: Vec::new(),
            sample_acc_s: 0.0,
            sample_interval_s: 0.005,
            snapshots: SnapshotRing::new(5.0, 720),
        }
    }

    pub fn time_s(&self) -> f64 {
        self.time_s
    }
    pub fn status(&self) -> Status {
        self.status
    }
    pub fn state(&self) -> &State {
        &self.y
    }
    pub fn model(&self) -> &HemodynamicsModel {
        &self.model
    }
    pub fn inputs(&self) -> Inputs {
        self.model.inputs()
    }
    pub fn snapshots(&self) -> &SnapshotRing {
        &self.snapshots
    }

    pub fn set_inputs(&mut self, inputs: Inputs) {
        self.model.set_inputs(inputs);
    }

    /// Injection d'adrénaline (unités normalisées ; 1,0 ≈ effet sympathomimétique demi-maximal).
    pub fn inject_adrenaline(&mut self, units: f64) {
        let u = if units.is_finite() { units.clamp(0.0, 100.0) } else { 0.0 };
        self.y[ADREN] = (self.y[ADREN] + u).min(1000.0);
    }

    /// Intervalle (s simulées) entre deux échantillons de forme d'onde.
    pub fn set_sample_interval(&mut self, interval_s: f64) {
        self.sample_interval_s = if interval_s.is_finite() { interval_s.max(0.001) } else { 0.005 };
    }

    /// Récupère (et vide) les échantillons de forme d'onde accumulés.
    pub fn drain_samples(&mut self) -> Vec<WaveSample> {
        std::mem::take(&mut self.samples)
    }

    /// Avance de `sim_seconds` secondes simulées avec des pas fixes `dt`.
    /// Le nombre de pas est plafonné (`max_steps`) : si le budget est dépassé,
    /// le retard est abandonné pour ne jamais bloquer le thread (spirale de la mort).
    pub fn advance(&mut self, sim_seconds: f64, dt: f64, max_steps: u32) -> u32 {
        if !sim_seconds.is_finite() || sim_seconds <= 0.0 || !(dt > 0.0) {
            return 0;
        }
        self.accumulator_s += sim_seconds;
        let mut n = 0;
        while self.accumulator_s >= dt && n < max_steps {
            self.step(dt);
            self.accumulator_s -= dt;
            n += 1;
        }
        if self.accumulator_s >= dt {
            self.accumulator_s = 0.0;
        }
        n
    }

    /// Un pas de simulation (RK4 + bornes + détection d'arrêt).
    pub fn step(&mut self, dt: f64) {
        self.maybe_snapshot();

        let prev = self.y;
        let mut next = rk4_step(&self.model, self.time_s, &prev, dt);
        if !all_finite(&next) {
            // Divergence : on revient au dernier état sain et on fige le modèle.
            self.y = prev;
            self.status = Status::NumericalFault;
            self.model.arrested = true;
            return;
        }
        clamp_state(&mut next);
        self.y = next;
        self.time_s += dt;

        let a = self.model.algebraic(&self.y);
        self.beat_max = self.beat_max.max(a.p_art);
        self.beat_min = self.beat_min.min(a.p_art);
        self.beat_sv += a.q_out * dt;
        self.lost_ml += a.bleed_ml_s * dt;

        if self.y[PHASE] >= 1.0 {
            self.y[PHASE] = self.y[PHASE].fract();
            self.last_sbp = self.beat_max;
            self.last_dbp = self.beat_min;
            self.last_sv = self.beat_sv;
            self.beat_max = f64::MIN;
            self.beat_min = f64::MAX;
            self.beat_sv = 0.0;
        }

        // Arrêt biologique : pression moyenne trop basse trop longtemps.
        if self.status == Status::Alive {
            if self.y[MAP_FILT] < self.model.params.arrest_map_threshold_mmhg {
                self.low_map_s += dt;
                if self.low_map_s >= self.model.params.arrest_delay_s {
                    self.status = Status::CardiacArrest;
                    self.model.arrested = true;
                }
            } else {
                self.low_map_s = 0.0;
            }
        }

        self.sample_acc_s += dt;
        if self.sample_acc_s >= self.sample_interval_s {
            self.sample_acc_s = 0.0;
            if self.samples.len() < MAX_PENDING_SAMPLES {
                let period = if a.heart_rate_bpm > 0.0 { 60.0 / a.heart_rate_bpm } else { 1.0 };
                let ecg = if a.heart_rate_bpm > 0.0 {
                    ecg_lead_ii(self.y[PHASE] * period, period, a.systole_s)
                } else {
                    0.0
                };
                self.samples.push(WaveSample {
                    time_s: self.time_s,
                    arterial_mmhg: a.p_art as f32,
                    lv_mmhg: a.p_lv as f32,
                    aortic_flow_ml_s: a.q_out as f32,
                    ecg_mv: ecg as f32,
                });
            }
        }
    }

    /// Construit la télémétrie courante.
    pub fn telemetry(&self) -> Telemetry {
        let a = self.model.algebraic(&self.y);
        let gas = self.model.gas();
        let (sbp, dbp) = if self.status == Status::Alive {
            (self.last_sbp, self.last_dbp)
        } else {
            (a.p_art, a.p_art)
        };
        Telemetry {
            status: self.status,
            time_s: self.time_s,
            heart_rate_bpm: a.heart_rate_bpm as f32,
            cardiac_phase: self.y[PHASE] as f32,
            ventricular_activation: a.activation as f32,
            lv_volume_ml: self.y[V_LV] as f32,
            lv_pressure_mmhg: a.p_lv as f32,
            arterial_pressure_mmhg: a.p_art as f32,
            venous_pressure_mmhg: a.p_ven as f32,
            mean_arterial_pressure_mmhg: self.y[MAP_FILT] as f32,
            systolic_mmhg: sbp as f32,
            diastolic_mmhg: dbp as f32,
            aortic_flow_ml_s: a.q_out as f32,
            stroke_volume_ml: self.last_sv as f32,
            cardiac_output_l_min: (self.last_sv * a.heart_rate_bpm / 1000.0) as f32,
            peripheral_resistance: a.r_periph as f32,
            spo2_percent: (gas.spo2 * 100.0) as f32,
            pao2_mmhg: gas.pao2_mmhg as f32,
            paco2_mmhg: gas.paco2_mmhg as f32,
            sympathetic: self.y[SYMP] as f32,
            parasympathetic: self.y[PARA] as f32,
            adrenaline: self.y[ADREN] as f32,
            blood_volume_ml: (NOMINAL_BLOOD_VOLUME_ML - self.lost_ml) as f32,
            blood_lost_ml: self.lost_ml as f32,
        }
    }

    // ------------------------------------------------------------------ snapshots

    fn maybe_snapshot(&mut self) {
        if self.time_s >= self.snapshots.next_time_s {
            let snap = Snapshot {
                time_s: self.time_s,
                state: self.y,
                status: self.status,
                lost_ml: self.lost_ml,
                inputs: self.model.inputs(),
            };
            if self.snapshots.items.len() == self.snapshots.capacity {
                self.snapshots.items.pop_front();
            }
            self.snapshots.items.push_back(snap);
            self.snapshots.next_time_s = self.time_s + self.snapshots.interval_s;
        }
    }

    /// Revient `seconds_back` secondes simulées en arrière (au snapshot le plus proche antérieur).
    /// Renvoie `false` si aucun snapshot n'est disponible.
    pub fn rewind(&mut self, seconds_back: f64) -> bool {
        if !seconds_back.is_finite() || self.snapshots.items.is_empty() {
            return false;
        }
        let target = self.time_s - seconds_back.max(0.0);
        let idx = self
            .snapshots
            .items
            .iter()
            .rposition(|s| s.time_s <= target)
            .unwrap_or(0);
        let snap = self.snapshots.items[idx].clone();
        self.snapshots.items.truncate(idx + 1);
        self.snapshots.next_time_s = snap.time_s + self.snapshots.interval_s;

        self.y = snap.state;
        self.time_s = snap.time_s;
        self.status = snap.status;
        self.lost_ml = snap.lost_ml;
        self.model.arrested = snap.status != Status::Alive;
        self.model.set_inputs(snap.inputs);
        self.low_map_s = 0.0;
        self.accumulator_s = 0.0;
        self.beat_max = f64::MIN;
        self.beat_min = f64::MAX;
        self.beat_sv = 0.0;
        self.samples.clear();
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn run(sim: &mut Simulation, seconds: f64) {
        sim.advance(seconds, 0.001, u32::MAX);
    }

    fn settled() -> Simulation {
        let mut sim = Simulation::new();
        run(&mut sim, 40.0);
        sim
    }

    fn assert_finite(t: &Telemetry) {
        for v in [
            t.heart_rate_bpm,
            t.arterial_pressure_mmhg,
            t.venous_pressure_mmhg,
            t.lv_volume_ml,
            t.systolic_mmhg,
            t.diastolic_mmhg,
            t.cardiac_output_l_min,
            t.spo2_percent,
            t.blood_volume_ml,
        ] {
            assert!(v.is_finite(), "valeur non finie dans {t:?}");
        }
    }

    #[test]
    fn rest_converges_to_physiological_range() {
        let t = settled().telemetry();
        assert_eq!(t.status, Status::Alive);
        assert!((55.0..90.0).contains(&t.heart_rate_bpm), "HR={}", t.heart_rate_bpm);
        assert!((95.0..135.0).contains(&t.systolic_mmhg), "SBP={}", t.systolic_mmhg);
        assert!((60.0..90.0).contains(&t.diastolic_mmhg), "DBP={}", t.diastolic_mmhg);
        assert!((3.5..6.5).contains(&t.cardiac_output_l_min), "CO={}", t.cardiac_output_l_min);
        assert!(t.spo2_percent > 95.0);
    }

    #[test]
    fn blood_mass_is_conserved_without_hemorrhage() {
        let mut sim = Simulation::new();
        let total = |s: &Simulation| s.state()[0] + s.state()[1] + s.state()[2];
        let t0 = total(&sim);
        run(&mut sim, 20.0);
        assert!((total(&sim) - t0).abs() < 1e-6, "dérive de volume = {}", total(&sim) - t0);
    }

    #[test]
    fn effort_raises_heart_rate_and_output() {
        let rest = settled().telemetry();
        let mut sim = settled();
        sim.set_inputs(Inputs { effort: 0.8, ..Inputs::default() });
        run(&mut sim, 40.0);
        let t = sim.telemetry();
        assert!(t.heart_rate_bpm > rest.heart_rate_bpm + 25.0);
        assert!(t.cardiac_output_l_min > rest.cardiac_output_l_min * 1.8);
    }

    #[test]
    fn hemorrhage_triggers_tachycardia_and_hypotension() {
        let rest = settled().telemetry();
        let mut sim = settled();
        sim.set_inputs(Inputs { hemorrhage_rate_ml_s: 20.0, ..Inputs::default() });
        run(&mut sim, 20.0);
        let t = sim.telemetry();
        assert!(t.heart_rate_bpm > rest.heart_rate_bpm + 30.0, "HR={}", t.heart_rate_bpm);
        assert!(t.systolic_mmhg < rest.systolic_mmhg - 20.0, "SBP={}", t.systolic_mmhg);
        assert!(t.blood_lost_ml > 250.0);
    }

    #[test]
    fn exsanguination_ends_in_arrest_without_nan() {
        let mut sim = settled();
        sim.set_inputs(Inputs { hemorrhage_rate_ml_s: 200.0, ..Inputs::default() });
        run(&mut sim, 120.0);
        let t = sim.telemetry();
        assert_finite(&t);
        assert_eq!(t.status, Status::CardiacArrest);
        assert_eq!(t.heart_rate_bpm, 0.0);
    }

    #[test]
    fn anaphylaxis_drops_diastolic_pressure() {
        let rest = settled().telemetry();
        let mut sim = settled();
        sim.set_inputs(Inputs { vasodilation_factor: 0.3, ..Inputs::default() });
        run(&mut sim, 20.0);
        let t = sim.telemetry();
        assert!(t.diastolic_mmhg < rest.diastolic_mmhg - 10.0, "DBP={}", t.diastolic_mmhg);
        assert!(t.heart_rate_bpm > rest.heart_rate_bpm + 15.0);
    }

    #[test]
    fn altitude_lowers_spo2_and_raises_heart_rate() {
        let rest = settled().telemetry();
        let mut sim = settled();
        sim.set_inputs(Inputs { altitude_m: 8848.0, ..Inputs::default() });
        run(&mut sim, 30.0);
        let t = sim.telemetry();
        assert!(t.spo2_percent < 60.0);
        assert!(t.heart_rate_bpm > rest.heart_rate_bpm + 15.0);
        assert_finite(&t);
    }

    #[test]
    fn adrenaline_bolus_raises_heart_rate_then_decays() {
        let rest = settled().telemetry();
        let mut sim = settled();
        sim.inject_adrenaline(3.0);
        run(&mut sim, 10.0);
        assert!(sim.telemetry().heart_rate_bpm > rest.heart_rate_bpm + 15.0);
        run(&mut sim, 1200.0);
        assert!((sim.telemetry().heart_rate_bpm - rest.heart_rate_bpm).abs() < 8.0);
    }

    #[test]
    fn extreme_and_hostile_inputs_never_produce_nan() {
        let hostile = [
            Inputs { effort: f64::NAN, hemorrhage_rate_ml_s: 1e30, altitude_m: -1e9, vasodilation_factor: 0.0, viscosity_factor: f64::INFINITY },
            Inputs { effort: 1.0, hemorrhage_rate_ml_s: 0.0, altitude_m: 8848.0, vasodilation_factor: 3.0, viscosity_factor: 4.0 },
            Inputs { effort: 1.0, hemorrhage_rate_ml_s: 200.0, altitude_m: 8848.0, vasodilation_factor: 0.05, viscosity_factor: 0.4 },
        ];
        for inputs in hostile {
            let mut sim = settled();
            sim.set_inputs(inputs);
            sim.inject_adrenaline(f64::INFINITY);
            sim.inject_adrenaline(1e6);
            run(&mut sim, 60.0);
            assert_finite(&sim.telemetry());
        }
    }

    #[test]
    fn simulation_is_deterministic() {
        let mut a = Simulation::new();
        let mut b = Simulation::new();
        a.set_inputs(Inputs { effort: 0.5, ..Inputs::default() });
        b.set_inputs(Inputs { effort: 0.5, ..Inputs::default() });
        run(&mut a, 15.0);
        run(&mut b, 15.0);
        assert_eq!(a.state(), b.state());
    }

    #[test]
    fn rewind_restores_state_before_failure() {
        let mut sim = settled();
        let before = sim.telemetry();
        let t_before = sim.time_s();
        sim.set_inputs(Inputs { hemorrhage_rate_ml_s: 200.0, ..Inputs::default() });
        run(&mut sim, 60.0);
        assert!(sim.telemetry().blood_lost_ml > 1000.0);
        assert!(sim.rewind(60.0 + 5.0));
        assert!(sim.time_s() <= t_before + 1e-9);
        assert_eq!(sim.inputs(), Inputs::default());
        assert_eq!(sim.status(), Status::Alive);
        assert!(sim.telemetry().blood_lost_ml < 1.0);
        let _ = before;
    }

    #[test]
    fn step_budget_is_enforced() {
        let mut sim = Simulation::new();
        let n = sim.advance(1000.0, 0.001, 100);
        assert_eq!(n, 100);
        assert_eq!(sim.advance(0.0005, 0.001, 100), 0);
    }
}
