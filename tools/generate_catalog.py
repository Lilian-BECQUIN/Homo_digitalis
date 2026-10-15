#!/usr/bin/env python3
"""Génère crates/homo-app/src/anatomy_catalog.rs à partir de atlas.json."""
import json
from pathlib import Path

ATLAS_PATH = Path("assets_cache/atlas.json")
OUTPUT_PATH = Path("crates/homo-app/src/anatomy_catalog.rs")

CATEGORY_MAP = {
    "skeletal": "🦴 Squelette & os",
    "muscular": "💪 Muscles & tendons",
    "connective": "🔗 Tendons, ligaments & aponévroses",
    "cardiac": "🫀 Cœur & cavités",
    "arterial": "🔴 Arbre artériel",
    "venous": "🔵 Réseau veineux",
    "nervous": "⚡ Système nerveux",
    "sensory": "👁️ Organes des sens",
    "respiratory": "🫁 Appareil respiratoire",
    "digestive": "🍽️ Système digestif",
    "urinary": "💧 Système urinaire",
    "endocrine": "⚖️ Système endocrinien",
    "lymphatic": "🛡️ Système lymphatique",
    "reproductive": "🧬 Système reproducteur",
    "integumentary": "🛡️ Enveloppe cutanée (peau)",
}

def get_layer_idx(sys: str, name: str) -> int:
    n = name.lower()
    if sys == "integumentary" and "skin" in n:
        return 0 # Skin
    if sys == "skeletal":
        if any(w in n for w in ["vertebra", "atlas", "axis", "sacrum", "coccyx"]):
            return 1 # Vertebrae
        return 2 # Skeleton
    if sys == "muscular":
        return 3 # Muscle
    if sys == "cardiac":
        return 4 # Heart
    if sys == "arterial":
        return 5 # Arteries
    if sys == "venous":
        return 6 # Veins
    if sys in ["nervous", "sensory", "brain"]:
        return 7 # Nervous
    if sys == "connective":
        if any(w in n for w in ["tendon", "aponeurosis"]):
            return 3 # Muscle
        return 2 # Skeleton
    return 8 # Viscera

def main():
    with open(ATLAS_PATH, "r", encoding="utf-8") as f:
        atlas = json.load(f)

    seen = set()
    entries = []
    for p in atlas["parts"]:
        name = p["name"].strip()
        sys = p.get("system", "digestive")
        key = (name, sys)
        if key in seen:
            continue
        seen.add(key)
        cat = CATEGORY_MAP.get(sys, "Organe interne")
        layer_idx = get_layer_idx(sys, name)
        # Escape string
        escaped_name = name.replace('"', '\\"')
        entries.append((escaped_name, cat, layer_idx))

    # Sort entries alphabetically
    entries.sort(key=lambda x: x[0])

    lines = [
        "// Ce fichier est généré automatiquement par tools/generate_catalog.py",
        "// Répertoire complet des 1 674 structures anatomiques humaines segmentées (BodyParts3D / Human Reference Atlas)",
        "",
        "#[derive(Clone, Copy, Debug)]",
        "pub struct AnatomyEntry {",
        "    pub name: &'static str,",
        "    pub category: &'static str,",
        "    pub layer_idx: usize,",
        "}",
        "",
        f"pub static TOTAL_ANATOMICAL_PARTS: usize = {len(entries)};",
        "",
        f"pub static ANATOMY_CATALOG: &[AnatomyEntry] = &[",
    ]

    for name, cat, l_idx in entries:
        lines.append(f'    AnatomyEntry {{ name: "{name}", category: "{cat}", layer_idx: {l_idx} }},')

    lines.extend([
        "];",
        "",
        "pub fn search_catalog(query: &str, max_results: usize) -> Vec<&'static AnatomyEntry> {",
        "    if query.is_empty() {",
        "        return ANATOMY_CATALOG.iter().take(max_results).collect();",
        "    }",
        "    let q = query.to_lowercase();",
        "    ANATOMY_CATALOG",
        "        .iter()",
        "        .filter(|e| e.name.to_lowercase().contains(&q) || e.category.to_lowercase().contains(&q))",
        "        .take(max_results)",
        "        .collect()",
        "}",
        "",
    ])

    OUTPUT_PATH.write_text("\n".join(lines), encoding="utf-8")
    print(f"Catalogue anatomique exporté : {OUTPUT_PATH} ({len(entries)} structures).")

if __name__ == "__main__":
    main()
