//! Calques anatomiques (9 couches segmentées) et leur opacité contrôlable en temps réel.
//! Inclut la sélection du sexe (Homme / Femme) et les préréglages d'isolation (solo).

use bevy::color::Alpha;
use bevy::picking::PickingBehavior;
use bevy::prelude::*;

#[derive(Component, Clone, Copy, PartialEq, Eq, Debug)]
pub enum AnatomyLayer {
    Skin = 0,
    Vertebrae = 1,
    Skeleton = 2,
    Muscle = 3,
    Heart = 4,
    Arteries = 5,
    Veins = 6,
    Nervous = 7,
    Viscera = 8,
}

impl AnatomyLayer {
    pub const ALL: [AnatomyLayer; 9] = [
        AnatomyLayer::Skin,
        AnatomyLayer::Vertebrae,
        AnatomyLayer::Skeleton,
        AnatomyLayer::Muscle,
        AnatomyLayer::Heart,
        AnatomyLayer::Arteries,
        AnatomyLayer::Veins,
        AnatomyLayer::Nervous,
        AnatomyLayer::Viscera,
    ];

    pub fn index(self) -> usize {
        self as usize
    }

    pub fn label(self) -> &'static str {
        match self {
            AnatomyLayer::Skin => "Enveloppe cutanée (Peau)",
            AnatomyLayer::Vertebrae => "Colonne vertébrale (Vertèbres & Rachis)",
            AnatomyLayer::Skeleton => "Squelette osseux & crâne",
            AnatomyLayer::Muscle => "Système musculaire complet",
            AnatomyLayer::Heart => "Cœur & cavités cardiaques",
            AnatomyLayer::Arteries => "Arbre artériel pulsatile",
            AnatomyLayer::Veins => "Toutes les veines (réseau de retour)",
            AnatomyLayer::Nervous => "Système nerveux (encéphale & nerfs)",
            AnatomyLayer::Viscera => "Organes internes (poumons, viscères)",
        }
    }

    pub fn default_opacity(self) -> f32 {
        match self {
            AnatomyLayer::Skin => 0.18,
            AnatomyLayer::Vertebrae => 1.0,
            AnatomyLayer::Skeleton => 0.95,
            AnatomyLayer::Muscle => 0.75,
            AnatomyLayer::Heart => 1.0,
            AnatomyLayer::Arteries => 1.0,
            AnatomyLayer::Veins => 1.0,
            AnatomyLayer::Nervous => 0.85,
            AnatomyLayer::Viscera => 0.9,
        }
    }
}

#[derive(Resource, Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum Gender {
    #[default]
    Male,
    Female,
}

impl Gender {
    pub fn label(self) -> &'static str {
        match self {
            Gender::Male => "Homme",
            Gender::Female => "Femme",
        }
    }
}

#[derive(Component)]
pub struct MaleModel;

#[derive(Component)]
pub struct FemaleModel;

#[derive(Resource)]
pub struct LayerSettings {
    pub opacity: [f32; 9],
}

impl Default for LayerSettings {
    fn default() -> Self {
        let mut opacity = [0.0; 9];
        for l in AnatomyLayer::ALL {
            opacity[l.index()] = l.default_opacity();
        }
        Self { opacity }
    }
}

impl LayerSettings {
    pub fn preset_all(&mut self) {
        for l in AnatomyLayer::ALL {
            self.opacity[l.index()] = l.default_opacity();
        }
    }

    pub fn preset_skin_only(&mut self) {
        self.opacity = [0.0; 9];
        self.opacity[AnatomyLayer::Skin.index()] = 1.0;
    }

    pub fn preset_skeleton_only(&mut self) {
        self.opacity = [0.0; 9];
        self.opacity[AnatomyLayer::Vertebrae.index()] = 1.0;
        self.opacity[AnatomyLayer::Skeleton.index()] = 1.0;
    }

    pub fn preset_vertebrae_only(&mut self) {
        self.opacity = [0.0; 9];
        self.opacity[AnatomyLayer::Vertebrae.index()] = 1.0;
    }

    pub fn preset_muscles_only(&mut self) {
        self.opacity = [0.0; 9];
        self.opacity[AnatomyLayer::Muscle.index()] = 1.0;
    }

    pub fn preset_vascular_only(&mut self) {
        self.opacity = [0.0; 9];
        self.opacity[AnatomyLayer::Heart.index()] = 1.0;
        self.opacity[AnatomyLayer::Arteries.index()] = 1.0;
        self.opacity[AnatomyLayer::Veins.index()] = 1.0;
    }

    pub fn preset_veins_only(&mut self) {
        self.opacity = [0.0; 9];
        self.opacity[AnatomyLayer::Veins.index()] = 1.0;
        self.opacity[AnatomyLayer::Heart.index()] = 0.4;
    }

    pub fn preset_nervous_only(&mut self) {
        self.opacity = [0.0; 9];
        self.opacity[AnatomyLayer::Nervous.index()] = 1.0;
        self.opacity[AnatomyLayer::Vertebrae.index()] = 0.15;
    }

    pub fn preset_viscera_only(&mut self) {
        self.opacity = [0.0; 9];
        self.opacity[AnatomyLayer::Viscera.index()] = 1.0;
        self.opacity[AnatomyLayer::Heart.index()] = 1.0;
    }
}

/// Couleur de base (sans alpha) d'une pièce anatomique ; modulée par les shaders physiologiques.
#[derive(Component, Clone, Copy)]
pub struct BaseColor(pub Color);

/// Sous ce seuil d'opacité une couche est un « fantôme » : visible mais non sélectionnable.
const SELECTABLE_OPACITY: f32 = 0.6;

fn apply_layer_materials(
    settings: Res<LayerSettings>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut query: Query<(
        &AnatomyLayer,
        Ref<BaseColor>,
        &MeshMaterial3d<StandardMaterial>,
        &mut Visibility,
        &mut PickingBehavior,
    )>,
) {
    let all = settings.is_changed();
    for (layer, base, mat, mut vis, mut picking) in &mut query {
        if !(all || base.is_changed()) {
            continue;
        }
        let a = settings.opacity[layer.index()].clamp(0.0, 1.0);
        *vis = if a < 0.015 { Visibility::Hidden } else { Visibility::Inherited };
        let selectable = a >= SELECTABLE_OPACITY;
        picking.should_block_lower = selectable;
        picking.is_hoverable = selectable;
        if let Some(m) = materials.get_mut(&mat.0) {
            m.base_color = base.0.with_alpha(a);
            m.alpha_mode = if a >= 0.999 { AlphaMode::Opaque } else { AlphaMode::Blend };
            if *layer == AnatomyLayer::Nervous {
                m.emissive = LinearRgba::new(0.35 * a, 0.30 * a, 0.05 * a, 1.0);
            }
        }
    }
}

fn update_gender_visibility(
    gender: Res<Gender>,
    mut male_query: Query<&mut Visibility, (With<MaleModel>, Without<FemaleModel>)>,
    mut female_query: Query<&mut Visibility, (With<FemaleModel>, Without<MaleModel>)>,
) {
    if !gender.is_changed() {
        return;
    }
    for mut vis in &mut male_query {
        *vis = if *gender == Gender::Male { Visibility::Inherited } else { Visibility::Hidden };
    }
    for mut vis in &mut female_query {
        *vis = if *gender == Gender::Female { Visibility::Inherited } else { Visibility::Hidden };
    }
}

pub struct LayersPlugin;

impl Plugin for LayersPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<LayerSettings>()
            .init_resource::<Gender>()
            .add_systems(Update, (apply_layer_materials, update_gender_visibility));
    }
}
