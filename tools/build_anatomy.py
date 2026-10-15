#!/usr/bin/env python3
"""
Homo Digitalis - Pipeline de génération des maillages 3D anatomiques réalistes.
Télécharge les structures médicales segmentées (BodyParts3D / Human Reference Atlas CC-BY),
fusionne par systèmes fonctionnels (Squelette, Vertèbres, Muscles, Vaisseaux, Nerfs, Viscères, Peau)
et exporte des conteneurs glTF 2.0 binaires (.glb) prêts pour Bevy 0.15 et WebAssembly.
"""

import os
import sys
import json
import struct
import urllib.request
import concurrent.futures
import numpy as np
from pathlib import Path

CACHE_DIR = Path("assets_cache")
OUTPUT_DIR = Path("crates/homo-app/assets/models")

BASE_URL_ATLAS = "https://raw.githubusercontent.com/slorksmo/Human-Atlas/main/public/models"

def download_file(url: str, dest: Path) -> None:
    if dest.exists() and dest.stat().st_size > 0:
        return
    dest.parent.mkdir(parents=True, exist_ok=True)
    tmp_dest = dest.with_suffix(".tmp")
    req = urllib.request.Request(url, headers={"User-Agent": "HomoDigitalis/1.0"})
    with urllib.request.urlopen(req) as resp, open(tmp_dest, "wb") as f:
        while True:
            chunk = resp.read(1024 * 1024)
            if not chunk:
                break
            f.write(chunk)
    tmp_dest.replace(dest)

def download_all(files: list[str]) -> None:
    CACHE_DIR.mkdir(parents=True, exist_ok=True)
    urls = [(f"{BASE_URL_ATLAS}/{fname}", CACHE_DIR / fname) for fname in files]
    with concurrent.futures.ThreadPoolExecutor(max_workers=6) as executor:
        futures = [executor.submit(download_file, url, path) for url, path in urls]
        for f in concurrent.futures.as_completed(futures):
            f.result()

def classify_male(p: dict) -> str:
    sys = p.get("system", "")
    name = p.get("name", "").lower()
    if sys == "integumentary" and "skin" in name:
        return "Skin"
    if sys == "skeletal":
        if any(w in name for w in ["vertebra", "atlas", "axis", "sacrum", "coccyx"]):
            return "Vertebrae"
        return "Skeleton"
    if sys == "muscular":
        return "Muscles"
    if sys == "cardiac":
        return "Heart"
    if sys == "arterial":
        return "Arteries"
    if sys == "venous":
        return "Veins"
    if sys in ["nervous", "brain", "sensory"]:
        return "Nervous"
    if sys in ["respiratory", "digestive", "urinary", "endocrine", "lymphatic", "reproductive"]:
        return "Viscera"
    if sys == "connective":
        return "Skeleton"
    return "Viscera"

def classify_female(p: dict) -> str:
    sys = p.get("system", "")
    name = p.get("name", "").lower()
    if sys == "integumentary" and "skin" in name:
        return "Skin"
    if sys in ["skeletal", "borrowed"]:
        if any(w in name for w in ["vertebra", "atlas", "axis", "sacrum", "coccyx"]):
            return "Vertebrae"
        return "Skeleton"
    if sys in ["muscular", "donor-muscle"]:
        return "Muscles"
    if sys == "cardiac":
        return "Heart"
    if sys == "arterial":
        return "Arteries"
    if sys == "venous":
        return "Veins"
    if sys in ["nervous", "brain", "sensory"]:
        return "Nervous"
    if sys in ["respiratory", "digestive", "urinary", "endocrine", "lymphatic", "reproductive", "pregnancy"]:
        return "Viscera"
    if sys == "connective":
        return "Skeleton"
    return "Viscera"

# Couleurs par défaut PBR réalistes
GROUP_COLORS = {
    "Skin":      [0.86, 0.68, 0.58, 0.25], # chair translucide
    "Vertebrae": [0.96, 0.94, 0.88, 1.0],  # ivoire os vertèbre
    "Skeleton":  [0.91, 0.88, 0.80, 1.0],  # ivoire osseux
    "Muscles":   [0.72, 0.22, 0.20, 1.0],  # rouge myocyte
    "Heart":     [0.82, 0.16, 0.16, 1.0],  # myocarde cardiaque
    "Arteries":  [0.88, 0.12, 0.12, 1.0],  # rouge artériel sang oxygéné
    "Veins":     [0.22, 0.38, 0.68, 1.0],  # bleu veineux retour cave
    "Nervous":   [0.94, 0.88, 0.35, 1.0],  # jaune myéline / tronc nerveux
    "Viscera":   [0.78, 0.45, 0.48, 1.0],  # parenchyme viscéral
}

# Ordre d'affichage dans la hiérarchie de la scène
GROUP_ORDER = [
    "Skin",
    "Vertebrae",
    "Skeleton",
    "Muscles",
    "Heart",
    "Arteries",
    "Veins",
    "Nervous",
    "Viscera",
]

def build_glb_from_parts(atlas_file: Path, bin_prefix: str, classify_fn, output_file: Path) -> None:
    print(f"--> Lecture atlas {atlas_file}...")
    with open(atlas_file, "r", encoding="utf-8") as f:
        atlas = json.load(f)
    
    parts = atlas["parts"]
    chunks_meta = atlas["chunks"]
    print(f"    {len(parts)} pièces anatomiques réparties sur {len(chunks_meta)} chunks.")
    
    # Chargement des buffers en mémoire
    chunk_buffers = []
    for ci in range(len(chunks_meta)):
        bin_path = CACHE_DIR / f"{bin_prefix}-{ci}.bin"
        with open(bin_path, "rb") as f:
            chunk_buffers.append(f.read())
            
    # Regroupement des géométries
    grouped_positions = {g: [] for g in GROUP_ORDER}
    grouped_normals = {g: [] for g in GROUP_ORDER}
    grouped_indices = {g: [] for g in GROUP_ORDER}
    vertex_offsets = {g: 0 for g in GROUP_ORDER}
    
    for p in parts:
        g = classify_fn(p)
        ci = p["chunk"]
        buf = chunk_buffers[ci]
        
        vc = p["vertexCount"]
        ic = p["indexCount"]
        
        pos = np.frombuffer(buf, dtype=np.float32, count=vc * 3, offset=p["positions"]).reshape(-1, 3)
        # Normaux int16 normalisés
        norm = (np.frombuffer(buf, dtype=np.int16, count=vc * 3, offset=p["normals"]).astype(np.float32) / 32767.0).reshape(-1, 3)
        idx = np.frombuffer(buf, dtype=np.uint32, count=ic, offset=p["indices"])
        
        base_v = vertex_offsets[g]
        grouped_positions[g].append(pos)
        grouped_normals[g].append(norm)
        grouped_indices[g].append(idx + base_v)
        vertex_offsets[g] += vc
        
    # Assemblage binaire glTF
    bin_bytes = bytearray()
    buffer_views = []
    accessors = []
    meshes = []
    nodes = []
    materials = []
    
    for g in GROUP_ORDER:
        if not grouped_positions[g]:
            continue
            
        pos_all = np.vstack(grouped_positions[g]).astype(np.float32)
        norm_all = np.vstack(grouped_normals[g]).astype(np.float32)
        idx_all = np.concatenate(grouped_indices[g]).astype(np.uint32)
        
        print(f"    Groupe {g:12}: {len(pos_all):8} sommets, {len(idx_all)//3:8} triangles.")
        
        # Positions
        pos_offset = len(bin_bytes)
        pos_raw = pos_all.tobytes()
        bin_bytes.extend(pos_raw)
        while len(bin_bytes) % 4 != 0:
            bin_bytes.append(0)
            
        pos_view = len(buffer_views)
        buffer_views.append({
            "buffer": 0,
            "byteOffset": pos_offset,
            "byteLength": len(pos_raw),
            "target": 34962 # ARRAY_BUFFER
        })
        pos_acc = len(accessors)
        accessors.append({
            "bufferView": pos_view,
            "byteOffset": 0,
            "componentType": 5126, # FLOAT
            "count": len(pos_all),
            "type": "VEC3",
            "min": pos_all.min(axis=0).tolist(),
            "max": pos_all.max(axis=0).tolist(),
        })
        
        # Normaux
        norm_offset = len(bin_bytes)
        norm_raw = norm_all.tobytes()
        bin_bytes.extend(norm_raw)
        while len(bin_bytes) % 4 != 0:
            bin_bytes.append(0)
            
        norm_view = len(buffer_views)
        buffer_views.append({
            "buffer": 0,
            "byteOffset": norm_offset,
            "byteLength": len(norm_raw),
            "target": 34962
        })
        norm_acc = len(accessors)
        accessors.append({
            "bufferView": norm_view,
            "byteOffset": 0,
            "componentType": 5126,
            "count": len(norm_all),
            "type": "VEC3"
        })
        
        # Indices
        idx_offset = len(bin_bytes)
        idx_raw = idx_all.tobytes()
        bin_bytes.extend(idx_raw)
        while len(bin_bytes) % 4 != 0:
            bin_bytes.append(0)
            
        idx_view = len(buffer_views)
        buffer_views.append({
            "buffer": 0,
            "byteOffset": idx_offset,
            "byteLength": len(idx_raw),
            "target": 34963 # ELEMENT_ARRAY_BUFFER
        })
        idx_acc = len(accessors)
        accessors.append({
            "bufferView": idx_view,
            "byteOffset": 0,
            "componentType": 5125, # UNSIGNED_INT
            "count": len(idx_all),
            "type": "SCALAR"
        })
        
        # Matériau PBR
        color = GROUP_COLORS.get(g, [0.8, 0.8, 0.8, 1.0])
        mat_idx = len(materials)
        materials.append({
            "name": f"Material_{g}",
            "pbrMetallicRoughness": {
                "baseColorFactor": color,
                "roughnessFactor": 0.55,
                "metallicFactor": 0.05
            },
            "doubleSided": True,
            "alphaMode": "BLEND" if color[3] < 0.99 else "OPAQUE"
        })
        
        # Maillage
        mesh_idx = len(meshes)
        meshes.append({
            "name": g,
            "primitives": [{
                "attributes": {
                    "POSITION": pos_acc,
                    "NORMAL": norm_acc
                },
                "indices": idx_acc,
                "material": mat_idx
            }]
        })
        
        # Nœud
        nodes.append({
            "name": g,
            "mesh": mesh_idx
        })
        
    gltf_manifest = {
        "asset": {
            "version": "2.0",
            "generator": "Homo Digitalis Realistic Anatomy Pipeline"
        },
        "scenes": [{
            "name": "HumanScene",
            "nodes": list(range(len(nodes)))
        }],
        "scene": 0,
        "nodes": nodes,
        "meshes": meshes,
        "materials": materials,
        "accessors": accessors,
        "bufferViews": buffer_views,
        "buffers": [{"byteLength": len(bin_bytes)}]
    }
    
    json_bytes = json.dumps(gltf_manifest, separators=(',', ':')).encode("utf-8")
    while len(json_bytes) % 4 != 0:
        json_bytes += b' '
        
    while len(bin_bytes) % 4 != 0:
        bin_bytes.append(0)
        
    total_len = 12 + 8 + len(json_bytes) + 8 + len(bin_bytes)
    
    header = struct.pack("<4sII", b"glTF", 2, total_len)
    chunk0_hdr = struct.pack("<II", len(json_bytes), 0x4E4F534A)
    chunk1_hdr = struct.pack("<II", len(bin_bytes), 0x004E4942)
    
    output_file.parent.mkdir(parents=True, exist_ok=True)
    with open(output_file, "wb") as f:
        f.write(header)
        f.write(chunk0_hdr)
        f.write(json_bytes)
        f.write(chunk1_hdr)
        f.write(bin_bytes)
        
    print(f"==> Exporté {output_file} ({total_len / (1024*1024):.2f} MB, {len(nodes)} groupes).")

def main():
    print("=== Homo Digitalis - Téléchargement et Génération 3D ===")
    files_to_download = [
        "atlas.json",
        "atlas-female.json",
    ]
    for i in range(15):
        files_to_download.append(f"body-{i}.bin")
        files_to_download.append(f"female-{i}.bin")
        
    print(f"Téléchargement de {len(files_to_download)} fichiers sources...")
    download_all(files_to_download)
    print("Téléchargements terminés.")
    
    # 1. Homme (Male)
    build_glb_from_parts(
        CACHE_DIR / "atlas.json",
        "body",
        classify_male,
        OUTPUT_DIR / "human_male.glb"
    )
    
    # 2. Femme (Female)
    build_glb_from_parts(
        CACHE_DIR / "atlas-female.json",
        "female",
        classify_female,
        OUTPUT_DIR / "human_female.glb"
    )
    print("=== Succès : modèles anatomiques prêts dans assets/models ===")

if __name__ == "__main__":
    main()
