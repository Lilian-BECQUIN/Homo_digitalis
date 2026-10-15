#!/usr/bin/env python3
"""
Lanceur local pour Homo Digitalis (version WebAssembly).
Ce script démarre un serveur HTTP local avec les bons types MIME (notamment .wasm)
et ouvre automatiquement votre navigateur sur le simulateur.
Si les fichiers compilés Web ne sont pas présents ou si l'argument --update est passé,
il télécharge automatiquement le dernier bundle d'artefacts depuis GitHub Actions.
"""

import http.server
import io
import json
import os
import socketserver
import subprocess
import sys
import urllib.error
import urllib.request
import webbrowser
import zipfile

DEFAULT_PORT = 8000
REPO = "Lilian-BECQUIN/Homo_digitalis"


def get_github_token():
    """Récupère le jeton GitHub depuis git credential ou les variables d'environnement."""
    token = os.environ.get("GITHUB_TOKEN") or os.environ.get("GH_TOKEN")
    if token:
        return token

    try:
        proc = subprocess.Popen(
            ["git", "credential", "fill"],
            stdin=subprocess.PIPE,
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
            text=True,
        )
        out, _ = proc.communicate("protocol=https\nhost=github.com\n\n", timeout=5)
        for line in out.splitlines():
            if line.startswith("password="):
                return line.split("=", 1)[1]
    except Exception:
        pass
    return None


class NoRedirect(urllib.request.HTTPRedirectHandler):
    """Empêche la redirection automatique pour éviter d'envoyer l'en-tête Authorization au stockage Azure."""
    def redirect_request(self, req, fp, code, msg, headers, newurl):
        return None


def download_latest_artifact(dist_dir, token=None):
    """Télécharge et extrait le dernier artefact homo-digitalis-web depuis GitHub Actions."""
    print("Vérification des versions compilées sur GitHub Actions...")
    api_url = f"https://api.github.com/repos/{REPO}/actions/artifacts?name=homo-digitalis-web&per_page=5"

    headers = {
        "Accept": "application/vnd.github+json",
        "User-Agent": "Homo-Digitalis-Launcher",
    }
    if token:
        headers["Authorization"] = f"Bearer {token}"

    req = urllib.request.Request(api_url, headers=headers)
    try:
        with urllib.request.urlopen(req, timeout=15) as resp:
            data = json.loads(resp.read().decode("utf-8"))
    except urllib.error.HTTPError as e:
        print(f"Erreur GitHub API ({e.code}): {e.reason}")
        return False
    except Exception as e:
        print(f"Erreur de connexion : {e}")
        return False

    artifacts = [a for a in data.get("artifacts", []) if not a.get("expired", False)]
    if not artifacts:
        print("Aucun artefact disponible sur GitHub Actions.")
        return False

    art = artifacts[0]
    art_id = art["id"]
    size_mb = art.get("size_in_bytes", 0) / (1024 * 1024)
    print(f"Dernier artefact trouvé : ID {art_id} (~{size_mb:.1f} Mo)")

    download_url = f"https://api.github.com/repos/{REPO}/actions/artifacts/{art_id}/zip"
    opener = urllib.request.build_opener(NoRedirect())
    down_req = urllib.request.Request(download_url, headers=headers)

    signed_url = None
    try:
        opener.open(down_req, timeout=20)
    except urllib.error.HTTPError as e:
        if e.code in (301, 302, 303, 307):
            signed_url = e.headers.get("Location")
        else:
            print(f"Erreur lors de la requête de téléchargement : HTTP {e.code}")
            return False

    if not signed_url:
        print("URL de téléchargement direct introuvable.")
        return False

    print("Téléchargement du bundle WebAssembly & Modèles 3D en cours...")
    try:
        req_direct = urllib.request.Request(signed_url, headers={"User-Agent": "Homo-Digitalis-Launcher"})
        with urllib.request.urlopen(req_direct, timeout=120) as resp:
            zip_bytes = resp.read()
    except Exception as e:
        print(f"Échec du téléchargement du fichier : {e}")
        return False

    print(f"Archive téléchargée ({len(zip_bytes) / (1024*1024):.1f} Mo). Extraction dans {dist_dir}...")
    os.makedirs(dist_dir, exist_ok=True)
    with zipfile.ZipFile(io.BytesIO(zip_bytes)) as z:
        z.extractall(dist_dir)

    print("Extraction terminée avec succès.")
    return True


def ensure_web_bundle(script_dir):
    """Vérifie que index.html existe, sinon tente de télécharger l'artefact."""
    dist_dir = os.path.join(script_dir, "dist")
    index_html = os.path.join(dist_dir, "index.html")

    force_update = "--update" in sys.argv
    if force_update or not os.path.exists(index_html):
        if force_update:
            print("Mise à jour demandée (--update)...")
        else:
            print("Aucune version compilée trouvée localement dans ./dist/.")
        token = get_github_token()
        success = download_latest_artifact(dist_dir, token=token)
        if not success and not os.path.exists(index_html):
            print("\nERREUR : Impossible de récupérer le build compilé.")
            print("Veuillez vérifier votre connexion Internet ou créer un jeton d'accès GitHub.")
            sys.exit(1)

    return dist_dir


class WasmHandler(http.server.SimpleHTTPRequestHandler):
    def end_headers(self):
        # En-têtes pour isolation WebAssembly
        self.send_header("Cross-Origin-Opener-Policy", "same-origin")
        self.send_header("Cross-Origin-Embedder-Policy", "require-corp")
        self.send_header("Cache-Control", "no-cache")
        super().end_headers()


# Garantir les bons types MIME
WasmHandler.extensions_map.update({
    ".wasm": "application/wasm",
    ".js": "application/javascript",
    ".html": "text/html",
    ".glb": "model/gltf-binary",
    ".gltf": "model/gltf+json",
})


def find_available_port(start_port):
    """Trouve un port TCP libre à partir de start_port."""
    import socket
    for p in range(start_port, start_port + 20):
        with socket.socket(socket.AF_INET, socket.SOCK_STREAM) as s:
            if s.connect_ex(("127.0.0.1", p)) != 0:
                return p
    return start_port


def main():
    script_dir = os.path.dirname(os.path.abspath(__file__))
    web_dir = ensure_web_bundle(script_dir)

    port = find_available_port(DEFAULT_PORT)

    class CustomHandler(WasmHandler):
        def __init__(self, *args, **kwargs):
            super().__init__(*args, directory=web_dir, **kwargs)

    print("=" * 64)
    print("   HOMO DIGITALIS — Simulateur Physiologique 3D (Web)")
    print("=" * 64)
    print(f" Dossier servi : {web_dir}")
    print(f" URL           : http://localhost:{port}")
    print(" Appuyez sur Ctrl+C pour arrêter le serveur.")
    print("=" * 64)

    webbrowser.open(f"http://localhost:{port}")

    socketserver.TCPServer.allow_reuse_address = True
    try:
        with socketserver.TCPServer(("", port), CustomHandler) as httpd:
            httpd.serve_forever()
    except KeyboardInterrupt:
        print("\nArrêt du serveur. Au revoir !")
        sys.exit(0)


if __name__ == "__main__":
    main()
