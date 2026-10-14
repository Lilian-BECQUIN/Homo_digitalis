//! Thread de simulation découplé du rendu : boucle à pas fixe (1 kHz de temps simulé),
//! commandes par canal, télémétrie publiée dans un état partagé.

use std::collections::VecDeque;
use std::sync::mpsc::{self, Receiver, Sender, TryRecvError};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

use bevy::prelude::*;
use sim_core::{recommended_dt, Inputs, Simulation, Telemetry, WaveSample};

/// Budget maximal de pas par itération du thread (évite de bloquer en cas de retard).
const MAX_STEPS_PER_TICK: u32 = 20_000;
const WAVE_CAPACITY: usize = 1200;

/// Commandes envoyées de l'UI vers le thread de simulation.
#[derive(Debug, Clone)]
pub enum Command {
    SetInputs(Inputs),
    InjectAdrenaline(f64),
    SetTimeScale(f64),
    SetPaused(bool),
    Rewind(f64),
    Reset,
}

struct Shared {
    telemetry: Telemetry,
    new_samples: Vec<WaveSample>,
}

#[derive(Resource)]
pub struct SimHandle {
    tx: Sender<Command>,
    shared: Arc<Mutex<Shared>>,
}

impl SimHandle {
    pub fn spawn() -> Self {
        let (tx, rx) = mpsc::channel();
        let shared = Arc::new(Mutex::new(Shared {
            telemetry: Telemetry::default(),
            new_samples: Vec::new(),
        }));
        let thread_shared = Arc::clone(&shared);
        thread::Builder::new()
            .name("sim-core".into())
            .spawn(move || sim_thread(rx, thread_shared))
            .expect("impossible de démarrer le thread de simulation");
        Self { tx, shared }
    }

    pub fn send(&self, command: Command) {
        // Si le thread est terminé, l'application se ferme : on ignore l'erreur.
        let _ = self.tx.send(command);
    }
}

fn sim_thread(rx: Receiver<Command>, shared: Arc<Mutex<Shared>>) {
    let mut sim = Simulation::new();
    let mut scale = 1.0_f64;
    let mut paused = false;
    let mut last = Instant::now();

    loop {
        loop {
            match rx.try_recv() {
                Ok(Command::SetInputs(i)) => sim.set_inputs(i),
                Ok(Command::InjectAdrenaline(u)) => sim.inject_adrenaline(u),
                Ok(Command::SetTimeScale(s)) => {
                    scale = if s.is_finite() { s.clamp(0.01, 3600.0) } else { 1.0 }
                }
                Ok(Command::SetPaused(p)) => paused = p,
                Ok(Command::Rewind(seconds)) => {
                    sim.rewind(seconds);
                }
                Ok(Command::Reset) => sim = Simulation::new(),
                Err(TryRecvError::Empty) => break,
                Err(TryRecvError::Disconnected) => return,
            }
        }

        let now = Instant::now();
        let real_dt = (now - last).as_secs_f64().min(0.1);
        last = now;

        if !paused {
            let dt = recommended_dt(scale);
            // ~200 échantillons d'oscilloscope par seconde réelle quel que soit le facteur de temps.
            sim.set_sample_interval((0.005 * scale).max(dt));
            sim.advance(real_dt * scale, dt, MAX_STEPS_PER_TICK);
        }

        let telemetry = sim.telemetry();
        let samples = sim.drain_samples();
        {
            let mut g = shared.lock().unwrap_or_else(|e| e.into_inner());
            g.telemetry = telemetry;
            g.new_samples.extend(samples);
            let overflow = g.new_samples.len().saturating_sub(WAVE_CAPACITY);
            if overflow > 0 {
                g.new_samples.drain(..overflow);
            }
        }

        thread::sleep(Duration::from_millis(1));
    }
}

/// Dernière télémétrie lue côté rendu + historique d'ondes pour l'oscilloscope.
#[derive(Resource, Default)]
pub struct LiveTelemetry {
    pub current: Telemetry,
    pub wave: VecDeque<WaveSample>,
}

fn pull_telemetry(handle: Res<SimHandle>, mut live: ResMut<LiveTelemetry>) {
    let mut g = handle.shared.lock().unwrap_or_else(|e| e.into_inner());
    live.current = g.telemetry;
    for s in g.new_samples.drain(..) {
        live.wave.push_back(s);
    }
    while live.wave.len() > WAVE_CAPACITY {
        live.wave.pop_front();
    }
}

pub struct RunnerPlugin;

impl Plugin for RunnerPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(SimHandle::spawn())
            .init_resource::<LiveTelemetry>()
            .add_systems(PreUpdate, pull_telemetry);
    }
}
