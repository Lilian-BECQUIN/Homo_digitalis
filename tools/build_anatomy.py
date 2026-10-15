#!/usr/bin/env python3
"""
Homo Digitalis - Pipeline de génération des maillages 3D anatomiques réalistes et articulés.
Télécharge les structures médicales segmentées (BodyParts3D / Human Reference Atlas CC-BY),
segmente en 15 articulations biomécaniques réelles (Hanches, Colonne, Thorax, Cou, Tête,
Cuisses, Genoux/Mollets, Pieds, Épaules/Bras, Coudes/Avant-bras, Mains),
regroupe par calques anatomiques fonctionnels (Squelette, Vertèbres, Muscles, Vaisseaux, Nerfs, Viscères, Peau)
et exporte des conteneurs glTF 2.0 binaires (.glb) hiérarchiques prêts pour Bevy 0.15 et WebAssembly.
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

# 15 articulations corporelles biomécaniques et leurs pivots anatomiques mondiaux (en mètres)
JOINTS_DEF = {
    "Hips":          {"parent": None,            "world": np.array([ 0.000, 0.950,  0.000], dtype=np.float32)},
    "Spine":         {"parent": "Hips",          "world": np.array([ 0.000, 1.100, -0.030], dtype=np.float32)},
    "Thorax":        {"parent": "Spine",         "world": np.array([ 0.000, 1.250, -0.010], dtype=np.float32)},
    "Neck":          {"parent": "Thorax",        "world": np.array([ 0.000, 1.440,  0.000], dtype=np.float32)},
    "Head":          {"parent": "Neck",          "world": np.array([ 0.000, 1.520,  0.020], dtype=np.float32)},
    
    "LeftThigh":     {"parent": "Hips",          "world": np.array([ 0.091, 0.900, -0.022], dtype=np.float32)},
    "LeftCalf":      {"parent": "LeftThigh",     "world": np.array([ 0.083, 0.450,  0.005], dtype=np.float32)},
    "LeftFoot":      {"parent": "LeftCalf",      "world": np.array([ 0.083, 0.080, -0.010], dtype=np.float32)},
    
    "RightThigh":    {"parent": "Hips",          "world": np.array([-0.092, 0.900, -0.022], dtype=np.float32)},
    "RightCalf":     {"parent": "RightThigh",    "world": np.array([-0.083, 0.450,  0.005], dtype=np.float32)},
    "RightFoot":     {"parent": "RightCalf",     "world": np.array([-0.083, 0.080, -0.010], dtype=np.float32)},
    
    "LeftUpperArm":  {"parent": "Thorax",        "world": np.array([ 0.180, 1.380, -0.025], dtype=np.float32)},
    "LeftForearm":   {"parent": "LeftUpperArm",  "world": np.array([ 0.245, 1.110, -0.010], dtype=np.float32)},
    "LeftHand":      {"parent": "LeftForearm",   "world": np.array([ 0.250, 0.880,  0.000], dtype=np.float32)},
    
    "RightUpperArm": {"parent": "Thorax",        "world": np.array([-0.180, 1.380, -0.025], dtype=np.float32)},
    "RightForearm":  {"parent": "RightUpperArm", "world": np.array([-0.245, 1.110, -0.010], dtype=np.float32)},
    "RightHand":     {"parent": "RightForearm",  "world": np.array([-0.250, 0.880,  0.000], dtype=np.float32)},
}

GROUP_COLORS = {
    "Skin":      [0.86, 0.68, 0.58, 0.25], # chair translucide
    "Vertebrae": [0.96, 0.94, 0.88, 1.0],  # ivoire os vertèbre
    "Skeleton":  [0.91, 0.88, 0.80, 1.0],  # ivoire osseux
    "Muscles":   [0.72, 0.22, 0.20, 1.0],  # rouge myocyte
    "Heart":     [0.82, 0.16, 0.16, 1.0],  # myocarde cardiaque
    "Arteries":  [0.88, 0.12, 0.12, 1.0],  # rouge artériel
    "Veins":     [0.22, 0.38, 0.68, 1.0],  # bleu veineux
    "Nervous":   [0.94, 0.88, 0.35, 1.0],  # jaune myéline
    "Viscera":   [0.78, 0.45, 0.48, 1.0],  # parenchyme viscéral
}

GROUP_ORDER = [
    "Skin", "Vertebrae", "Skeleton", "Muscles", "Heart", "Arteries", "Veins", "Nervous", "Viscera"
]

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

def classify_layer_male(p: dict) -> str:
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

def classify_layer_female(p: dict) -> str:
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

def classify_segment_point(x: float, y: float, z: float) -> str:
    """Classifie un point (x, y, z) vers l'articulation la plus pertinente."""
    # Pieds
    if y < 0.08:
        return "LeftFoot" if x > 0 else "RightFoot"
    # Mollets / Jambes inférieures
    if y < 0.45:
        return "LeftCalf" if x > 0 else "RightCalf"
    # Cuisses vs Bassin
    if y < 0.90:
        return ("LeftThigh" if x > 0 else "RightThigh") if abs(x) > 0.04 else "Hips"
    # Bras vs Tronc
    if y < 0.88 and abs(x) > 0.17:
        return "LeftHand" if x > 0 else "RightHand"
    if y < 1.12 and abs(x) > 0.17:
        return "LeftForearm" if x > 0 else "RightForearm"
    if y < 1.44 and abs(x) > 0.15:
        return "LeftUpperArm" if x > 0 else "RightUpperArm"
    # Tête & Cou
    if y > 1.50:
        return "Head"
    if y > 1.42:
        return "Neck"
    # Tronc
    if y < 1.05:
        return "Hips"
    if y < 1.22:
        return "Spine"
    return "Thorax"

def build_articulated_glb(atlas_file: Path, bin_prefix: str, classify_layer_fn, output_file: Path) -> None:
    print(f"--> Lecture atlas {atlas_file}...")
    with open(atlas_file, "r", encoding="utf-8") as f:
        atlas = json.load(f)
        
    parts = atlas["parts"]
    chunks_meta = atlas["chunks"]
    print(f"    {len(parts)} pièces anatomiques sur {len(chunks_meta)} chunks.")
    
    chunk_buffers = []
    for ci in range(len(chunks_meta)):
        with open(CACHE_DIR / f"{bin_prefix}-{ci}.bin", "rb") as f:
            chunk_buffers.append(f.read())
            
    geom = {}
    for j in JOINTS_DEF:
        for l in GROUP_ORDER:
            geom[(j, l)] = {"pos": [], "norm": [], "idx": [], "v_offset": 0}
            
    for p in parts:
        layer = classify_layer_fn(p)
        buf = chunk_buffers[p["chunk"]]
        vc = p["vertexCount"]
        ic = p["indexCount"]
        pos = np.frombuffer(buf, dtype=np.float32, count=vc*3, offset=p["positions"]).reshape(-1, 3)
        norm = (np.frombuffer(buf, dtype=np.int16, count=vc*3, offset=p["normals"]).astype(np.float32) / 32767.0).reshape(-1, 3)
        idx = np.frombuffer(buf, dtype=np.uint32, count=ic, offset=p["indices"]).reshape(-1, 3)
        
        if layer == "Skin":
            # Découpe géométrique des facettes de peau pour chaque articulation
            tri_centers = pos[idx].mean(axis=1)
            for seg in JOINTS_DEF:
                if seg == "Hips":
                    mask = (tri_centers[:, 1] >= 0.90) & (tri_centers[:, 1] < 1.05) & (np.abs(tri_centers[:, 0]) <= 0.15)
                elif seg == "Spine":
                    mask = (tri_centers[:, 1] >= 1.05) & (tri_centers[:, 1] < 1.22) & (np.abs(tri_centers[:, 0]) <= 0.15)
                elif seg == "Thorax":
                    mask = (tri_centers[:, 1] >= 1.22) & (tri_centers[:, 1] < 1.42) & (np.abs(tri_centers[:, 0]) <= 0.16)
                elif seg == "Neck":
                    mask = (tri_centers[:, 1] >= 1.42) & (tri_centers[:, 1] < 1.50)
                elif seg == "Head":
                    mask = (tri_centers[:, 1] >= 1.50)
                elif seg == "LeftThigh":
                    mask = (tri_centers[:, 1] >= 0.45) & (tri_centers[:, 1] < 0.90) & (tri_centers[:, 0] > 0.04)
                elif seg == "RightThigh":
                    mask = (tri_centers[:, 1] >= 0.45) & (tri_centers[:, 1] < 0.90) & (tri_centers[:, 0] < -0.04)
                elif seg == "LeftCalf":
                    mask = (tri_centers[:, 1] >= 0.08) & (tri_centers[:, 1] < 0.45) & (tri_centers[:, 0] > 0)
                elif seg == "RightCalf":
                    mask = (tri_centers[:, 1] >= 0.08) & (tri_centers[:, 1] < 0.45) & (tri_centers[:, 0] < 0)
                elif seg == "LeftFoot":
                    mask = (tri_centers[:, 1] < 0.08) & (tri_centers[:, 0] > 0)
                elif seg == "RightFoot":
                    mask = (tri_centers[:, 1] < 0.08) & (tri_centers[:, 0] < 0)
                elif seg == "LeftUpperArm":
                    mask = (tri_centers[:, 1] >= 1.12) & (tri_centers[:, 1] < 1.44) & (tri_centers[:, 0] > 0.15)
                elif seg == "RightUpperArm":
                    mask = (tri_centers[:, 1] >= 1.12) & (tri_centers[:, 1] < 1.44) & (tri_centers[:, 0] < -0.15)
                elif seg == "LeftForearm":
                    mask = (tri_centers[:, 1] >= 0.88) & (tri_centers[:, 1] < 1.12) & (tri_centers[:, 0] > 0.16)
                elif seg == "RightForearm":
                    mask = (tri_centers[:, 1] >= 0.88) & (tri_centers[:, 1] < 1.12) & (tri_centers[:, 0] < -0.16)
                elif seg == "LeftHand":
                    mask = (tri_centers[:, 1] < 0.88) & (tri_centers[:, 0] > 0.16)
                elif seg == "RightHand":
                    mask = (tri_centers[:, 1] < 0.88) & (tri_centers[:, 0] < -0.16)
                else:
                    mask = np.zeros(len(idx), dtype=bool)
                    
                sub_idx = idx[mask]
                if len(sub_idx) == 0:
                    continue
                u_idx, new_tri = np.unique(sub_idx, return_inverse=True)
                sub_pos = pos[u_idx] - JOINTS_DEF[seg]["world"]
                sub_norm = norm[u_idx]
                
                target = geom[(seg, "Skin")]
                target["idx"].append(new_tri.astype(np.uint32) + target["v_offset"])
                target["pos"].append(sub_pos)
                target["norm"].append(sub_norm)
                target["v_offset"] += len(u_idx)
        else:
            c = pos.mean(axis=0)
            seg = classify_segment_point(c[0], c[1], c[2])
            sub_pos = pos - JOINTS_DEF[seg]["world"]
            target = geom[(seg, layer)]
            target["idx"].append(idx.flatten() + target["v_offset"])
            target["pos"].append(sub_pos)
            target["norm"].append(norm)
            target["v_offset"] += vc
            
    bin_bytes = bytearray()
    buffer_views = []
    accessors = []
    meshes = []
    nodes = []
    materials = []
    
    mat_map = {}
    for l in GROUP_ORDER:
        col = GROUP_COLORS[l]
        mat_map[l] = len(materials)
        materials.append({
            "name": f"Material_{l}",
            "pbrMetallicRoughness": {
                "baseColorFactor": col,
                "roughnessFactor": 0.55,
                "metallicFactor": 0.05
            },
            "doubleSided": True,
            "alphaMode": "BLEND" if col[3] < 0.99 else "OPAQUE"
        })
        
    joint_mesh_children = {j: [] for j in JOINTS_DEF}
    
    for (seg, layer), gdata in geom.items():
        if not gdata["pos"]:
            continue
        pos_all = np.vstack(gdata["pos"]).astype(np.float32)
        norm_all = np.vstack(gdata["norm"]).astype(np.float32)
        idx_all = np.concatenate(gdata["idx"]).astype(np.uint32)
        
        # Position
        pos_offset = len(bin_bytes)
        pos_raw = pos_all.tobytes()
        bin_bytes.extend(pos_raw)
        while len(bin_bytes) % 4 != 0: bin_bytes.append(0)
        
        pos_view = len(buffer_views)
        buffer_views.append({"buffer": 0, "byteOffset": pos_offset, "byteLength": len(pos_raw), "target": 34962})
        pos_acc = len(accessors)
        accessors.append({
            "bufferView": pos_view, "byteOffset": 0, "componentType": 5126,
            "count": len(pos_all), "type": "VEC3",
            "min": pos_all.min(axis=0).tolist(), "max": pos_all.max(axis=0).tolist()
        })
        
        # Normal
        norm_offset = len(bin_bytes)
        norm_raw = norm_all.tobytes()
        bin_bytes.extend(norm_raw)
        while len(bin_bytes) % 4 != 0: bin_bytes.append(0)
        
        norm_view = len(buffer_views)
        buffer_views.append({"buffer": 0, "byteOffset": norm_offset, "byteLength": len(norm_raw), "target": 34962})
        norm_acc = len(accessors)
        accessors.append({"bufferView": norm_view, "byteOffset": 0, "componentType": 5126, "count": len(norm_all), "type": "VEC3"})
        
        # Indices
        idx_offset = len(bin_bytes)
        idx_raw = idx_all.tobytes()
        bin_bytes.extend(idx_raw)
        while len(bin_bytes) % 4 != 0: bin_bytes.append(0)
        
        idx_view = len(buffer_views)
        buffer_views.append({"buffer": 0, "byteOffset": idx_offset, "byteLength": len(idx_raw), "target": 34963})
        idx_acc = len(accessors)
        accessors.append({"bufferView": idx_view, "byteOffset": 0, "componentType": 5125, "count": len(idx_all), "type": "SCALAR"})
        
        mesh_idx = len(meshes)
        meshes.append({
            "name": f"{seg}_{layer}",
            "primitives": [{
                "attributes": {"POSITION": pos_acc, "NORMAL": norm_acc},
                "indices": idx_acc,
                "material": mat_map[layer]
            }]
        })
        
        node_idx = len(nodes)
        nodes.append({
            "name": f"{seg}_{layer}",
            "mesh": mesh_idx
        })
        joint_mesh_children[seg].append(node_idx)

    # Création des 15 nœuds d'articulations avec leur hiérarchie
    joint_node_indices = {}
    for jname, jinfo in JOINTS_DEF.items():
        if jinfo["parent"] is None:
            local_pos = jinfo["world"].tolist()
        else:
            pworld = JOINTS_DEF[jinfo["parent"]]["world"]
            local_pos = (jinfo["world"] - pworld).tolist()
            
        j_node_idx = len(nodes)
        joint_node_indices[jname] = j_node_idx
        nodes.append({
            "name": jname,
            "translation": local_pos,
            "children": list(joint_mesh_children[jname])
        })
        
    for jname, jinfo in JOINTS_DEF.items():
        pname = jinfo["parent"]
        if pname:
            p_idx = joint_node_indices[pname]
            nodes[p_idx]["children"].append(joint_node_indices[jname])
            
    root_node_idx = joint_node_indices["Hips"]
    
    gltf = {
        "asset": {"version": "2.0", "generator": "Homo Digitalis Realistic Articulated Anatomy"},
        "scenes": [{"name": "HumanScene", "nodes": [root_node_idx]}],
        "scene": 0,
        "nodes": nodes,
        "meshes": meshes,
        "materials": materials,
        "accessors": accessors,
        "bufferViews": buffer_views,
        "buffers": [{"byteLength": len(bin_bytes)}]
    }
    
    json_bytes = json.dumps(gltf, separators=(',', ':')).encode("utf-8")
    while len(json_bytes) % 4 != 0: json_bytes += b' '
    while len(bin_bytes) % 4 != 0: bin_bytes.append(0)
    
    total_len = 12 + 8 + len(json_bytes) + 8 + len(bin_bytes)
    hdr = struct.pack("<4sII", b"glTF", 2, total_len)
    c0 = struct.pack("<II", len(json_bytes), 0x4E4F534A)
    c1 = struct.pack("<II", len(bin_bytes), 0x004E4942)
    
    output_file.parent.mkdir(parents=True, exist_ok=True)
    with open(output_file, "wb") as f:
        f.write(hdr)
        f.write(c0)
        f.write(json_bytes)
        f.write(c1)
        f.write(bin_bytes)
        
    print(f"==> Exporté avec succès : {output_file} ({total_len / (1024*1024):.1f} MB, {len(nodes)} nœuds, 15 articulations).")

def main():
    print("=== Homo Digitalis - Génération des Modèles 3D Articulés ===")
    files_to_download = [
        "atlas.json",
        "atlas-female.json",
    ]
    for i in range(15):
        files_to_download.append(f"body-{i}.bin")
        files_to_download.append(f"female-{i}.bin")
        
    download_all(files_to_download)
    
    # 1. Homme (Male)
    build_articulated_glb(
        CACHE_DIR / "atlas.json",
        "body",
        classify_layer_male,
        OUTPUT_DIR / "human_male.glb"
    )
    
    # 2. Femme (Female)
    build_articulated_glb(
        CACHE_DIR / "atlas-female.json",
        "female",
        classify_layer_female,
        OUTPUT_DIR / "human_female.glb"
    )
    print("=== Modèles articulés prêts dans crates/homo-app/assets/models ===")

if __name__ == "__main__":
    main()
