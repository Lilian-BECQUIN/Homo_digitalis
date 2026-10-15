# Homo Digitalis

Simulateur multi-physique en temps réel de la physiologie humaine (hémodynamique, puis
biomécanique, métabolisme/PBPK, thermorégulation) avec visualisation 3D interactive.

> ⚠ Simulateur théorique à but éducatif et scientifique. **Pas un dispositif de diagnostic médical.**

## État d'avancement
> **Note** : le code Rust a été écrit sans toolchain installée (jamais compilé). Le modèle
> cardiovasculaire a été validé via un prototype Python équivalent ; le premier `cargo test`
> peut révéler des erreurs de compilation à corriger (surtout côté Bevy/egui).

- ✅ **Phase 1** (socle) : Bevy/wgpu, caméra orbitale, picking, 6 calques à opacité réglable.
  Anatomie provisoire procédurale (maillages Z-Anatomy : `docs/ASSET_PIPELINE.md`).
- ✅ **Phase 2** (cœur) : RK4 + Windkessel 3 éléments en boucle fermée, baroréflexe, effort,
  hémorragie, altitude/SpO₂, anaphylaxie, viscosité, adrénaline, snapshots/rewind, oscilloscopes.
- ⏳ Phases 3 à 5 : voir `docs/ARCHITECTURE.md`.

## Lancement rapide sans configuration (WebAssembly)
Si votre système Windows bloque l'exécution des exécutables `.exe` non signés (Smart App Control / WDAC), utilisez la version WebAssembly dans votre navigateur (Chrome, Edge) :

1. Allez sur les **Actions GitHub** de votre dépôt : <https://github.com/Lilian-BECQUIN/Homo_digitalis/actions>
2. Ouvrez le workflow **Build Web (WASM)** et téléchargez l'artefact **`homo-digitalis-web`** en bas de la page.
3. Décompressez le fichier `.zip` dans un dossier.
4. Lancez simplement dans PowerShell ou cmd :
   ```bash
   python serve_web.py
   ```
   Votre navigateur s'ouvre automatiquement sur `http://localhost:8000` avec l'application 3D interactive et le solveur temps réel !

## Prérequis pour le développement natif
1. Installer Rust (≥ 1.82) : <https://rustup.rs>
2. Windows : *Build Tools for Visual Studio* (charge de travail C++). Linux : `libasound2-dev libudev-dev libwayland-dev libxkbcommon-dev`.

## Commandes (Développement natif)
```bash
cargo test -p sim-core                 # solveur, modèle cardiovasculaire, garde-fous (sans GPU)
cargo run -p homo-app                  # application desktop (1re compilation de Bevy : plusieurs minutes)
cargo run -p homo-app --release
```

## Contrôles
Clic droit + glisser : orbite · clic milieu : pan · molette : zoom · clic gauche : sélection
(calques ≥ 60 % d'opacité). Panneaux : environnement/calques (gauche), interventions et temps (bas),
inspecteur télémétrique (droite).

## Structure
```
crates/sim-core   noyau déterministe (aucune dépendance)
crates/homo-app   rendu Bevy 0.15 + HUD egui + thread de simulation
docs/             architecture, pipeline d'assets
```
