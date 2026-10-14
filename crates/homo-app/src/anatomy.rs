//! Anatomie de la Phase 1 : pièces *procédurales* de substitution, nommées selon la
//! nomenclature anatomique (Terminologia Anatomica). Elles seront remplacées par les
//! maillages glTF Z-Anatomy / BodyParts3D (voir `docs/ASSET_PIPELINE.md`) en conservant
//! exactement les mêmes composants (`Name`, `Organ`, `AnatomyLayer`, `BaseColor`).

use bevy::color::Alpha;
use bevy::picking::events::{Click, Pointer};
use bevy::picking::pointer::PointerButton;
use bevy::picking::PickingBehavior;
use bevy::prelude::*;

use crate::layers::{AnatomyLayer, BaseColor};

/// Rôle physiologique d'une pièce : détermine ce que l'inspecteur télémétrique affiche.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum OrganTag {
    None,
    Heart,
    Artery,
    Vein,
}

#[derive(Component)]
pub struct Organ {
    pub info: &'static str,
    pub tag: OrganTag,
}

/// Cœur : reçoit l'animation de contraction.
#[derive(Component)]
pub struct HeartMarker;

/// Sang oxygéné : la couleur suit la SpO₂ (HbO₂ rouge vif ↔ Hb rouge sombre).
#[derive(Component)]
pub struct Oxygenated;

/// Vaisseau pulsatile : le rayon suit la pression artérielle.
#[derive(Component)]
pub struct Pulsatile;

#[derive(Component)]
pub struct BaseScale(pub Vec3);

#[derive(Resource, Default)]
pub struct Selection {
    pub name: Option<String>,
    pub info: &'static str,
    pub tag: Option<OrganTag>,
    pub layer: Option<AnatomyLayer>,
}

enum Shape {
    Sphere(f32),
    Capsule(f32, f32),
    Cuboid(f32, f32, f32),
    Cylinder(f32, f32),
    Torus(f32, f32),
}

struct Part {
    name: &'static str,
    info: &'static str,
    layer: AnatomyLayer,
    shape: Shape,
    pos: [f32; 3],
    scale: [f32; 3],
    color: [f32; 3],
    tag: OrganTag,
    heart: bool,
    oxygenated: bool,
    pulsatile: bool,
}

impl Part {
    fn new(name: &'static str, layer: AnatomyLayer, shape: Shape, pos: [f32; 3], color: [f32; 3]) -> Self {
        Self {
            name,
            info: "",
            layer,
            shape,
            pos,
            scale: [1.0; 3],
            color,
            tag: OrganTag::None,
            heart: false,
            oxygenated: false,
            pulsatile: false,
        }
    }
    fn info(mut self, info: &'static str) -> Self {
        self.info = info;
        self
    }
    fn scale(mut self, s: [f32; 3]) -> Self {
        self.scale = s;
        self
    }
}

/// Place une paire symétrique (droite = x négatif côté patient, affichage de face).
fn pair(
    out: &mut Vec<Part>,
    names: [&'static str; 2],
    layer: AnatomyLayer,
    make_shape: impl Fn() -> Shape,
    pos: [f32; 3],
    scale: [f32; 3],
    color: [f32; 3],
    info: &'static str,
) {
    for (i, sign) in [-1.0_f32, 1.0].into_iter().enumerate() {
        out.push(
            Part::new(names[i], layer, make_shape(), [pos[0] * sign, pos[1], pos[2]], color)
                .scale(scale)
                .info(info),
        );
    }
}

fn parts() -> Vec<Part> {
    use AnatomyLayer::*;
    let mut v: Vec<Part> = Vec::new();

    // ---------------------------------------------------------------- Enveloppe cutanée
    let skin = [0.88, 0.68, 0.58];
    v.push(Part::new("Cutis (truncus)", Skin, Shape::Capsule(0.17, 0.45), [0.0, 1.2, 0.0], skin).scale([1.0, 1.0, 0.65]).info("Enveloppe cutanée : sudation, pâleur, cyanose, érythème (phases ultérieures)."));
    v.push(Part::new("Cutis (caput)", Skin, Shape::Sphere(0.11), [0.0, 1.66, 0.0], skin));
    pair(&mut v, ["Cutis (membrum superius dextrum)", "Cutis (membrum superius sinistrum)"], Skin, || Shape::Capsule(0.05, 0.5), [0.29, 1.15, 0.0], [1.0; 3], skin, "");
    pair(&mut v, ["Cutis (membrum inferius dextrum)", "Cutis (membrum inferius sinistrum)"], Skin, || Shape::Capsule(0.075, 0.75), [0.1, 0.5, 0.0], [1.0; 3], skin, "");

    // ---------------------------------------------------------------- Squelette
    let bone = [0.93, 0.9, 0.8];
    v.push(Part::new("Cranium", Skeleton, Shape::Sphere(0.095), [0.0, 1.66, 0.0], bone));
    for (name, y) in [("Vertebra lumbalis L5", 0.98), ("Vertebra lumbalis L4", 1.03), ("Vertebra lumbalis L3", 1.08), ("Vertebra lumbalis L2", 1.13), ("Vertebra lumbalis L1", 1.18)] {
        v.push(Part::new(name, Skeleton, Shape::Cuboid(0.055, 0.04, 0.05), [0.0, y, -0.075], bone).info("Vertèbre lombaire : le couple L4-L5 / L5-S1 sera calculé en phase 4."));
    }
    v.push(Part::new("Os sacrum", Skeleton, Shape::Cuboid(0.09, 0.08, 0.05), [0.0, 0.92, -0.075], bone));
    v.push(Part::new("Pelvis", Skeleton, Shape::Cuboid(0.3, 0.09, 0.14), [0.0, 0.9, 0.0], bone));
    for (i, y) in [1.46, 1.40, 1.34, 1.28, 1.22].into_iter().enumerate() {
        let names = ["Costa I", "Costa II", "Costa III", "Costa IV", "Costa V"];
        v.push(Part::new(names[i], Skeleton, Shape::Torus(0.115, 0.135), [0.0, y, 0.0], bone).scale([1.0, 1.0, 0.7]));
    }
    pair(&mut v, ["Humerus dexter", "Humerus sinister"], Skeleton, || Shape::Capsule(0.022, 0.28), [0.29, 1.27, 0.0], [1.0; 3], bone, "");
    pair(&mut v, ["Femur dextrum", "Femur sinistrum"], Skeleton, || Shape::Capsule(0.028, 0.4), [0.1, 0.65, 0.0], [1.0; 3], bone, "");
    pair(&mut v, ["Tibia dextra", "Tibia sinistra"], Skeleton, || Shape::Capsule(0.024, 0.38), [0.1, 0.25, 0.0], [1.0; 3], bone, "");

    // ---------------------------------------------------------------- Muscles
    let muscle = [0.72, 0.18, 0.16];
    pair(&mut v, ["Musculus pectoralis major dexter", "Musculus pectoralis major sinister"], Muscle, || Shape::Cuboid(0.12, 0.1, 0.04), [0.09, 1.38, 0.1], [1.0; 3], muscle, "Muscle : activation nerveuse → couleur bleu (repos) → orange → violet (acidose).");
    pair(&mut v, ["Musculus biceps brachii dexter", "Musculus biceps brachii sinister"], Muscle, || Shape::Capsule(0.03, 0.2), [0.29, 1.3, 0.04], [1.0; 3], muscle, "Muscle fléchisseur du coude.");
    pair(&mut v, ["Musculus erector spinae dexter", "Musculus erector spinae sinister"], Muscle, || Shape::Capsule(0.025, 0.4), [0.055, 1.15, -0.095], [1.0; 3], muscle, "Extenseur du rachis : contrainte lombaire en soulevé de charge.");
    pair(&mut v, ["Musculus gluteus maximus dexter", "Musculus gluteus maximus sinister"], Muscle, || Shape::Sphere(0.07), [0.09, 0.88, -0.08], [1.0; 3], muscle, "");
    pair(&mut v, ["Musculus quadriceps femoris dexter", "Musculus quadriceps femoris sinister"], Muscle, || Shape::Capsule(0.05, 0.35), [0.1, 0.65, 0.035], [1.0; 3], muscle, "Principal extenseur du genou.");

    // ---------------------------------------------------------------- Cœur & vaisseaux
    let mut heart = Part::new("Cor", Vascular, Shape::Sphere(0.06), [0.035, 1.36, 0.035], [0.8, 0.1, 0.1])
        .scale([1.0, 1.2, 1.0])
        .info("Ventricule gauche à élastance variable : bat au rythme de la fréquence cardiaque simulée.");
    heart.tag = OrganTag::Heart;
    heart.heart = true;
    heart.oxygenated = true;
    v.push(heart);

    let mut aorta = Part::new("Aorta thoracica", Vascular, Shape::Cylinder(0.013, 0.34), [0.0, 1.27, -0.03], [0.85, 0.12, 0.1])
        .info("Aorte : pression artérielle pulsée (Windkessel à 3 éléments).");
    aorta.tag = OrganTag::Artery;
    aorta.oxygenated = true;
    aorta.pulsatile = true;
    v.push(aorta);

    let mut abdominal = Part::new("Aorta abdominalis", Vascular, Shape::Cylinder(0.012, 0.3), [0.0, 1.03, -0.04], [0.85, 0.12, 0.1])
        .info("Aorte abdominale.");
    abdominal.tag = OrganTag::Artery;
    abdominal.oxygenated = true;
    abdominal.pulsatile = true;
    v.push(abdominal);

    let mut vena = Part::new("Vena cava inferior", Vascular, Shape::Cylinder(0.015, 0.45), [0.035, 1.12, -0.04], [0.25, 0.22, 0.55])
        .info("Retour veineux vers l'oreillette droite.");
    vena.tag = OrganTag::Vein;
    v.push(vena);

    for (i, sign) in [-1.0_f32, 1.0].into_iter().enumerate() {
        let names = ["Arteria femoralis dextra", "Arteria femoralis sinistra"];
        let mut a = Part::new(names[i], Vascular, Shape::Cylinder(0.008, 0.55), [0.09 * sign, 0.6, 0.02], [0.85, 0.12, 0.1]).info("Artère fémorale.");
        a.tag = OrganTag::Artery;
        a.oxygenated = true;
        a.pulsatile = true;
        v.push(a);
    }

    // ---------------------------------------------------------------- Système nerveux
    let nerve = [0.95, 0.9, 0.45];
    v.push(Part::new("Medulla spinalis", Nervous, Shape::Cylinder(0.009, 0.5), [0.0, 1.2, -0.078], nerve).info("Moelle épinière : décharge sympathique (phase ultérieure)."));
    pair(&mut v, ["Nervus vagus dexter", "Nervus vagus sinister"], Nervous, || Shape::Cylinder(0.004, 0.42), [0.045, 1.42, 0.0], [1.0; 3], nerve, "Nerf vague : frein parasympathique du nœud sinusal.");

    // ---------------------------------------------------------------- Viscères
    v.push(Part::new("Encephalon", Viscera, Shape::Sphere(0.075), [0.0, 1.665, 0.0], [0.86, 0.66, 0.68]).info("Cerveau : compartiment PBPK (phase 3)."));
    pair(&mut v, ["Pulmo dexter", "Pulmo sinister"], Viscera, || Shape::Sphere(0.085), [0.115, 1.38, 0.0], [0.8, 1.3, 0.7], [0.85, 0.55, 0.58], "Poumon : échanges gazeux, PaO₂/PaCO₂.");
    v.push(Part::new("Hepar", Viscera, Shape::Sphere(0.08), [-0.075, 1.24, 0.04], [0.45, 0.2, 0.12]).scale([1.4, 0.8, 0.8]).info("Foie : clairance enzymatique CYP450 (phase 3)."));
    v.push(Part::new("Gaster", Viscera, Shape::Sphere(0.05), [0.07, 1.2, 0.05], [0.8, 0.5, 0.45]).info("Estomac : absorption (phase 3)."));
    pair(&mut v, ["Ren dexter", "Ren sinister"], Viscera, || Shape::Sphere(0.035), [0.075, 1.12, -0.045], [0.8, 1.3, 0.8], [0.55, 0.17, 0.17], "Rein : filtration, équilibre hydro-électrolytique.");
    v.push(Part::new("Intestinum tenue", Viscera, Shape::Sphere(0.07), [0.0, 1.03, 0.05], [0.9, 0.7, 0.6]).scale([1.7, 0.8, 0.8]).info("Intestin grêle : absorption des nutriments."));

    v
}

fn spawn_anatomy(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    commands.init_resource::<Selection>();
    for p in parts() {
        let mesh = match p.shape {
            Shape::Sphere(r) => meshes.add(Sphere::new(r)),
            Shape::Capsule(r, l) => meshes.add(Capsule3d::new(r, l)),
            Shape::Cuboid(x, y, z) => meshes.add(Cuboid::new(x, y, z)),
            Shape::Cylinder(r, h) => meshes.add(Cylinder::new(r, h)),
            Shape::Torus(i, o) => meshes.add(Torus::new(i, o)),
        };
        let color = Color::srgb(p.color[0], p.color[1], p.color[2]);
        let emissive = if p.layer == AnatomyLayer::Nervous {
            LinearRgba::new(0.35, 0.3, 0.05, 1.0)
        } else {
            LinearRgba::BLACK
        };
        let material = materials.add(StandardMaterial {
            base_color: color.with_alpha(1.0),
            perceptual_roughness: 0.55,
            emissive,
            ..default()
        });
        let scale = Vec3::from(p.scale);
        let mut entity = commands.spawn((
            Name::new(p.name),
            Organ { info: p.info, tag: p.tag },
            p.layer,
            BaseColor(color),
            Mesh3d(mesh),
            MeshMaterial3d(material),
            Transform::from_translation(Vec3::from(p.pos)).with_scale(scale),
            PickingBehavior::default(),
            BaseScale(scale),
        ));
        if p.heart {
            entity.insert(HeartMarker);
        }
        if p.oxygenated {
            entity.insert(Oxygenated);
        }
        if p.pulsatile {
            entity.insert(Pulsatile);
        }
        entity.observe(on_click);
    }
}

fn on_click(
    trigger: Trigger<Pointer<Click>>,
    mut selection: ResMut<Selection>,
    query: Query<(&Name, &Organ, &AnatomyLayer)>,
) {
    if trigger.event().button != PointerButton::Primary {
        return;
    }
    if let Ok((name, organ, layer)) = query.get(trigger.entity()) {
        selection.name = Some(name.to_string());
        selection.info = organ.info;
        selection.tag = Some(organ.tag);
        selection.layer = Some(*layer);
    }
}

pub struct AnatomyPlugin;

impl Plugin for AnatomyPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, spawn_anatomy);
    }
}
