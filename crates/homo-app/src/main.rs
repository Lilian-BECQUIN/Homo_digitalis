//! Homo Digitalis — point d'entrée. La simulation tourne dans son propre thread
//! (voir `runner`) ; Bevy ne fait que lire la télémétrie et envoyer des commandes.

mod activity;
mod anatomy;
pub mod anatomy_catalog;
mod camera;
mod layers;
mod runner;
mod ui;
mod visuals;

use bevy::picking::mesh_picking::MeshPickingPlugin;
use bevy::prelude::*;
use bevy_egui::EguiPlugin;

fn main() {
    App::new()
        .insert_resource(ClearColor(Color::srgb(0.02, 0.03, 0.05)))
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "Homo Digitalis — simulateur physiologique (éducatif)".into(),
                resolution: (1600.0_f32, 900.0_f32).into(),
                #[cfg(target_arch = "wasm32")]
                fit_canvas_to_parent: true,
                #[cfg(target_arch = "wasm32")]
                canvas: Some("#bevy_canvas".into()),
                ..default()
            }),
            ..default()
        }))
        .add_plugins(MeshPickingPlugin)
        .add_plugins(EguiPlugin)
        .add_plugins((
            runner::RunnerPlugin,
            layers::LayersPlugin,
            camera::OrbitCameraPlugin,
            anatomy::AnatomyPlugin,
            activity::ActivityPlugin,
            visuals::VisualsPlugin,
            ui::UiPlugin,
        ))
        .run();
}
