//! Liaison simulation → shaders/animations : le cœur bat avec le volume ventriculaire simulé,
//! les artères pulsent avec la pression, la couleur du sang suit la saturation en O₂.

use bevy::prelude::*;

use crate::anatomy::{BaseScale, HeartMarker, Oxygenated, Pulsatile};
use crate::layers::BaseColor;
use crate::runner::LiveTelemetry;

fn animate_heart(live: Res<LiveTelemetry>, mut query: Query<(&BaseScale, &mut Transform), With<HeartMarker>>) {
    let t = &live.current;
    // Contraction géométrique (équivalent d'un morph target) pilotée par le volume du VG.
    let fill = ((t.lv_volume_ml - 90.0) / 60.0).clamp(-1.0, 1.0);
    let k = 1.0 + 0.12 * fill;
    for (base, mut tf) in &mut query {
        tf.scale = base.0 * k;
    }
}

fn pulse_vessels(live: Res<LiveTelemetry>, mut query: Query<(&BaseScale, &mut Transform), With<Pulsatile>>) {
    let t = &live.current;
    let pulse = ((t.arterial_pressure_mmhg - 80.0) / 80.0).clamp(-0.5, 1.0);
    let k = 1.0 + 0.18 * pulse;
    for (base, mut tf) in &mut query {
        tf.scale = Vec3::new(base.0.x * k, base.0.y, base.0.z * k);
    }
}

/// HbO₂ (rouge vif) ↔ Hb désoxygénée (rouge sombre violacé).
fn oxygenation_color(spo2_percent: f32) -> Color {
    let s = ((spo2_percent - 40.0) / 60.0).clamp(0.0, 1.0);
    let lerp = |a: f32, b: f32| a + (b - a) * s;
    let q = |x: f32| (x * 255.0).round() / 255.0;
    Color::srgb(q(lerp(0.30, 0.90)), q(lerp(0.02, 0.10)), q(lerp(0.12, 0.08)))
}

fn tint_blood(live: Res<LiveTelemetry>, mut query: Query<&mut BaseColor, With<Oxygenated>>) {
    let color = oxygenation_color(live.current.spo2_percent);
    for mut base in &mut query {
        if base.0 != color {
            base.0 = color;
        }
    }
}

pub struct VisualsPlugin;

impl Plugin for VisualsPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, (animate_heart, pulse_vessels, tint_blood));
    }
}
