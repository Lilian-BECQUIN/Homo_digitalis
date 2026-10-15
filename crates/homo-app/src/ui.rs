//! HUD « bac à sable » (egui) : avertissement d'usage, environnement, calques, interventions,
//! contrôleur temporel et inspecteur télémétrique avec oscilloscopes.

use bevy::prelude::*;
use bevy_egui::{egui, EguiContexts};
use sim_core::{gas, Inputs, Status};

use crate::activity::{ActivityState, ActivityType};
use crate::anatomy::{OrganTag, Selection};
use crate::layers::{AnatomyLayer, Gender, LayerSettings};
use crate::runner::{Command, LiveTelemetry, SimHandle};

const TIME_SCALES: [(&str, f64); 5] = [
    ("0,01×", 0.01),
    ("1×", 1.0),
    ("60×", 60.0),
    ("600×", 600.0),
    ("3600×", 3600.0),
];
const SCOPE_SAMPLES: usize = 600;

#[derive(Resource)]
pub struct UiState {
    disclaimer_accepted: bool,
    inputs: Inputs,
    last_sent: Inputs,
    time_scale_idx: usize,
    paused: bool,
    adrenaline_dose: f64,
    // Paramètres d'environnement des phases ultérieures (affichés, non encore actifs).
    ambient_temp_c: f64,
    humidity_pct: f64,
    gravity_g: f64,
}

impl Default for UiState {
    fn default() -> Self {
        Self {
            disclaimer_accepted: false,
            inputs: Inputs::default(),
            last_sent: Inputs::default(),
            time_scale_idx: 1,
            paused: true,
            adrenaline_dose: 1.0,
            ambient_temp_c: 20.0,
            humidity_pct: 50.0,
            gravity_g: 1.0,
        }
    }
}

fn pause_until_disclaimer_accepted(handle: Res<SimHandle>) {
    handle.send(Command::SetPaused(true));
}

fn kv(ui: &mut egui::Ui, key: &str, value: String) {
    ui.label(key);
    ui.monospace(value);
    ui.end_row();
}

fn scope(ui: &mut egui::Ui, title: &str, values: &[f32], lo: f32, hi: f32, color: egui::Color32) {
    ui.label(title);
    let (resp, painter) =
        ui.allocate_painter(egui::vec2(ui.available_width(), 84.0), egui::Sense::hover());
    let rect = resp.rect;
    painter.rect_filled(rect, 4.0, egui::Color32::from_rgb(6, 12, 9));
    if values.len() < 2 {
        return;
    }
    let n = values.len();
    let span = (hi - lo).max(1e-6);
    let points: Vec<egui::Pos2> = values
        .iter()
        .enumerate()
        .map(|(i, &v)| {
            let x = rect.left() + rect.width() * i as f32 / (n - 1) as f32;
            let t = ((v - lo) / span).clamp(0.0, 1.0);
            egui::pos2(x, rect.bottom() - 2.0 - t * (rect.height() - 4.0))
        })
        .collect();
    painter.add(egui::Shape::line(points, egui::Stroke::new(1.5_f32, color)));
}

fn clock(seconds: f64) -> String {
    let s = seconds as u64;
    format!("{:02}:{:02}:{:02}", s / 3600, (s / 60) % 60, s % 60)
}

#[allow(clippy::too_many_arguments)]
fn hud(
    mut contexts: EguiContexts,
    mut state: ResMut<UiState>,
    mut layers: ResMut<LayerSettings>,
    mut gender: ResMut<Gender>,
    mut activity: ResMut<ActivityState>,
    live: Res<LiveTelemetry>,
    selection: Res<Selection>,
    handle: Res<SimHandle>,
) {
    let ctx = contexts.ctx_mut().clone();
    let s = &mut *state;

    if !s.disclaimer_accepted {
        egui::Window::new("⚠ Avertissement d'usage")
            .collapsible(false)
            .resizable(false)
            .anchor(egui::Align2::CENTER_CENTER, egui::vec2(0.0, 0.0))
            .show(&ctx, |ui| {
                ui.set_max_width(460.0);
                ui.label(sim_core::DISCLAIMER);
                ui.add_space(8.0);
                if ui.button("J'ai compris — lancer la simulation").clicked() {
                    s.disclaimer_accepted = true;
                    s.paused = false;
                    handle.send(Command::SetPaused(false));
                }
            });
        return;
    }

    let t = live.current;

    // ------------------------------------------------------------ Bas : interventions + temps
    egui::TopBottomPanel::bottom("interventions").show(&ctx, |ui| {
        ui.horizontal_wrapped(|ui| {
            ui.strong("Interventions");
            ui.add(egui::Slider::new(&mut s.inputs.hemorrhage_rate_ml_s, 0.0..=100.0).text("Hémorragie").suffix(" mL/s"));
            ui.add(egui::Slider::new(&mut s.inputs.vasodilation_factor, 0.1..=2.0).text("Résistance vasc. (choc anaphylactique <1)"));
            ui.add(egui::Slider::new(&mut s.inputs.viscosity_factor, 0.5..=2.0).text("Viscosité sanguine η/η₀"));
        });
        ui.horizontal_wrapped(|ui| {
            ui.add(egui::Slider::new(&mut s.adrenaline_dose, 0.1..=10.0).text("Dose adrénaline"));
            if ui.button("💉 Injecter adrénaline").clicked() {
                handle.send(Command::InjectAdrenaline(s.adrenaline_dose));
            }
            ui.separator();
            ui.strong("Temps");
            let before = s.time_scale_idx;
            for (i, (label, _)) in TIME_SCALES.iter().enumerate() {
                ui.selectable_value(&mut s.time_scale_idx, i, *label);
            }
            if s.time_scale_idx != before {
                handle.send(Command::SetTimeScale(TIME_SCALES[s.time_scale_idx].1));
            }
            if ui.button(if s.paused { "▶ Reprendre" } else { "⏸ Pause" }).clicked() {
                s.paused = !s.paused;
                handle.send(Command::SetPaused(s.paused));
            }
            if ui.button("⏪ Rewind 30 s").clicked() {
                handle.send(Command::Rewind(30.0));
            }
            if ui.button("↺ Réinitialiser").clicked() {
                s.inputs = Inputs::default();
                handle.send(Command::Reset);
                handle.send(Command::SetInputs(s.inputs));
                s.last_sent = s.inputs;
            }
            ui.monospace(format!("t = {}", clock(t.time_s)));
        });
    });

    // ------------------------------------------------------------ Gauche : activités + sujet + isolation + calques
    egui::SidePanel::left("environment").default_width(320.0).show(&ctx, |ui| {
        ui.heading("Activités & Biomécanique");
        ui.horizontal_wrapped(|ui| {
            for act in ActivityType::ALL {
                let is_cur = activity.current == act;
                if ui.selectable_label(is_cur, act.label()).clicked() {
                    activity.set_activity(act);
                    s.inputs.effort = activity.target_effort;
                }
            }
        });
        ui.small(activity.current.description());
        ui.separator();

        ui.heading("Sujet Anatomique");
        ui.horizontal(|ui| {
            ui.selectable_value(&mut *gender, Gender::Male, "👨 Homme (Adulte)");
            ui.selectable_value(&mut *gender, Gender::Female, "👩 Femme (Adulte)");
        });
        ui.separator();

        ui.heading("Filtres & Isolation Rapide (Solo)");
        let mut layers_changed = false;
        ui.horizontal_wrapped(|ui| {
            if ui.button("🧬 Écorché complet").on_hover_text("Enveloppe cutanée translucide laissant voir les muscles, organes et squelette").clicked() {
                layers.preset_all();
                layers_changed = true;
            }
            if ui.button("🧍 Peau opaque").on_hover_text("Corps complet extérieur").clicked() {
                layers.preset_skin_only();
                layers_changed = true;
            }
            if ui.button("🦴 Os & Squelette").on_hover_text("Squelette complet et boîte crânienne").clicked() {
                layers.preset_skeleton_only();
                layers_changed = true;
            }
            if ui.button("🏛 Vertèbres seules").on_hover_text("Rachis complet (cervical, thoracique, lombaire, sacrum, coccyx)").clicked() {
                layers.preset_vertebrae_only();
                layers_changed = true;
            }
            if ui.button("💪 Muscles seuls").on_hover_text("Système musculaire squelettique").clicked() {
                layers.preset_muscles_only();
                layers_changed = true;
            }
            if ui.button("🩸 Vasculaire complet").on_hover_text("Cœur + toutes les artères et veines").clicked() {
                layers.preset_vascular_only();
                layers_changed = true;
            }
            if ui.button("💙 Veines seules").on_hover_text("Réseau veineux complet de retour").clicked() {
                layers.preset_veins_only();
                layers_changed = true;
            }
            if ui.button("⚡ Système nerveux").on_hover_text("Encéphale, moelle épinière et troncs nerveux").clicked() {
                layers.preset_nervous_only();
                layers_changed = true;
            }
            if ui.button("🫁 Organes internes").on_hover_text("Poumons, cœur, foie, estomac, intestins, reins").clicked() {
                layers.preset_viscera_only();
                layers_changed = true;
            }
        });
        ui.separator();

        ui.heading("Calques Anatomiques (Opacité)");
        {
            let opacity = &mut layers.bypass_change_detection().opacity;
            for layer in AnatomyLayer::ALL {
                layers_changed |= ui
                    .add(egui::Slider::new(&mut opacity[layer.index()], 0.0..=1.0).text(layer.label()))
                    .changed();
            }
        }
        if layers_changed {
            layers.set_changed();
        }
        ui.separator();

        ui.heading("Environnement & Effort");
        ui.add(egui::Slider::new(&mut s.inputs.altitude_m, 0.0..=Inputs::MAX_ALTITUDE_M).text("Altitude").suffix(" m"));
        let g = gas::compute(s.inputs.altitude_m);
        ui.small(format!("PiO₂ = {:.0} mmHg · Patm = {:.0} mmHg", g.inspired_o2_mmhg, g.barometric_mmhg));
        let effort_slider = ui.add(egui::Slider::new(&mut s.inputs.effort, 0.0..=1.0).text("Effort physique"));
        if effort_slider.changed() {
            activity.target_effort = s.inputs.effort;
        } else {
            s.inputs.effort = activity.current_effort;
        }
        ui.add_space(6.0);
        ui.small("Souris : Clic gauche = sélection · Clic droit = orbite · Clic milieu = pan · Molette = zoom");
    });

    // ------------------------------------------------------------ Droite : inspecteur
    egui::SidePanel::right("inspector").default_width(340.0).show(&ctx, |ui| {
        ui.heading("Inspecteur télémétrique");
        match &selection.name {
            Some(name) => {
                ui.strong(name);
                if let Some(layer) = selection.layer {
                    ui.small(layer.label());
                }
                if !selection.info.is_empty() {
                    ui.label(selection.info);
                }
            }
            None => {
                ui.label("Cliquez sur un organe, un os ou un vaisseau.");
            }
        }
        ui.separator();

        let tail = |f: &dyn Fn(&sim_core::WaveSample) -> f32| -> Vec<f32> {
            let skip = live.wave.len().saturating_sub(SCOPE_SAMPLES);
            live.wave.iter().skip(skip).map(f).collect()
        };
        match selection.tag {
            Some(OrganTag::Heart) => {
                scope(ui, "ECG (dérivation II)", &tail(&|w| w.ecg_mv), -0.4, 1.1, egui::Color32::from_rgb(80, 255, 120));
                scope(ui, "Pression ventriculaire gauche (mmHg)", &tail(&|w| w.lv_mmhg), 0.0, 180.0, egui::Color32::from_rgb(255, 170, 60));
            }
            Some(OrganTag::Artery) => {
                scope(ui, "Pression artérielle (mmHg)", &tail(&|w| w.arterial_mmhg), 30.0, 180.0, egui::Color32::from_rgb(255, 90, 90));
                scope(ui, "Débit aortique (mL/s)", &tail(&|w| w.aortic_flow_ml_s), 0.0, 600.0, egui::Color32::from_rgb(90, 190, 255));
            }
            Some(OrganTag::Vein) => {
                ui.label(format!("Pression veineuse centrale : {:.1} mmHg", t.venous_pressure_mmhg));
                scope(ui, "Pression artérielle (référence)", &tail(&|w| w.arterial_mmhg), 30.0, 180.0, egui::Color32::from_rgb(255, 90, 90));
            }
            _ => {
                scope(ui, "ECG (référence)", &tail(&|w| w.ecg_mv), -0.4, 1.1, egui::Color32::from_rgb(80, 255, 120));
                ui.small("Concentrations locales et métabolisme : phase 3.");
            }
        }

        ui.separator();
        ui.heading("Constantes vitales");
        match t.status {
            Status::Alive => {}
            Status::CardiacArrest => {
                ui.colored_label(egui::Color32::LIGHT_RED, "ARRÊT CIRCULATOIRE (état d'arrêt biologique du modèle)");
            }
            Status::NumericalFault => {
                ui.colored_label(egui::Color32::YELLOW, "Divergence numérique rattrapée : modèle figé");
            }
        }
        egui::Grid::new("vitals").num_columns(2).striped(true).show(ui, |ui| {
            kv(ui, "Fréquence cardiaque", format!("{:.0} bpm", t.heart_rate_bpm));
            kv(ui, "Pression artérielle", format!("{:.0}/{:.0} mmHg", t.systolic_mmhg, t.diastolic_mmhg));
            kv(ui, "PAM (filtrée)", format!("{:.0} mmHg", t.mean_arterial_pressure_mmhg));
            kv(ui, "Pression veineuse", format!("{:.1} mmHg", t.venous_pressure_mmhg));
            kv(ui, "Volume d'éjection", format!("{:.0} mL", t.stroke_volume_ml));
            kv(ui, "Débit cardiaque", format!("{:.1} L/min", t.cardiac_output_l_min));
            kv(ui, "Résistance périph.", format!("{:.2} mmHg·s/mL", t.peripheral_resistance));
            kv(ui, "SpO₂", format!("{:.0} %", t.spo2_percent));
            kv(ui, "PaO₂ / PaCO₂", format!("{:.0} / {:.0} mmHg", t.pao2_mmhg, t.paco2_mmhg));
            kv(ui, "Sympathique", format!("{:.2}", t.sympathetic));
            kv(ui, "Parasympathique", format!("{:.2}", t.parasympathetic));
            kv(ui, "Volume sanguin", format!("{:.0} mL (−{:.0})", t.blood_volume_ml, t.blood_lost_ml));
        });

        ui.separator();
        ui.heading("Activité, Biomécanique & Métabolisme");
        egui::Grid::new("activity_metrics").num_columns(2).striped(true).show(ui, |ui| {
            kv(ui, "Mouvement actuel", activity.current.label().to_string());
            kv(ui, "Vitesse de déplacement", format!("{:.1} km/h", activity.speed_kmh));
            kv(ui, "Cadence locomotrice", format!("{:.0} pas/min", activity.cadence_spm));
            kv(ui, "Force d'impact sol", format!("{:.2} G", activity.impact_g));
            kv(ui, "Puissance mécanique", format!("{:.0} W", activity.power_watts));
            kv(ui, "Dépense énergétique", format!("{:.0} kcal/h", activity.calories_kcal_h));
            kv(ui, "Consommation VO₂", format!("{:.1} mL/kg/min", activity.vo2_ml_kg_min));
            kv(ui, "Équivalent métabolique", format!("{:.1} METs", activity.mets));
            kv(ui, "Fréquence respiratoire", format!("{:.0} /min", activity.breath_rate_rpm));
            kv(ui, "Débit ventilatoire", format!("{:.1} L/min", activity.minute_ventilation_l_min));
        });
    });

    // Synchronisation automatique de l'effort physiologique avec l'activité sélectionnée
    s.inputs.effort = activity.current_effort;

    // Envoi des entrées uniquement si elles ont changé.
    if s.inputs != s.last_sent {
        handle.send(Command::SetInputs(s.inputs));
        s.last_sent = s.inputs;
    }
}

pub struct UiPlugin;

impl Plugin for UiPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<UiState>()
            .add_systems(Startup, pause_until_disclaimer_accepted)
            .add_systems(Update, hud);
    }
}
