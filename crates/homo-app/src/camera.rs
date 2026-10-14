//! Caméra orbitale fluide : clic droit = orbite, clic milieu = panoramique, molette = zoom.

use bevy::input::mouse::{AccumulatedMouseMotion, AccumulatedMouseScroll, MouseScrollUnit};
use bevy::prelude::*;
use bevy_egui::EguiContexts;

#[derive(Component)]
pub struct OrbitCamera {
    target: Vec3,
    yaw: f32,
    pitch: f32,
    distance: f32,
    // Valeurs lissées effectivement appliquées.
    s_target: Vec3,
    s_yaw: f32,
    s_pitch: f32,
    s_distance: f32,
}

impl OrbitCamera {
    fn new(target: Vec3, yaw: f32, pitch: f32, distance: f32) -> Self {
        Self {
            target,
            yaw,
            pitch,
            distance,
            s_target: target,
            s_yaw: yaw,
            s_pitch: pitch,
            s_distance: distance,
        }
    }
}

fn spawn_camera(mut commands: Commands) {
    commands.spawn((
        Camera3d::default(),
        Transform::default(),
        OrbitCamera::new(Vec3::new(0.0, 1.1, 0.0), 0.3, 0.1, 3.2),
    ));
    commands.spawn((
        DirectionalLight {
            illuminance: 12_000.0,
            shadows_enabled: false,
            ..default()
        },
        Transform::from_xyz(2.0, 4.0, 3.0).looking_at(Vec3::new(0.0, 1.0, 0.0), Vec3::Y),
    ));
    commands.insert_resource(AmbientLight {
        color: Color::srgb(0.75, 0.82, 1.0),
        brightness: 350.0,
    });
}

fn orbit_camera(
    mut contexts: EguiContexts,
    buttons: Res<ButtonInput<MouseButton>>,
    motion: Res<AccumulatedMouseMotion>,
    scroll: Res<AccumulatedMouseScroll>,
    time: Res<Time>,
    mut query: Query<(&mut OrbitCamera, &mut Transform)>,
) {
    let over_ui = contexts.ctx_mut().is_pointer_over_area();
    let Ok((mut cam, mut tf)) = query.get_single_mut() else {
        return;
    };

    if !over_ui {
        if buttons.pressed(MouseButton::Right) {
            cam.yaw -= motion.delta.x * 0.006;
            cam.pitch = (cam.pitch + motion.delta.y * 0.006).clamp(-1.4, 1.4);
        }
        if buttons.pressed(MouseButton::Middle) {
            let right = tf.right();
            let up = tf.up();
            let k = cam.distance * 0.0014;
            let delta = (-right * motion.delta.x + up * motion.delta.y) * k;
            cam.target += delta;
        }
        let scroll_amount = match scroll.unit {
            MouseScrollUnit::Line => scroll.delta.y,
            MouseScrollUnit::Pixel => scroll.delta.y * 0.02,
        };
        if scroll_amount != 0.0 {
            cam.distance = (cam.distance * (1.0 - scroll_amount * 0.1)).clamp(0.3, 12.0);
        }
    }

    // Lissage exponentiel indépendant du framerate.
    let k = 1.0 - (-14.0 * time.delta_secs()).exp();
    cam.s_yaw += (cam.yaw - cam.s_yaw) * k;
    cam.s_pitch += (cam.pitch - cam.s_pitch) * k;
    cam.s_distance += (cam.distance - cam.s_distance) * k;
    let target = cam.target;
    cam.s_target = cam.s_target.lerp(target, k);

    let rotation = Quat::from_euler(EulerRot::YXZ, cam.s_yaw, -cam.s_pitch, 0.0);
    tf.translation = cam.s_target + rotation * Vec3::new(0.0, 0.0, cam.s_distance);
    tf.look_at(cam.s_target, Vec3::Y);
}

pub struct OrbitCameraPlugin;

impl Plugin for OrbitCameraPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, spawn_camera)
            .add_systems(Update, orbit_camera);
    }
}
