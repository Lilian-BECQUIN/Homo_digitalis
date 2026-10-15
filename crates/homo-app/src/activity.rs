//! Moteur biomécanique et cinématique réelle des articulations humaines (marche, course, sprint, saut, squat).
//! Anime les 15 articulations squelettiques (hanches, genoux, chevilles, épaules, coudes, colonne, cou)
//! tout en coordonnant les 9 calques anatomiques (os, muscles, vaisseaux, viscères, peau) et le métabolisme.

use core::f32::consts::{FRAC_PI_2, PI, TAU};
use bevy::prelude::*;

use crate::anatomy::{BaseJointTransform, BaseScale, BodyJoint, MusclesMarker, VertebraeMarker, VisceraMarker};
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
            ActivityType::Jump => "🦘 Saut Vertical (CMJ)",
            ActivityType::Squat => "🏋 Squat (Flexions 90°)",
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
            ActivityType::Walk => "Locomotion active, cadence 112 pas/min, balancement pendulaire.",
            ActivityType::Run => "Foulée aérobie soutenue (165 pas/min), flexion genou 85°, pompe musculaire.",
            ActivityType::Sprint => "Sprint maximal (220 pas/min), triple extension explosive, flexion genou 115°.",
            ActivityType::Jump => "Saut CMJ 5 phases : flexion, triple extension 2,85 G, vol libre, amorti 2,75 G.",
            ActivityType::Squat => "Flexion bilatérale 90°, cuisses horizontales, charge quadriceps 480 W.",
        }
    }
}

/// État cinématique, dynamique et métabolique de l'activité en cours.
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

    pub cycle_time: f32,
    pub jump_timer: f32,

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

    let k = 1.0 - (-2.2 * dt as f64).exp();
    act.current_effort += (act.target_effort - act.current_effort) * k;

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

/// Anime la cinématique réelle des 15 articulations (Hanches, Genoux, Chevilles, Épaules, Coudes, Colonne).
#[allow(clippy::type_complexity)]
fn animate_biomechanics(
    _time: Res<Time>,
    gender: Res<Gender>,
    mut act: ResMut<ActivityState>,
    mut set: ParamSet<(
        Query<(&mut Transform, Option<&MaleModel>, Option<&FemaleModel>)>,
        Query<(&BodyJoint, &BaseJointTransform, &mut Transform)>,
        Query<(&BaseScale, &mut Transform), With<VertebraeMarker>>,
        Query<(&BaseScale, &mut Transform), With<VisceraMarker>>,
        Query<(&BaseScale, &mut Transform), With<MusclesMarker>>,
    )>,
) {
    let t = act.cycle_time;
    let jt = act.jump_timer;

    let mut current_g = 1.0_f32;
    let mut vert_compression = 0.0_f32;

    let mut hips_offset = Vec3::ZERO;
    let mut hips_rot = Quat::IDENTITY;
    let mut spine_rot = Quat::IDENTITY;
    let mut thorax_rot = Quat::IDENTITY;
    let mut neck_rot = Quat::IDENTITY;
    let mut head_rot = Quat::IDENTITY;

    let mut l_thigh_rot = Quat::IDENTITY;
    let mut l_calf_rot = Quat::IDENTITY;
    let mut l_foot_rot = Quat::IDENTITY;

    let mut r_thigh_rot = Quat::IDENTITY;
    let mut r_calf_rot = Quat::IDENTITY;
    let mut r_foot_rot = Quat::IDENTITY;

    let mut l_upperarm_rot = Quat::IDENTITY;
    let mut l_forearm_rot = Quat::IDENTITY;
    let mut l_hand_rot = Quat::IDENTITY;

    let mut r_upperarm_rot = Quat::IDENTITY;
    let mut r_forearm_rot = Quat::IDENTITY;
    let mut r_hand_rot = Quat::IDENTITY;

    match act.current {
        ActivityType::Idle => {
            let resp = (t * 0.233 * TAU).sin();
            hips_offset.y = resp * 0.003;
            thorax_rot = Quat::from_rotation_x(resp * 0.012);
            hips_rot = Quat::from_euler(EulerRot::YXZ, (t * 0.35).sin() * 0.005, resp * 0.004, (t * 0.25).cos() * 0.003);
            current_g = 1.0;
            vert_compression = 0.0;
        }
        ActivityType::Walk => {
            let cad_hz = 112.0 / 60.0;
            let omega = cad_hz * TAU;
            let phi_l = omega * t;
            let phi_r = phi_l + PI;

            let bounce = (omega * 2.0 * t).sin();
            let sway = (omega * t).sin();

            hips_offset.y = bounce * 0.032;
            hips_offset.x = sway * 0.022;

            hips_rot = Quat::from_euler(EulerRot::YXZ, -sway * 0.045, 0.035, -sway * 0.035);
            thorax_rot = Quat::from_euler(EulerRot::YXZ, sway * 0.040, 0.020, 0.0);

            // Flexion / extension hanches
            l_thigh_rot = Quat::from_rotation_x(phi_l.sin() * 0.38);
            r_thigh_rot = Quat::from_rotation_x(phi_r.sin() * 0.38);

            // Flexion genoux
            let l_knee = (phi_l - 0.3).cos().max(0.0) * 0.85;
            let r_knee = (phi_r - 0.3).cos().max(0.0) * 0.85;
            l_calf_rot = Quat::from_rotation_x(l_knee);
            r_calf_rot = Quat::from_rotation_x(r_knee);

            // Chevilles
            l_foot_rot = Quat::from_rotation_x(phi_l.sin() * 0.20);
            r_foot_rot = Quat::from_rotation_x(phi_r.sin() * 0.20);

            // Balancement des bras en opposition croisée
            l_upperarm_rot = Quat::from_rotation_x(-phi_l.sin() * 0.35);
            r_upperarm_rot = Quat::from_rotation_x(-phi_r.sin() * 0.35);
            l_forearm_rot = Quat::from_rotation_x(0.35);
            r_forearm_rot = Quat::from_rotation_x(0.35);

            current_g = 1.0 + bounce.max(0.0) * 0.35;
            vert_compression = bounce.max(0.0) * 0.025;
        }
        ActivityType::Run | ActivityType::Sprint => {
            let is_sprint = act.current == ActivityType::Sprint;
            let cad_hz = if is_sprint { 220.0 / 60.0 } else { 165.0 / 60.0 };
            let omega = cad_hz * TAU;
            let phi_l = omega * t;
            let phi_r = phi_l + PI;

            let bounce = (omega * 2.0 * t).sin();
            let sway = (omega * t).sin();

            let a_thigh = if is_sprint { 0.95 } else { 0.65 };
            let a_knee = if is_sprint { 1.95 } else { 1.45 };
            let a_arm = if is_sprint { 1.15 } else { 0.75 };
            let pitch = if is_sprint { 0.22 } else { 0.14 };

            hips_offset.y = bounce * if is_sprint { 0.088 } else { 0.062 };
            hips_offset.x = sway * 0.028;

            hips_rot = Quat::from_euler(EulerRot::YXZ, -sway * 0.075, pitch, -sway * 0.055);
            thorax_rot = Quat::from_euler(EulerRot::YXZ, sway * 0.085, 0.040, 0.0);

            // Hanches : foulée puissante alternée
            l_thigh_rot = Quat::from_rotation_x(phi_l.sin() * a_thigh);
            r_thigh_rot = Quat::from_rotation_x(phi_r.sin() * a_thigh);

            // Genoux : forte flexion au retour de jambe (talon sous fessier)
            let l_knee = (phi_l - 0.4).cos().max(0.0) * a_knee;
            let r_knee = (phi_r - 0.4).cos().max(0.0) * a_knee;
            l_calf_rot = Quat::from_rotation_x(l_knee);
            r_calf_rot = Quat::from_rotation_x(r_knee);

            // Chevilles : impulsion plantaire
            l_foot_rot = Quat::from_rotation_x(phi_l.sin() * 0.35);
            r_foot_rot = Quat::from_rotation_x(phi_r.sin() * 0.35);

            // Bras : pompage athlétique à 90° opposé aux jambes
            l_upperarm_rot = Quat::from_rotation_x(-phi_l.sin() * a_arm);
            r_upperarm_rot = Quat::from_rotation_x(-phi_r.sin() * a_arm);
            l_forearm_rot = Quat::from_rotation_x(1.45 + phi_l.sin() * 0.30);
            r_forearm_rot = Quat::from_rotation_x(1.45 + phi_r.sin() * 0.30);

            current_g = 1.0 + bounce.max(0.0) * if is_sprint { 2.2 } else { 1.25 };
            vert_compression = bounce.max(0.0) * if is_sprint { 0.085 } else { 0.055 };
        }
        ActivityType::Jump => {
            // Cycle de saut complet CMJ (durée 2.0 s)
            let cycle = 2.0_f32;
            let phase_t = jt % cycle;

            if phase_t < 0.45 {
                // Phase 1 : Armement / Flexion préliminaire (knees bend, hips sink, arms back)
                let p = phase_t / 0.45;
                let s = (p * FRAC_PI_2).sin();

                hips_offset.y = -s * 0.28;
                hips_rot = Quat::from_rotation_x(s * 0.22);
                thorax_rot = Quat::from_rotation_x(s * 0.16);

                l_thigh_rot = Quat::from_rotation_x(-s * 0.70); // hanches fléchies -40°
                r_thigh_rot = Quat::from_rotation_x(-s * 0.70);

                l_calf_rot = Quat::from_rotation_x(s * 1.40);   // genoux fléchis +80°
                r_calf_rot = Quat::from_rotation_x(s * 1.40);

                l_foot_rot = Quat::from_rotation_x(-s * 0.35);  // dorsiflexion
                r_foot_rot = Quat::from_rotation_x(-s * 0.35);

                l_upperarm_rot = Quat::from_rotation_x(-s * 0.75); // bras en arrière
                r_upperarm_rot = Quat::from_rotation_x(-s * 0.75);
                l_forearm_rot = Quat::from_rotation_x(s * 0.45);
                r_forearm_rot = Quat::from_rotation_x(s * 0.45);

                current_g = 1.0 - s * 0.4;
                vert_compression = 0.01;
            } else if phase_t < 0.70 {
                // Phase 2 : Impulsion explosive / Triple Extension (snap knees, push toes, arms overhead)
                let p = (phase_t - 0.45) / 0.25;
                let s = 1.0 - p;

                hips_offset.y = -s * 0.28 + p * 0.12;
                hips_rot = Quat::from_rotation_x(s * 0.22 - p * 0.08);
                thorax_rot = Quat::from_rotation_x(-p * 0.10);

                l_thigh_rot = Quat::from_rotation_x(-s * 0.70); // extension complète
                r_thigh_rot = Quat::from_rotation_x(-s * 0.70);

                l_calf_rot = Quat::from_rotation_x(s * 1.40);   // genoux tendus à 0°
                r_calf_rot = Quat::from_rotation_x(s * 1.40);

                l_foot_rot = Quat::from_rotation_x(p * 0.65);   // poussée sur la pointe +37°
                r_foot_rot = Quat::from_rotation_x(p * 0.65);

                l_upperarm_rot = Quat::from_rotation_x(p * 1.45); // bras propulsés vers le haut +83°
                r_upperarm_rot = Quat::from_rotation_x(p * 1.45);
                l_forearm_rot = Quat::from_rotation_x(0.30);
                r_forearm_rot = Quat::from_rotation_x(0.30);

                let impulse = (p * PI).sin();
                current_g = 1.0 + impulse * 1.85; // pic de décollage à 2,85 G
                vert_compression = 0.04 * impulse;
            } else if phase_t < 1.35 {
                // Phase 3 : Vol balistique libre / Apex en apesanteur (+45 cm)
                let p = (phase_t - 0.70) / 0.65;
                let arc = (p * PI).sin();

                hips_offset.y = 0.12 + arc * 0.45; // corps suspendu en l'air à +57 cm
                hips_rot = Quat::from_rotation_x(-arc * 0.08);
                thorax_rot = Quat::IDENTITY;

                l_thigh_rot = Quat::from_rotation_x(-arc * 0.35); // jambes légèrement repliées
                r_thigh_rot = Quat::from_rotation_x(-arc * 0.35);

                l_calf_rot = Quat::from_rotation_x(arc * 0.55);   // genoux fléchis 31°
                r_calf_rot = Quat::from_rotation_x(arc * 0.55);

                l_foot_rot = Quat::from_rotation_x(0.15);
                r_foot_rot = Quat::from_rotation_x(0.15);

                let arm_float = 1.45 * (1.0 - p) + 0.30 * p;
                l_upperarm_rot = Quat::from_rotation_x(arm_float);
                r_upperarm_rot = Quat::from_rotation_x(arm_float);
                l_forearm_rot = Quat::from_rotation_x(0.40);
                r_forearm_rot = Quat::from_rotation_x(0.40);

                current_g = 0.05; // apesanteur en vol libre
                vert_compression = 0.0;
            } else if phase_t < 1.70 {
                // Phase 4 : Réception & Amorti d'impact (knees bend to 72° to absorb shock)
                let p = (phase_t - 1.35) / 0.35;
                let decel = (p * PI).sin() * (1.0 - 0.25 * p);

                hips_offset.y = -decel * 0.22;
                hips_rot = Quat::from_rotation_x(decel * 0.18);
                thorax_rot = Quat::from_rotation_x(decel * 0.25);

                l_thigh_rot = Quat::from_rotation_x(-decel * 0.60); // flexion hanches
                r_thigh_rot = Quat::from_rotation_x(-decel * 0.60);

                l_calf_rot = Quat::from_rotation_x(decel * 1.25);   // flexion amortissante genoux +72°
                r_calf_rot = Quat::from_rotation_x(decel * 1.25);

                l_foot_rot = Quat::from_rotation_x(-decel * 0.25);
                r_foot_rot = Quat::from_rotation_x(-decel * 0.25);

                l_upperarm_rot = Quat::from_rotation_x(-decel * 0.35);
                r_upperarm_rot = Quat::from_rotation_x(-decel * 0.35);
                l_forearm_rot = Quat::from_rotation_x(0.50);
                r_forearm_rot = Quat::from_rotation_x(0.50);

                let impact_pulse = (p * FRAC_PI_2).sin() * (1.0 - p);
                current_g = 1.0 + impact_pulse * 1.75; // pic d'impact 2,75 G
                vert_compression = impact_pulse * 0.065;
            } else {
                // Phase 5 : Rétablissement postural élastique
                let p = (phase_t - 1.70) / 0.30;
                let s = 1.0 - p;

                hips_offset.y = -s * 0.06;
                l_calf_rot = Quat::from_rotation_x(s * 0.20);
                r_calf_rot = Quat::from_rotation_x(s * 0.20);
                current_g = 1.0;
                vert_compression = 0.0;
            }
        }
        ActivityType::Squat => {
            // Flexion profonde contrôlée 90° (cycle 2,4 s)
            let cycle = 2.4_f32;
            let theta = (jt % cycle) / cycle * TAU;
            let c = (1.0 - theta.cos()) * 0.5; // 0 -> 1 -> 0

            hips_offset.y = -c * 0.38;
            hips_offset.z = -c * 0.12; // fessiers repoussés en arrière

            hips_rot = Quat::from_rotation_x(c * 0.20);
            thorax_rot = Quat::from_rotation_x(c * 0.25); // tronc droit incliné pour équilibre

            // Cuisses horizontales à 90°
            l_thigh_rot = Quat::from_rotation_x(-c * 1.35);
            r_thigh_rot = Quat::from_rotation_x(-c * 1.35);

            // Genoux fléchis à angle droit
            l_calf_rot = Quat::from_rotation_x(c * 1.55);
            r_calf_rot = Quat::from_rotation_x(c * 1.55);

            l_foot_rot = Quat::from_rotation_x(-c * 0.35);
            r_foot_rot = Quat::from_rotation_x(-c * 0.35);

            // Bras levés à l'horizontale pour contrepoids
            l_upperarm_rot = Quat::from_rotation_x(c * 1.40);
            r_upperarm_rot = Quat::from_rotation_x(c * 1.40);
            l_forearm_rot = Quat::from_rotation_x(0.20);
            r_forearm_rot = Quat::from_rotation_x(0.20);

            current_g = 1.0 + c * 2.5; // charge lombaire quadruplée en bas
            vert_compression = c * 0.045;
        }
    }

    act.impact_g = current_g;

    // 1. Modèle racine (Homme / Femme) : translation globale
    for (mut tf, male, female) in &mut set.p0() {
        let is_active = (*gender == Gender::Male && male.is_some())
            || (*gender == Gender::Female && female.is_some());
        if is_active {
            // Translation globale
            tf.translation = Vec3::new(hips_offset.x, 0.0, hips_offset.z);
        }
    }

    // 2. Application aux 15 articulations biomécaniques réelles
    for (joint, base_tf, mut tf) in &mut set.p1() {
        match joint {
            BodyJoint::Hips => {
                tf.translation = base_tf.0.translation + Vec3::new(0.0, hips_offset.y, 0.0);
                tf.rotation = base_tf.0.rotation * hips_rot;
            }
            BodyJoint::Spine => {
                tf.rotation = base_tf.0.rotation * spine_rot;
            }
            BodyJoint::Thorax => {
                tf.rotation = base_tf.0.rotation * thorax_rot;
            }
            BodyJoint::Neck => {
                tf.rotation = base_tf.0.rotation * neck_rot;
            }
            BodyJoint::Head => {
                tf.rotation = base_tf.0.rotation * head_rot;
            }
            BodyJoint::LeftThigh => {
                tf.rotation = base_tf.0.rotation * l_thigh_rot;
            }
            BodyJoint::LeftCalf => {
                tf.rotation = base_tf.0.rotation * l_calf_rot;
            }
            BodyJoint::LeftFoot => {
                tf.rotation = base_tf.0.rotation * l_foot_rot;
            }
            BodyJoint::RightThigh => {
                tf.rotation = base_tf.0.rotation * r_thigh_rot;
            }
            BodyJoint::RightCalf => {
                tf.rotation = base_tf.0.rotation * r_calf_rot;
            }
            BodyJoint::RightFoot => {
                tf.rotation = base_tf.0.rotation * r_foot_rot;
            }
            BodyJoint::LeftUpperArm => {
                tf.rotation = base_tf.0.rotation * l_upperarm_rot;
            }
            BodyJoint::LeftForearm => {
                tf.rotation = base_tf.0.rotation * l_forearm_rot;
            }
            BodyJoint::LeftHand => {
                tf.rotation = base_tf.0.rotation * l_hand_rot;
            }
            BodyJoint::RightUpperArm => {
                tf.rotation = base_tf.0.rotation * r_upperarm_rot;
            }
            BodyJoint::RightForearm => {
                tf.rotation = base_tf.0.rotation * r_forearm_rot;
            }
            BodyJoint::RightHand => {
                tf.rotation = base_tf.0.rotation * r_hand_rot;
            }
        }
    }

    // 3. Colonne vertébrale (Vertebrae) : micro-compression élastique d'impact
    for (base, mut tf) in &mut set.p2() {
        let comp = vert_compression.clamp(0.0, 0.12);
        tf.scale = Vec3::new(
            base.0.x * (1.0 + comp * 0.35),
            base.0.y * (1.0 - comp),
            base.0.z * (1.0 + comp * 0.35),
        );
    }

    // 4. Poumons & Viscères (Viscera) : ventilation pulmonaire en direct (rythme d'effort)
    let resp_hz = act.breath_rate_rpm / 60.0;
    let breath_wave = (t * resp_hz * TAU).sin();
    let effort_factor = 1.0 + act.current_effort as f32 * 0.8;
    let breath_expand = breath_wave * 0.045 * effort_factor;
    for (base, mut tf) in &mut set.p3() {
        tf.scale = Vec3::new(
            base.0.x * (1.0 + breath_expand * 0.8),
            base.0.y * (1.0 + breath_expand * 0.4),
            base.0.z * (1.0 + breath_expand),
        );
    }

    // 5. Muscles : contraction et tonus musculaire d'effort
    let muscle_pump = 1.0 + (act.current_effort as f32) * 0.05;
    for (base, mut tf) in &mut set.p4() {
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
