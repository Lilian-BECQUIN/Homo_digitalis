//! Moteur biomécanique et cinématique des activités humaines (marche, course, sprint, saut, squat).
//! Anime simultanément le squelette, les vertèbres, les muscles, les viscères, la peau et le système vasculaire,
//! tout en pilotant l'effort métabolique et le tableau de bord physiologique.

use bevy::prelude::*;

use crate::anatomy::{BaseScale, MusclesMarker, VertebraeMarker, VisceraMarker};
use crate::layers::{FemaleModel, Gender, MaleModel};

#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum ActivityType {
    #[default]
    Idle,
    Walk,
    Run,
    Sprint,
    Jump,
    Squat,
}

impl ActivityType {
    pub const ALL: [ActivityType; 6] = [
        ActivityType::Idle,
        ActivityType::Walk,
        ActivityType::Run,
        ActivityType::Sprint,
        ActivityType::Jump,
        ActivityType::Squat,
    ];

    pub fn label(self) -> &'static str {
        match self {
            ActivityType::Idle => "🧘 Repos (Calme)",
            ActivityType::Walk => "🚶 Marche (5 km/h)",
            ActivityType::Run => "🏃 Course (12 km/h)",
            ActivityType::Sprint => "⚡ Sprint (24 km/h)",
            ActivityType::Jump => "🦘 Saut Vertical",
            ActivityType::Squat => "🏋 Squat (Flexions)",
        }
    }

    pub fn short_name(self) -> &'static str {
        match self {
            ActivityType::Idle => "Repos",
            ActivityType::Walk => "Marche",
            ActivityType::Run => "Course",
            ActivityType::Sprint => "Sprint",
            ActivityType::Jump => "Saut",
            ActivityType::Squat => "Squat",
        }
    }

    pub fn description(self) -> &'static str {
        match self {
            ActivityType::Idle => "Postural au repos, ventilation basale eupnéique.",
            ActivityType::Walk => "Locomotion active, cadence 112 pas/min, résonance hémodynamique.",
            ActivityType::Run => "Foulée aérobie soutenue, impact 2,2 G, pompe veino-musculaire accrue.",
            ActivityType::Sprint => "Effort anaérobie maximal, recrutement complet des unités motrices.",
            ActivityType::Jump => "Impulsion explosive, phase aérienne balistique et amorti rachidien.",
            ActivityType::Squat => "Contrainte lombaire L4-L5 quadruplée, charge isométrique quadriceps.",
        }
    }
}

#[derive(Resource)]
pub struct ActivityState {
    pub current: ActivityType,
    pub speed_kmh: f32,
    pub cadence_spm: f32,
    pub impact_g: f32,
    pub vo2_ml_kg_min: f32,
    pub mets: f32,
    pub power_watts: f32,
    pub calories_kcal_h: f32,
    pub breath_rate_rpm: f32,
    pub minute_ventilation_l_min: f32,
    // Timers pour les cycles cinématiques continus
    pub cycle_time: f32,
    pub jump_timer: f32,
    // Effort cible et lissé envoyé au modèle cardiovasculaire
    pub current_effort: f64,
    pub target_effort: f64,
}

impl Default for ActivityState {
    fn default() -> Self {
        Self {
            current: ActivityType::Idle,
            speed_kmh: 0.0,
            cadence_spm: 0.0,
            impact_g: 1.0,
            vo2_ml_kg_min: 3.5,
            mets: 1.0,
            power_watts: 75.0,
            calories_kcal_h: 70.0,
            breath_rate_rpm: 14.0,
            minute_ventilation_l_min: 7.0,
            cycle_time: 0.0,
            jump_timer: 0.0,
            current_effort: 0.0,
            target_effort: 0.0,
        }
    }
}

impl ActivityState {
    pub fn set_activity(&mut self, act: ActivityType) {
        if self.current == act {
            return;
        }
        self.current = act;
        self.jump_timer = 0.0;
        self.target_effort = match act {
            ActivityType::Idle => 0.00,
            ActivityType::Walk => 0.25,
            ActivityType::Run => 0.65,
            ActivityType::Sprint => 1.00,
            ActivityType::Jump => 0.75,
            ActivityType::Squat => 0.55,
        };
    }
}

/// Met à jour les paramètres métaboliques et la biomécanique en temps réel.
fn update_activity_physics(
    time: Res<Time>,
    mut act: ResMut<ActivityState>,
) {
    let dt = time.delta_secs();
    act.cycle_time += dt;
    act.jump_timer += dt;

    // Lissage exponentiel de l'effort physiologique (adaptation cardiovasculaire réaliste en 1,5 s)
    let k = 1.0 - (-2.2 * dt as f64).exp();
    act.current_effort += (act.target_effort - act.current_effort) * k;

    // Calcul des cibles d'après l'activité courante
    let (tgt_spd, tgt_cad, tgt_vo2, tgt_mets, tgt_w, tgt_kcal, tgt_rpm, tgt_vent) = match act.current {
        ActivityType::Idle => (0.0, 0.0, 3.5, 1.0, 75.0, 70.0, 14.0, 7.0),
        ActivityType::Walk => (5.0, 112.0, 14.5, 4.1, 260.0, 275.0, 22.0, 24.0),
        ActivityType::Run => (12.0, 165.0, 38.0, 10.8, 760.0, 740.0, 36.0, 56.0),
        ActivityType::Sprint => (24.0, 220.0, 56.0, 16.0, 1250.0, 1150.0, 52.0, 95.0),
        ActivityType::Jump => (3.5, 60.0, 32.0, 9.1, 620.0, 600.0, 30.0, 48.0),
        ActivityType::Squat => (1.5, 30.0, 25.0, 7.1, 480.0, 460.0, 28.0, 40.0),
    };

    let blend = 1.0 - (-3.0 * dt).exp();
    act.speed_kmh += (tgt_spd - act.speed_kmh) * blend;
    act.cadence_spm += (tgt_cad - act.cadence_spm) * blend;
    act.vo2_ml_kg_min += (tgt_vo2 - act.vo2_ml_kg_min) * blend;
    act.mets += (tgt_mets - act.mets) * blend;
    act.power_watts += (tgt_w - act.power_watts) * blend;
    act.calories_kcal_h += (tgt_kcal - act.calories_kcal_h) * blend;
    act.breath_rate_rpm += (tgt_rpm - act.breath_rate_rpm) * blend;
    act.minute_ventilation_l_min += (tgt_vent - act.minute_ventilation_l_min) * blend;
}

/// Anime la cinématique globale du corps humain (translation, rebonds, oscillations, inclinaisons)
/// pour que les 9 calques anatomiques (os, vertèbres, muscles, vaisseaux, organes, nerfs, peau)
/// bougent en parfaite cohérence.
#[allow(clippy::type_complexity)]
fn animate_biomechanics(
    _time: Res<Time>,
    gender: Res<Gender>,
    mut act: ResMut<ActivityState>,
    mut transforms: ParamSet<(
        Query<(&mut Transform, Option<&MaleModel>, Option<&FemaleModel>)>,
        Query<(&BaseScale, &mut Transform), With<VertebraeMarker>>,
        Query<(&BaseScale, &mut Transform), With<VisceraMarker>>,
        Query<(&BaseScale, &mut Transform), With<MusclesMarker>>,
    )>,
) {
    let t = act.cycle_time;
    let jt = act.jump_timer;

    let mut trans = Vec3::ZERO;
    let mut pitch = 0.0_f32;
    let mut roll = 0.0_f32;
    let mut yaw = 0.0_f32;
    let mut current_g = 1.0_f32;
    let mut vert_compression = 0.0_f32;

    match act.current {
        ActivityType::Idle => {
            // Respiration lente au repos (cycle de ~4.3 s) et micro-balancement postural
            let resp = (t * 0.233 * core::f32::consts::TAU).sin();
            trans.y = resp * 0.004;
            pitch = resp * 0.008;
            roll = (t * 0.35).sin() * 0.003;
            current_g = 1.0;
            vert_compression = 0.0;
        }
        ActivityType::Walk => {
            let cad_hz = 112.0 / 60.0;
            let omega = cad_hz * core::f32::consts::TAU;
            // 2 pas par cycle : oscillation verticale
            let bounce = (omega * 2.0 * t).sin();
            trans.y = bounce * 0.032;
            // Balancement latéral vers le pied d'appui
            trans.x = (omega * t).sin() * 0.022;
            roll = -(omega * t).sin() * 0.035;
            yaw = (omega * t).cos() * 0.045;
            pitch = 0.035; // légère inclinaison avant
            current_g = 1.0 + bounce.max(0.0) * 0.35;
            vert_compression = bounce.max(0.0) * 0.025;
        }
        ActivityType::Run => {
            let cad_hz = 165.0 / 60.0;
            let omega = cad_hz * core::f32::consts::TAU;
            let bounce = (omega * 2.0 * t).sin();
            trans.y = bounce * 0.068;
            trans.x = (omega * t).sin() * 0.028;
            roll = -(omega * t).sin() * 0.055;
            yaw = (omega * t).cos() * 0.075;
            pitch = 0.125 + bounce * 0.025; // inclinaison de course ~7.5°
            current_g = 1.0 + bounce.max(0.0) * 1.25;
            vert_compression = bounce.max(0.0) * 0.055;
        }
        ActivityType::Sprint => {
            let cad_hz = 220.0 / 60.0;
            let omega = cad_hz * core::f32::consts::TAU;
            let bounce = (omega * 2.0 * t).sin();
            trans.y = bounce * 0.092;
            trans.x = (omega * t).sin() * 0.035;
            roll = -(omega * t).sin() * 0.070;
            yaw = (omega * t).cos() * 0.095;
            pitch = 0.22 + bounce * 0.035; // attaque agressive ~13°
            current_g = 1.0 + bounce.max(0.0) * 2.2;
            vert_compression = bounce.max(0.0) * 0.085;
        }
        ActivityType::Jump => {
            // Cycle de saut périodique complet (durée 2.0 s)
            let cycle = 2.0_f32;
            let phase_t = jt % cycle;

            if phase_t < 0.45 {
                // Phase 1 : Flexion / Armement
                let p = phase_t / 0.45;
                let s = (p * core::f32::consts::FRAC_PI_2).sin();
                trans.y = -s * 0.26;
                pitch = s * 0.16;
                current_g = 1.0 - s * 0.4;
                vert_compression = 0.01;
            } else if phase_t < 0.70 {
                // Phase 2 : Impulsion explosive
                let p = (phase_t - 0.45) / 0.25;
                trans.y = -0.26 + p * 0.26;
                pitch = 0.16 * (1.0 - p);
                let impulse = (p * core::f32::consts::PI).sin();
                current_g = 1.0 + impulse * 1.85; // pic à 2.85 G
                vert_compression = 0.04 * impulse;
            } else if phase_t < 1.35 {
                // Phase 3 : Vol balistique libre (suspension dans les airs)
                let p = (phase_t - 0.70) / 0.65;
                let arc = (p * core::f32::consts::PI).sin();
                trans.y = arc * 0.45; // élévation de 45 cm au-dessus du sol
                pitch = -0.06 * arc;
                current_g = 0.05; // quasi-apesanteur en chute libre
                vert_compression = 0.0;
            } else {
                // Phase 4 : Réception & Amorti au sol
                let p = (phase_t - 1.35) / 0.65;
                let decel = (p * core::f32::consts::PI).sin() * (1.0 - 0.4 * p);
                trans.y = -decel * 0.16;
                pitch = decel * 0.12;
                let impact_pulse = (p * core::f32::consts::FRAC_PI_2).sin() * (1.0 - p);
                current_g = 1.0 + impact_pulse * 1.75; // pic de contact 2.75 G
                vert_compression = impact_pulse * 0.065; // forte compression des disques
            }
        }
        ActivityType::Squat => {
            // Flexion profonde contrôlée (cycle de 2.4 s)
            let cycle = 2.4_f32;
            let theta = (jt % cycle) / cycle * core::f32::consts::TAU;
            let c = (1.0 - theta.cos()) * 0.5; // 0 (haut) -> 1 (bas) -> 0
            trans.y = -c * 0.38; // descente profonde de 38 cm
            pitch = c * 0.24;   // inclinaison lombaire pour équilibre
            current_g = 1.0 + c * 2.5; // charge lombaire quadruplée en position basse
            vert_compression = c * 0.045;
        }
    }

    act.impact_g = current_g;

    // 1. Application aux modèles racines (Homme / Femme) : tous les 9 calques bougent ensemble !
    for (mut tf, male, female) in &mut transforms.p0() {
        let is_active = (*gender == Gender::Male && male.is_some())
            || (*gender == Gender::Female && female.is_some());
        if is_active {
            tf.translation = trans;
            tf.rotation = Quat::from_euler(EulerRot::YXZ, yaw, -pitch, roll);
        }
    }

    // 2. Colonne vertébrale (Vertebrae) : micro-compression élastique à chaque impact
    for (base, mut tf) in &mut transforms.p1() {
        let comp = vert_compression.clamp(0.0, 0.12);
        tf.scale = Vec3::new(
            base.0.x * (1.0 + comp * 0.35),
            base.0.y * (1.0 - comp),
            base.0.z * (1.0 + comp * 0.35),
        );
    }

    // 3. Poumons & Viscères (Viscera) : ventilation pulmonaire en direct (rythme d'effort)
    let resp_hz = act.breath_rate_rpm / 60.0;
    let breath_wave = (t * resp_hz * core::f32::consts::TAU).sin();
    let effort_factor = 1.0 + act.current_effort as f32 * 0.8;
    let breath_expand = breath_wave * 0.045 * effort_factor;
    for (base, mut tf) in &mut transforms.p2() {
        tf.scale = Vec3::new(
            base.0.x * (1.0 + breath_expand * 0.8),
            base.0.y * (1.0 + breath_expand * 0.4),
            base.0.z * (1.0 + breath_expand),
        );
    }

    // 4. Muscles : contraction et tonus musculaire d'effort
    let muscle_pump = 1.0 + (act.current_effort as f32) * 0.05;
    for (base, mut tf) in &mut transforms.p3() {
        tf.scale = Vec3::new(
            base.0.x * muscle_pump,
            base.0.y,
            base.0.z * muscle_pump,
        );
    }
}

pub struct ActivityPlugin;

impl Plugin for ActivityPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<ActivityState>()
            .add_systems(Update, (update_activity_physics, animate_biomechanics));
    }
}
