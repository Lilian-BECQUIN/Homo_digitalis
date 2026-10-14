# Pipeline d'assets 3D (Phase 1b)

Objectif : remplacer les primitives procédurales de `crates/homo-app/src/anatomy.rs` par des
maillages anatomiques segmentés, **en conservant la nomenclature exacte**.

## Sources (open-source)
- **Z-Anatomy** (CC BY-SA 4.0) — fichier `.blend`, noms TA2 par structure.
- **BodyParts3D** (CC BY-SA 2.1 JP) — OBJ par structure FMA.

> Respecter les licences : le dépôt doit contenir l'attribution et rester sous une licence
> compatible (partage à l'identique) pour les assets dérivés.

## Étapes Blender
1. Placer les sources dans `assets_src/` (ignoré par git).
2. Importer, ne garder que les 6 calques : peau, muscles, squelette, vaisseaux+cœur, nerfs, viscères.
3. Conserver le **nom d'objet exact** (TA2) : il devient le composant `Name` côté Bevy.
4. Décimer (Modifier *Decimate* ratio 0,1–0,3) en gardant les UV ; créer LOD0/LOD1/LOD2
   (`nom.LOD1`…). Viser < 500 k triangles au total pour LOD0.
5. Cœur : créer un *shape key* « Systole » (morph target) ; aorte/artères : shape key « Pulse ».
6. Organiser en collections `Layer_Skin`, `Layer_Muscle`, `Layer_Skeleton`, `Layer_Vascular`,
   `Layer_Nervous`, `Layer_Viscera`, avec propriété personnalisée `organ_tag` (`heart`, `artery`, `vein`).
7. Exporter en **glTF 2.0 binaire** vers `crates/homo-app/assets/models/anatomy.glb`
   (« +Y up », shape keys et extras activés).

## Intégration Bevy
Charger le glb comme `SceneRoot`, puis un système `Added<Name>` attache `AnatomyLayer`
(d'après la collection), `Organ`, `BaseColor`, `PickingBehavior`, `BaseScale` et l'observer
`on_click` — exactement les mêmes composants que `spawn_anatomy` aujourd'hui, de sorte que
`layers.rs`, `visuals.rs` et `ui.rs` ne changent pas.
