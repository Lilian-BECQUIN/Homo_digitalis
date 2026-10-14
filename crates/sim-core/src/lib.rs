//! `sim-core` : noyau physiologique déterministe de Homo Digitalis.
//!
//! Aucune dépendance graphique : le noyau peut tourner dans un thread dédié à 1000 Hz
//! et être testé unitairement sans GPU.
//!
//! > Simulateur théorique à but éducatif et scientifique — pas un dispositif médical.

pub mod gas;
pub mod hemodynamics;
pub mod inputs;
pub mod params;
pub mod safety;
pub mod simulation;
pub mod solver;
pub mod telemetry;
pub mod vessel;

pub use inputs::Inputs;
pub use simulation::{recommended_dt, Simulation};
pub use telemetry::{Status, Telemetry, WaveSample};

/// Avertissement d'usage à afficher au lancement.
pub const DISCLAIMER: &str = "Homo Digitalis est un simulateur théorique à but éducatif et \
scientifique. Il ne constitue en aucun cas un dispositif de diagnostic médical et ne doit \
jamais être utilisé pour prendre une décision de santé.";
