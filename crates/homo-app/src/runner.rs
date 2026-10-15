//! Thread de simulation découplé du rendu : boucle à pas fixe (1 kHz de temps simulé)
//! en natif, ou système dans la boucle de frame sur WebAssembly (pas de threads OS).
//! Commandes par canal/tampon, télémétrie publiée dans un état partagé.

use std::collections::VecDeque;

#[cfg(not(target_arch = "wasm32"))]
use std::sync::mpsc::{self, Receiver, Sender, TryRecvError};
#[cfg(not(target_arch = "wasm32"))]
use std::sync::{Arc, Mutex};
#[cfg(not(target_arch = "wasm32"))]
use std::thread;
#[cfg(not(target_arch = "wasm32"))]
use std::time::{Duration, Instant};

#[cfg(target_arch = "wasm32")]
use std::sync::Mutex;

use bevy::prelude::*;
use sim_core::{recommended_dt, Inputs, Simulation, Telemetry, WaveSample};

/// Budget maximal de pas par itération du thread natif (évite de bloquer en cas de retard).
#[cfg(not(target_arch = "wasm32"))]
const MAX_STEPS_PER_TICK: u32 = 20_000;

/// Budget maximal de pas par frame sur Web (pour garantir 60 FPS).
#[cfg(target_arch = "wasm32")]
const MAX_STEPS_PER_FRAME_WEB: u32 = 600;

const WAVE_CAPACITY: usize = 1200;

/// Commandes envoyées de l'UI vers le simulateur.
#[derive(Debug, Clone)]
pub enum Command {
    SetInputs(Inputs),
    InjectAdrenaline(f64),
    SetTimeScale(f64),
    SetPaused(bool),
    Rewind(f64),
    Reset,
}

#[cfg(not(target_arch = "wasm32"))]
struct Shared {
    telemetry: Telemetry,
    new_samples: Vec<WaveSample>,
}

#[derive(Resource)]
pub struct SimHandle {
    #[cfg(not(target_arch = "wasm32"))]
    tx: Sender<Command>,
    #[cfg(not(target_arch = "wasm32"))]
    shared: Arc<Mutex<Shared>>,

    #[cfg(target_arch = "wasm32")]
    commands: Mutex<Vec<Command>>,
}

impl SimHandle {
    #[cfg(not(target_arch = "wasm32"))]
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

    #[cfg(target_arch = "wasm32")]
    pub fn new_wasm() -> Self {
        Self {
            commands: Mutex::new(Vec::new()),
        }
    }

    pub fn send(&self, command: Command) {
        #[cfg(not(target_arch = "wasm32"))]
        {
            // Si le thread est terminé, l'application se ferme : on ignore l'erreur.
            let _ = self.tx.send(command);
        }
        #[cfg(target_arch = "wasm32")]
        {
            if let Ok(mut cmds) = self.commands.lock() {
                cmds.push(command);
            }
        }
    }
}

#[cfg(not(target_arch = "wasm32"))]
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

#[cfg(not(target_arch = "wasm32"))]
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

#[cfg(target_arch = "wasm32")]
#[derive(Resource)]
struct WasmSimState {
    sim: Simulation,
    scale: f64,
    paused: bool,
}

#[cfg(target_arch = "wasm32")]
fn wasm_sim_tick(
    handle: Res<SimHandle>,
    mut state: ResMut<WasmSimState>,
    mut live: ResMut<LiveTelemetry>,
    time: Res<Time>,
) {
    if let Ok(mut cmds) = handle.commands.lock() {
        for cmd in cmds.drain(..) {
            match cmd {
                Command::SetInputs(i) => state.sim.set_inputs(i),
                Command::InjectAdrenaline(u) => state.sim.inject_adrenaline(u),
                Command::SetTimeScale(s) => {
                    state.scale = if s.is_finite() { s.clamp(0.01, 3600.0) } else { 1.0 }
                }
                Command::SetPaused(p) => state.paused = p,
                Command::Rewind(seconds) => {
                    state.sim.rewind(seconds);
                }
                Command::Reset => state.sim = Simulation::new(),
            }
        }
    }

    let real_dt = (time.delta_secs_f64()).min(0.1);
    if !state.paused {
        let scale = state.scale;
        let dt = recommended_dt(scale);
        state.sim.set_sample_interval((0.005 * scale).max(dt));
        state.sim.advance(real_dt * scale, dt, MAX_STEPS_PER_FRAME_WEB);
    }

    live.current = state.sim.telemetry();
    let samples = state.sim.drain_samples();
    for s in samples {
        live.wave.push_back(s);
    }
    while live.wave.len() > WAVE_CAPACITY {
        live.wave.pop_front();
    }
}

pub struct RunnerPlugin;

impl Plugin for RunnerPlugin {
    fn build(&self, app: &mut App) {
        #[cfg(not(target_arch = "wasm32"))]
        app.insert_resource(SimHandle::spawn())
            .init_resource::<LiveTelemetry>()
            .add_systems(PreUpdate, pull_telemetry);

        #[cfg(target_arch = "wasm32")]
        app.insert_resource(SimHandle::new_wasm())
            .insert_resource(WasmSimState {
                sim: Simulation::new(),
                scale: 1.0,
                paused: false,
            })
            .init_resource::<LiveTelemetry>()
            .add_systems(PreUpdate, wasm_sim_tick);
    }
}
