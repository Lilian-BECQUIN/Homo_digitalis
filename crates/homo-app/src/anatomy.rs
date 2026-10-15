//! Anatomie réaliste et articulée : chargement des modèles 3D segmentés
//! pour l'Homme et la Femme. Chaque calque est rattaché aux composants physiologiques,
//! et chaque articulation est identifiée pour l'animation cinématique réelle.

use bevy::picking::events::{Click, Pointer};
use bevy::picking::pointer::PointerButton;
use bevy::picking::PickingBehavior;
use bevy::prelude::*;

use crate::layers::{AnatomyLayer, BaseColor, FemaleModel, MaleModel};

/// Articulations squelettiques et biomécaniques réelles du corps humain.
#[derive(Component, Clone, Copy, PartialEq, Eq, Debug)]
pub enum BodyJoint {
    Hips,
    Spine,
    Thorax,
    Neck,
    Head,
    LeftThigh,
    LeftCalf,
    LeftFoot,
    RightThigh,
    RightCalf,
    RightFoot,
    LeftUpperArm,
    LeftForearm,
    LeftHand,
    RightUpperArm,
    RightForearm,
    RightHand,
}

impl BodyJoint {
    pub fn from_name(name: &str) -> Option<Self> {
        match name {
            "Hips" => Some(BodyJoint::Hips),
            "Spine" => Some(BodyJoint::Spine),
            "Thorax" => Some(BodyJoint::Thorax),
            "Neck" => Some(BodyJoint::Neck),
            "Head" => Some(BodyJoint::Head),
            "LeftThigh" => Some(BodyJoint::LeftThigh),
            "LeftCalf" => Some(BodyJoint::LeftCalf),
            "LeftFoot" => Some(BodyJoint::LeftFoot),
            "RightThigh" => Some(BodyJoint::RightThigh),
            "RightCalf" => Some(BodyJoint::RightCalf),
            "RightFoot" => Some(BodyJoint::RightFoot),
            "LeftUpperArm" => Some(BodyJoint::LeftUpperArm),
            "LeftForearm" => Some(BodyJoint::LeftForearm),
            "LeftHand" => Some(BodyJoint::LeftHand),
            "RightUpperArm" => Some(BodyJoint::RightUpperArm),
            "RightForearm" => Some(BodyJoint::RightForearm),
            "RightHand" => Some(BodyJoint::RightHand),
            _ => None,
        }
    }
}

/// Transform local de repos (bind pose) de chaque articulation anatomique.
#[derive(Component, Clone, Copy)]
pub struct BaseJointTransform(pub Transform);

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

/// Cœur : reçoit l'animation de contraction ventriculaire.
#[derive(Component)]
pub struct HeartMarker;

/// Sang oxygéné : la couleur suit la SpO₂ (HbO₂ rouge vif ↔ Hb rouge sombre).
#[derive(Component)]
pub struct Oxygenated;

/// Vaisseau pulsatile : le rayon suit la pression artérielle.
#[derive(Component)]
pub struct Pulsatile;

#[derive(Component)]
pub struct VertebraeMarker;

#[derive(Component)]
pub struct VisceraMarker;

#[derive(Component)]
pub struct MusclesMarker;

#[derive(Component)]
pub struct BaseScale(pub Vec3);

#[derive(Resource, Default)]
pub struct Selection {
    pub name: Option<String>,
    pub info: &'static str,
    pub tag: Option<OrganTag>,
    pub layer: Option<AnatomyLayer>,
}

fn spawn_scenes(mut commands: Commands, asset_server: Res<AssetServer>) {
    commands.init_resource::<Selection>();

    // Homme (chargé au centre, visible par défaut)
    commands.spawn((
        SceneRoot(asset_server.load("models/human_male.glb#Scene0")),
        MaleModel,
        Visibility::Inherited,
        Transform::from_translation(Vec3::new(0.0, 0.0, 0.0)),
    ));

    // Femme (chargée au centre, masquée par défaut jusqu'au basculement)
    commands.spawn((
        SceneRoot(asset_server.load("models/human_female.glb#Scene0")),
        FemaleModel,
        Visibility::Hidden,
        Transform::from_translation(Vec3::new(0.0, 0.0, 0.0)),
    ));
}

/// Système dynamique qui configure chaque pièce anatomique dès que le glTF est chargé dans la scène.
#[allow(clippy::type_complexity)]
fn setup_loaded_nodes(
    mut commands: Commands,
    query: Query<(Entity, &Name), (Without<AnatomyLayer>, With<Mesh3d>)>,
) {
    for (entity, name) in &query {
        let n = name.as_str();
        let mut ec = commands.entity(entity);

        if n.ends_with("Skin") {
            ec.insert((
                AnatomyLayer::Skin,
                BaseColor(Color::srgba(0.86, 0.68, 0.58, 1.0)),
                Organ {
                    info: "Enveloppe cutanée humaine : barrière physique protectrice, thermorégulation et sudation.",
                    tag: OrganTag::None,
                },
                BaseScale(Vec3::ONE),
                PickingBehavior::default(),
            )).observe(on_click);
        } else if n.ends_with("Vertebrae") {
            ec.insert((
                AnatomyLayer::Vertebrae,
                BaseColor(Color::srgb(0.96, 0.94, 0.88)),
                Organ {
                    info: "Rachis complet : 7 vertèbres cervicales (C1 atlas, C2 axis... C7), 12 thoraciques (T1-T12), 5 lombaires (L1-L5), sacrum et coccyx.",
                    tag: OrganTag::None,
                },
                BaseScale(Vec3::ONE),
                VertebraeMarker,
                PickingBehavior::default(),
            )).observe(on_click);
        } else if n.ends_with("Skeleton") {
            ec.insert((
                AnatomyLayer::Skeleton,
                BaseColor(Color::srgb(0.91, 0.88, 0.80)),
                Organ {
                    info: "Squelette osseux : crâne, côtes I-XII, sternum, clavicules, omoplates, humérus, radius, ulna, bassin, fémurs, tibias.",
                    tag: OrganTag::None,
                },
                BaseScale(Vec3::ONE),
                PickingBehavior::default(),
            )).observe(on_click);
        } else if n.ends_with("Muscles") {
            ec.insert((
                AnatomyLayer::Muscle,
                BaseColor(Color::srgb(0.72, 0.22, 0.20)),
                Organ {
                    info: "Système musculaire articulé : pectoraux, deltoïdes, biceps, triceps, abdominaux, fessiers, quadriceps, ischio-jambiers, mollets.",
                    tag: OrganTag::None,
                },
                BaseScale(Vec3::ONE),
                MusclesMarker,
                PickingBehavior::default(),
            )).observe(on_click);
        } else if n.ends_with("Heart") {
            ec.insert((
                AnatomyLayer::Heart,
                BaseColor(Color::srgb(0.82, 0.16, 0.16)),
                Organ {
                    info: "Cœur & cavités : ventricules et oreillettes à élastance variable. Se contracte en temps réel selon le rythme cardiaque simulé.",
                    tag: OrganTag::Heart,
                },
                BaseScale(Vec3::ONE),
                HeartMarker,
                Oxygenated,
                PickingBehavior::default(),
            )).observe(on_click);
        } else if n.ends_with("Arteries") {
            ec.insert((
                AnatomyLayer::Arteries,
                BaseColor(Color::srgb(0.88, 0.12, 0.12)),
                Organ {
                    info: "Arbre artériel pulsatile : aorte, artères carotides, sous-clavières, fémorales, tibiales. Rayon indexé sur la pression artérielle.",
                    tag: OrganTag::Artery,
                },
                BaseScale(Vec3::ONE),
                Oxygenated,
                Pulsatile,
                PickingBehavior::default(),
            )).observe(on_click);
        } else if n.ends_with("Veins") {
            ec.insert((
                AnatomyLayer::Veins,
                BaseColor(Color::srgb(0.22, 0.38, 0.68)),
                Organ {
                    info: "Réseau veineux de retour : veines caves supérieure et inférieure, jugulaires, rénales, fémorales, saphènes.",
                    tag: OrganTag::Vein,
                },
                BaseScale(Vec3::ONE),
                PickingBehavior::default(),
            )).observe(on_click);
        } else if n.ends_with("Nervous") {
            ec.insert((
                AnatomyLayer::Nervous,
                BaseColor(Color::srgb(0.94, 0.88, 0.35)),
                Organ {
                    info: "Système nerveux central & périphérique : encéphale, cervelet, moelle épinière, tronc cérébral, plexus brachial et nerfs sciatiques.",
                    tag: OrganTag::None,
                },
                BaseScale(Vec3::ONE),
                PickingBehavior::default(),
            )).observe(on_click);
        } else if n.ends_with("Viscera") {
            ec.insert((
                AnatomyLayer::Viscera,
                BaseColor(Color::srgb(0.78, 0.45, 0.48)),
                Organ {
                    info: "Organes internes : poumons (ventilation & échanges gazeux), foie, estomac, intestins, reins (filtration glomérulaire).",
                    tag: OrganTag::None,
                },
                BaseScale(Vec3::ONE),
                VisceraMarker,
                PickingBehavior::default(),
            )).observe(on_click);
        }
    }
}

/// Identifie et enregistre les 15 nœuds d'articulations biomécaniques réelles.
fn setup_loaded_joints(
    mut commands: Commands,
    query: Query<(Entity, &Name, &Transform), (Without<BodyJoint>, Without<Mesh3d>)>,
) {
    for (entity, name, tf) in &query {
        let n = name.as_str();
        if let Some(joint) = BodyJoint::from_name(n) {
            commands.entity(entity).insert((
                joint,
                BaseJointTransform(*tf),
            ));
        }
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
        app.add_systems(Startup, spawn_scenes)
            .add_systems(Update, (setup_loaded_nodes, setup_loaded_joints));
    }
}
