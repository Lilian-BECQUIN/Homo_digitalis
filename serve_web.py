#!/usr/bin/env python3
"""
Lanceur local pour Homo Digitalis (version WebAssembly).
Ce script démarre un serveur HTTP local avec les bons types MIME (notamment .wasm)
et ouvre automatiquement votre navigateur sur le simulateur.
"""

import http.server
import socketserver
import webbrowser
import os
import sys

PORT = 8000

# Recherche du dossier contenant index.html et les fichiers .wasm
script_dir = os.path.dirname(os.path.abspath(__file__))
candidates = [
    os.path.join(script_dir, "dist"),
    os.path.join(script_dir, "crates", "homo-app", "dist"),
    script_dir,
]

web_dir = None
for c in candidates:
    if os.path.exists(os.path.join(c, "index.html")):
        web_dir = c
        break

if not web_dir:
    web_dir = script_dir

class WasmHandler(http.server.SimpleHTTPRequestHandler):
    def __init__(self, *args, **kwargs):
        super().__init__(*args, directory=web_dir, **kwargs)

    def end_headers(self):
        # En-têtes recommandés pour l'isolation et les performances WebAssembly / WebGPU
        self.send_header("Cross-Origin-Opener-Policy", "same-origin")
        self.send_header("Cross-Origin-Embedder-Policy", "require-corp")
        self.send_header("Cache-Control", "no-cache")
        super().end_headers()

# Garantir le bon type MIME pour le WebAssembly
WasmHandler.extensions_map.update({
    ".wasm": "application/wasm",
    ".js": "application/javascript",
    ".html": "text/html",
    ".glb": "model/gltf-binary",
    ".gltf": "model/gltf+json",
})

print("=" * 60)
print("  HOMO DIGITALIS — Simulateur Physiologique 3D (Web)")
print("=" * 60)
print(f"Dossier servi : {web_dir}")
print(f"URL           : http://localhost:{PORT}")
print("Appuyez sur Ctrl+C pour arrêter le serveur.")
print("=" * 60)

# Ouvre le navigateur par défaut
webbrowser.open(f"http://localhost:{PORT}")

socketserver.TCPServer.allow_reuse_address = True
try:
    with socketserver.TCPServer(("", PORT), WasmHandler) as httpd:
        httpd.serve_forever()
except KeyboardInterrupt:
    print("\nArrêt du serveur. Au revoir !")
    sys.exit(0)
