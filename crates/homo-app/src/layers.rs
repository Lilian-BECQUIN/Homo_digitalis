//! Calques anatomiques (6 couches) et leur opacité contrôlable en temps réel.

use bevy::color::Alpha;
use bevy::picking::PickingBehavior;
use bevy::prelude::*;

#[derive(Component, Clone, Copy, PartialEq, Eq, Debug)]
pub enum AnatomyLayer {
    Skin,
    Muscle,
    Skeleton,
    Vascular,
    Nervous,
    Viscera,
}

impl AnatomyLayer {
    pub const ALL: [AnatomyLayer; 6] = [
        AnatomyLayer::Skin,
        AnatomyLayer::Muscle,
        AnatomyLayer::Skeleton,
        AnatomyLayer::Vascular,
        AnatomyLayer::Nervous,
        AnatomyLayer::Viscera,
    ];

    pub fn index(self) -> usize {
        self as usize
    }

    pub fn label(self) -> &'static str {
        match self {
            AnatomyLayer::Skin => "Enveloppe cutanée",
            AnatomyLayer::Muscle => "Système musculaire",
            AnatomyLayer::Skeleton => "Squelette & articulations",
            AnatomyLayer::Vascular => "Cœur & arbre vasculaire",
            AnatomyLayer::Nervous => "Système nerveux",
            AnatomyLayer::Viscera => "Organes internes",
        }
    }

    fn default_opacity(self) -> f32 {
        match self {
            AnatomyLayer::Skin => 0.12,
            AnatomyLayer::Muscle => 0.0,
            AnatomyLayer::Skeleton => 0.9,
            AnatomyLayer::Vascular => 1.0,
            AnatomyLayer::Nervous => 0.7,
            AnatomyLayer::Viscera => 1.0,
        }
    }
}

#[derive(Resource)]
pub struct LayerSettings {
    pub opacity: [f32; 6],
}

impl Default for LayerSettings {
    fn default() -> Self {
        let mut opacity = [0.0; 6];
        for l in AnatomyLayer::ALL {
            opacity[l.index()] = l.default_opacity();
        }
        Self { opacity }
    }
}

/// Couleur de base (sans alpha) d'une pièce anatomique ; modifiée par les shaders « physiologiques ».
#[derive(Component, Clone, Copy)]
pub struct BaseColor(pub Color);

/// Sous ce seuil d'opacité une couche est un « fantôme » : visible mais non sélectionnable,
/// ce qui permet de cliquer à travers la peau ou les muscles translucides.
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
        *vis = if a < 0.02 { Visibility::Hidden } else { Visibility::Inherited };
        let selectable = a >= SELECTABLE_OPACITY;
        picking.should_block_lower = selectable;
        picking.is_hoverable = selectable;
        if let Some(m) = materials.get_mut(&mat.0) {
            m.base_color = base.0.with_alpha(a);
            m.alpha_mode = if a >= 0.999 { AlphaMode::Opaque } else { AlphaMode::Blend };
        }
    }
}

pub struct LayersPlugin;

impl Plugin for LayersPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<LayerSettings>()
            .add_systems(Update, apply_layer_materials);
    }
}
