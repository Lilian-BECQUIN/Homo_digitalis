#!/usr/bin/env python3
"""
Homo Digitalis - Pipeline de génération des maillages 3D anatomiques réalistes et articulés (Skinning LBS).
Télécharge les structures médicales segmentées (BodyParts3D / Human Reference Atlas CC-BY),
génère les 9 calques anatomiques fonctionnels complets et continus (aucun découpage agressif, aucune déchirure),
calcule les pondérations de skinning squelettique (Linear Blend Skinning sur 17 articulations biomécaniques)
et exporte des conteneurs glTF 2.0 binaires (.glb) avec skinning GPU prêt pour Bevy 0.15 et WebAssembly.
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

# 21 articulations squelettiques biomécaniques réelles (Segments osseux A -> B)
BONES = [
    ("Hips",          np.array([ 0.000, 0.900, -0.020], dtype=np.float32), np.array([ 0.000, 1.020, -0.030], dtype=np.float32), None),
    ("Spine",         np.array([ 0.000, 1.020, -0.030], dtype=np.float32), np.array([ 0.000, 1.180, -0.020], dtype=np.float32), "Hips"),
    ("Thorax",        np.array([ 0.000, 1.180, -0.020], dtype=np.float32), np.array([ 0.000, 1.420,  0.000], dtype=np.float32), "Spine"),
    ("Neck",          np.array([ 0.000, 1.420,  0.000], dtype=np.float32), np.array([ 0.000, 1.500,  0.010], dtype=np.float32), "Thorax"),
    ("Head",          np.array([ 0.000, 1.500,  0.010], dtype=np.float32), np.array([ 0.000, 1.720,  0.020], dtype=np.float32), "Neck"),
    
    ("LeftClavicle",  np.array([ 0.020, 1.410,  0.005], dtype=np.float32), np.array([ 0.170, 1.390, -0.025], dtype=np.float32), "Thorax"),
    ("LeftUpperArm",  np.array([ 0.170, 1.390, -0.025], dtype=np.float32), np.array([ 0.245, 1.110, -0.010], dtype=np.float32), "LeftClavicle"),
    ("LeftForearm",   np.array([ 0.245, 1.110, -0.010], dtype=np.float32), np.array([ 0.250, 0.880,  0.000], dtype=np.float32), "LeftUpperArm"),
    ("LeftHand",      np.array([ 0.250, 0.880,  0.000], dtype=np.float32), np.array([ 0.260, 0.720,  0.000], dtype=np.float32), "LeftForearm"),
    
    ("RightClavicle", np.array([-0.020, 1.410,  0.005], dtype=np.float32), np.array([-0.170, 1.390, -0.025], dtype=np.float32), "Thorax"),
    ("RightUpperArm", np.array([-0.170, 1.390, -0.025], dtype=np.float32), np.array([-0.245, 1.110, -0.010], dtype=np.float32), "RightClavicle"),
    ("RightForearm",  np.array([-0.245, 1.110, -0.010], dtype=np.float32), np.array([-0.250, 0.880,  0.000], dtype=np.float32), "RightUpperArm"),
    ("RightHand",     np.array([-0.250, 0.880,  0.000], dtype=np.float32), np.array([-0.260, 0.720,  0.000], dtype=np.float32), "RightForearm"),
    
    ("LeftThigh",     np.array([ 0.091, 0.900, -0.022], dtype=np.float32), np.array([ 0.083, 0.450,  0.005], dtype=np.float32), "Hips"),
    ("LeftCalf",      np.array([ 0.083, 0.450,  0.005], dtype=np.float32), np.array([ 0.083, 0.080, -0.010], dtype=np.float32), "LeftThigh"),
    ("LeftFoot",      np.array([ 0.083, 0.080, -0.010], dtype=np.float32), np.array([ 0.083, 0.035,  0.065], dtype=np.float32), "LeftCalf"),
    ("LeftToes",      np.array([ 0.083, 0.035,  0.065], dtype=np.float32), np.array([ 0.083, 0.015,  0.135], dtype=np.float32), "LeftFoot"),
    
    ("RightThigh",    np.array([-0.092, 0.900, -0.022], dtype=np.float32), np.array([-0.083, 0.450,  0.005], dtype=np.float32), "Hips"),
    ("RightCalf",     np.array([-0.083, 0.450,  0.005], dtype=np.float32), np.array([-0.083, 0.080, -0.010], dtype=np.float32), "RightThigh"),
    ("RightFoot",     np.array([-0.083, 0.080, -0.010], dtype=np.float32), np.array([-0.083, 0.035,  0.065], dtype=np.float32), "RightCalf"),
    ("RightToes",     np.array([-0.083, 0.035,  0.065], dtype=np.float32), np.array([-0.083, 0.015,  0.135], dtype=np.float32), "RightFoot"),
]

BONE_NAMES = [b[0] for b in BONES]
BONE_NAME_TO_IDX = {b[0]: i for i, b in enumerate(BONES)}

bone_A = np.array([b[1] for b in BONES], dtype=np.float32)
bone_B = np.array([b[2] for b in BONES], dtype=np.float32)
bone_AB = bone_B - bone_A
bone_len_sq = np.sum(bone_AB**2, axis=1)

GROUP_COLORS = {
    "Skin":      [0.86, 0.68, 0.58, 0.25],
    "Vertebrae": [0.96, 0.94, 0.88, 1.0],
    "Skeleton":  [0.91, 0.88, 0.80, 1.0],
    "Muscles":   [0.72, 0.22, 0.20, 1.0],
    "Heart":     [0.82, 0.16, 0.16, 1.0],
    "Arteries":  [0.88, 0.12, 0.12, 1.0],
    "Veins":     [0.22, 0.38, 0.68, 1.0],
    "Nervous":   [0.94, 0.88, 0.35, 1.0],
    "Viscera":   [0.78, 0.45, 0.48, 1.0],
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
        if any(w in name for w in ["tendon", "aponeurosis"]):
            return "Muscles"
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
        if any(w in name for w in ["tendon", "aponeurosis"]):
            return "Muscles"
        return "Skeleton"
    return "Viscera"

TRUNK_BONES = {'Hips', 'Spine', 'Thorax', 'Neck', 'Head'}
ARM_L_BONES = {'LeftClavicle', 'LeftUpperArm', 'LeftForearm', 'LeftHand'}
ARM_R_BONES = {'RightClavicle', 'RightUpperArm', 'RightForearm', 'RightHand'}
LEG_L_BONES = {'LeftThigh', 'LeftCalf', 'LeftFoot', 'LeftToes'}
LEG_R_BONES = {'RightThigh', 'RightCalf', 'RightFoot', 'RightToes'}

is_trunk = np.array([b[0] in TRUNK_BONES for b in BONES])
is_arm_l = np.array([b[0] in ARM_L_BONES for b in BONES])
is_arm_r = np.array([b[0] in ARM_R_BONES for b in BONES])
is_leg_l = np.array([b[0] in LEG_L_BONES for b in BONES])
is_leg_r = np.array([b[0] in LEG_R_BONES for b in BONES])

NEIGHBORS = {
    'Hips': {'Hips', 'Spine', 'LeftThigh', 'RightThigh'},
    'Spine': {'Hips', 'Spine', 'Thorax'},
    'Thorax': {'Spine', 'Thorax', 'Neck', 'LeftClavicle', 'RightClavicle'},
    'Neck': {'Thorax', 'Neck', 'Head'},
    'Head': {'Neck', 'Head'},
    
    'LeftClavicle': {'Thorax', 'LeftClavicle', 'LeftUpperArm'},
    'LeftUpperArm': {'LeftClavicle', 'LeftUpperArm', 'LeftForearm'},
    'LeftForearm': {'LeftUpperArm', 'LeftForearm', 'LeftHand'},
    'LeftHand': {'LeftForearm', 'LeftHand'},
    
    'RightClavicle': {'Thorax', 'RightClavicle', 'RightUpperArm'},
    'RightUpperArm': {'RightClavicle', 'RightUpperArm', 'RightForearm'},
    'RightForearm': {'RightUpperArm', 'RightForearm', 'RightHand'},
    'RightHand': {'RightForearm', 'RightHand'},
    
    'LeftThigh': {'Hips', 'LeftThigh', 'LeftCalf'},
    'LeftCalf': {'LeftThigh', 'LeftCalf', 'LeftFoot'},
    'LeftFoot': {'LeftCalf', 'LeftFoot', 'LeftToes'},
    'LeftToes': {'LeftFoot', 'LeftToes'},
    
    'RightThigh': {'Hips', 'RightThigh', 'RightCalf'},
    'RightCalf': {'RightThigh', 'RightCalf', 'RightFoot'},
    'RightFoot': {'RightCalf', 'RightFoot', 'RightToes'},
    'RightToes': {'RightFoot', 'RightToes'},
}

def compute_weights_layer(pos: np.ndarray, layer: str):
    """Calcule les 4 os d'influence et leurs poids de skinning avec partitionnement cinématique et anatomique strict."""
    AP = pos[:, None, :] - bone_A[None, :, :]
    dot = np.sum(AP * bone_AB[None, :, :], axis=2)
    t = np.clip(dot / bone_len_sq[None, :], 0.0, 1.0)
    Q = bone_A[None, :, :] + t[:, :, None] * bone_AB[None, :, :]
    d_sq = np.sum((pos[:, None, :] - Q)**2, axis=2)
    
    # 1. Les organes internes et vertèbres sont 100% rattachés au squelette axial (tronc)
    if layer in ('Heart', 'Viscera', 'Vertebrae'):
        d_sq[:, ~is_trunk] += 1e6
        
    # Masquage latéral strict : gauche vs droite (aucun membre gauche influencé par la droite, et réciproquement)
    is_left_pt = (pos[:, 0] >= 0)[:, None]
    d_sq[is_left_pt & (is_arm_r | is_leg_r)[None, :]] += 1e6
    d_sq[(~is_left_pt) & (is_arm_l | is_leg_l)[None, :]] += 1e6
    
    # 2. Identification de l'os primaire le plus proche
    primary_bone_idx = np.argmin(d_sq, axis=1)
    
    # 3. Masquage cinématique strict : seuls l'os primaire et ses voisins directs
    # dans la chaîne squelettique (parent/enfants) sont autorisés à influencer le vertex.
    # Ceci élimine radicalement tout artefact d'étirement (ex: avant-bras attaché aux hanches).
    mask = np.zeros_like(d_sq, dtype=bool)
    for b_idx, (bname, _, _, _) in enumerate(BONES):
        allowed_names = NEIGHBORS[bname]
        allowed_indices = [BONE_NAME_TO_IDX[n] for n in allowed_names]
        v_in_b = (primary_bone_idx == b_idx)
        for ai in allowed_indices:
            mask[v_in_b, ai] = True
            
    d_sq[~mask] += 1e6
    
    raw_weights = 1.0 / (d_sq + 0.0025)**2
    top4_idx = np.argsort(-raw_weights, axis=1)[:, :4]
    top4_weights = np.take_along_axis(raw_weights, top4_idx, axis=1)
    sum_w = np.sum(top4_weights, axis=1, keepdims=True)
    top4_weights /= (sum_w + 1e-8)
    return top4_idx.astype(np.uint16), top4_weights.astype(np.float32)

def build_skinned_glb(atlas_file: Path, bin_prefix: str, classify_layer_fn, output_file: Path) -> None:
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
            
    # Regroupement des géométries complètes et intactes par calque (aucun découpage)
    layer_geom = {l: {"pos": [], "norm": [], "idx": [], "v_offset": 0} for l in GROUP_ORDER}
    
    for p in parts:
        layer = classify_layer_fn(p)
        buf = chunk_buffers[p["chunk"]]
        vc = p["vertexCount"]
        ic = p["indexCount"]
        pos = np.frombuffer(buf, dtype=np.float32, count=vc*3, offset=p["positions"]).reshape(-1, 3)
        norm = (np.frombuffer(buf, dtype=np.int16, count=vc*3, offset=p["normals"]).astype(np.float32) / 32767.0).reshape(-1, 3)
        idx = np.frombuffer(buf, dtype=np.uint32, count=ic, offset=p["indices"])
        
        target = layer_geom[layer]
        target["idx"].append(idx + target["v_offset"])
        target["pos"].append(pos)
        target["norm"].append(norm)
        target["v_offset"] += vc
        
    print("Géométries regroupées intactes. Calcul du skinning LBS anatomique (17 articulations)...")
    bin_bytes = bytearray()
    buffer_views = []
    accessors = []
    meshes = []
    nodes = []
    materials = []
    
    # 1. Inverse Bind Matrices (IBM)
    ibm = np.zeros((len(BONES), 4, 4), dtype=np.float32)
    for i in range(len(BONES)):
        ibm[i, 0, 0] = 1.0
        ibm[i, 1, 1] = 1.0
        ibm[i, 2, 2] = 1.0
        ibm[i, 3, 3] = 1.0
        wx, wy, wz = bone_A[i]
        ibm[i, 3, 0] = -wx
        ibm[i, 3, 1] = -wy
        ibm[i, 3, 2] = -wz
        
    ibm_raw = ibm.tobytes()
    ibm_offset = len(bin_bytes)
    bin_bytes.extend(ibm_raw)
    while len(bin_bytes) % 4 != 0: bin_bytes.append(0)
    
    ibm_view = len(buffer_views)
    buffer_views.append({"buffer": 0, "byteOffset": ibm_offset, "byteLength": len(ibm_raw)})
    ibm_acc = len(accessors)
    accessors.append({
        "bufferView": ibm_view, "byteOffset": 0, "componentType": 5126,
        "count": len(BONES), "type": "MAT4"
    })
    
    # Matériaux PBR des 9 couches
    for l in GROUP_ORDER:
        col = GROUP_COLORS[l]
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

    mesh_node_indices = []

    for l_idx, l in enumerate(GROUP_ORDER):
        gdata = layer_geom[l]
        if not gdata["pos"]:
            continue
        pos_all = np.vstack(gdata["pos"]).astype(np.float32)
        norm_all = np.vstack(gdata["norm"]).astype(np.float32)
        idx_all = np.concatenate(gdata["idx"]).astype(np.uint32)
        
        joints_all, weights_all = compute_weights_layer(pos_all, l)
        
        # Positions
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
        
        # Normales
        norm_offset = len(bin_bytes)
        norm_raw = norm_all.tobytes()
        bin_bytes.extend(norm_raw)
        while len(bin_bytes) % 4 != 0: bin_bytes.append(0)
        
        norm_view = len(buffer_views)
        buffer_views.append({"buffer": 0, "byteOffset": norm_offset, "byteLength": len(norm_raw), "target": 34962})
        norm_acc = len(accessors)
        accessors.append({"bufferView": norm_view, "byteOffset": 0, "componentType": 5126, "count": len(norm_all), "type": "VEC3"})
        
        # Joints (VEC4 UNSIGNED_SHORT)
        joints_offset = len(bin_bytes)
        joints_raw = joints_all.tobytes()
        bin_bytes.extend(joints_raw)
        while len(bin_bytes) % 4 != 0: bin_bytes.append(0)
        
        joints_view = len(buffer_views)
        buffer_views.append({"buffer": 0, "byteOffset": joints_offset, "byteLength": len(joints_raw), "target": 34962})
        joints_acc = len(accessors)
        accessors.append({"bufferView": joints_view, "byteOffset": 0, "componentType": 5123, "count": len(joints_all), "type": "VEC4"})
        
        # Weights (VEC4 FLOAT)
        weights_offset = len(bin_bytes)
        weights_raw = weights_all.tobytes()
        bin_bytes.extend(weights_raw)
        while len(bin_bytes) % 4 != 0: bin_bytes.append(0)
        
        weights_view = len(buffer_views)
        buffer_views.append({"buffer": 0, "byteOffset": weights_offset, "byteLength": len(weights_raw), "target": 34962})
        weights_acc = len(accessors)
        accessors.append({"bufferView": weights_view, "byteOffset": 0, "componentType": 5126, "count": len(weights_all), "type": "VEC4"})
        
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
            "name": l,
            "primitives": [{
                "attributes": {
                    "POSITION": pos_acc,
                    "NORMAL": norm_acc,
                    "JOINTS_0": joints_acc,
                    "WEIGHTS_0": weights_acc
                },
                "indices": idx_acc,
                "material": l_idx
            }]
        })
        
        node_idx = len(nodes)
        nodes.append({
            "name": l,
            "mesh": mesh_idx,
            "skin": 0
        })
        mesh_node_indices.append(node_idx)

    # 2. Nœuds des 17 articulations biomécaniques
    bone_node_indices = []
    for i, b in enumerate(BONES):
        jname = b[0]
        pname = b[3]
        if pname is None:
            local_pos = bone_A[i].tolist()
        else:
            p_idx = BONE_NAME_TO_IDX[pname]
            local_pos = (bone_A[i] - bone_A[p_idx]).tolist()
            
        j_node_idx = len(nodes)
        bone_node_indices.append(j_node_idx)
        nodes.append({
            "name": jname,
            "translation": local_pos,
            "children": []
        })
        
    for i, b in enumerate(BONES):
        pname = b[3]
        if pname:
            p_node_idx = bone_node_indices[BONE_NAME_TO_IDX[pname]]
            nodes[p_node_idx]["children"].append(bone_node_indices[i])
            
    skins = [{
        "inverseBindMatrices": ibm_acc,
        "joints": bone_node_indices,
        "skeleton": bone_node_indices[0]
    }]
    
    root_scene_nodes = [bone_node_indices[0]] + mesh_node_indices
    
    gltf = {
        "asset": {"version": "2.0", "generator": "Homo Digitalis Smooth Skinned Anatomy"},
        "scenes": [{"name": "HumanScene", "nodes": root_scene_nodes}],
        "scene": 0,
        "nodes": nodes,
        "meshes": meshes,
        "skins": skins,
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
        
    print(f"==> Modèle skinné exporté : {output_file} ({total_len / (1024*1024):.1f} MB, {len(nodes)} nœuds, 9 calques continus).")

def main():
    print("=== Homo Digitalis - Génération des Modèles Skinnés LBS ===")
    files_to_download = [
        "atlas.json",
        "atlas-female.json",
    ]
    for i in range(15):
        files_to_download.append(f"body-{i}.bin")
        files_to_download.append(f"female-{i}.bin")
        
    download_all(files_to_download)
    
    # 1. Homme (Male)
    build_skinned_glb(
        CACHE_DIR / "atlas.json",
        "body",
        classify_layer_male,
        OUTPUT_DIR / "human_male.glb"
    )
    
    # 2. Femme (Female)
    build_skinned_glb(
        CACHE_DIR / "atlas-female.json",
        "female",
        classify_layer_female,
        OUTPUT_DIR / "human_female.glb"
    )
    print("=== Modèles anatomiques skinnés prêts dans assets/models ===")

if __name__ == "__main__":
    main()
