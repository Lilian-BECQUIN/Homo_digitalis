import urllib.request
import json
import subprocess
import zipfile
import io
import time
import sys

def get_github_headers():
    proc = subprocess.Popen(['git', 'credential', 'fill'], stdin=subprocess.PIPE, stdout=subprocess.PIPE, stderr=subprocess.PIPE, text=True)
    out, _ = proc.communicate('protocol=https\nhost=github.com\n\n')
    token = [l.split('=', 1)[1] for l in out.splitlines() if l.startswith('password=')][0]
    return {
        'Authorization': f'Bearer {token}',
        'Accept': 'application/vnd.github+json',
        'User-Agent': 'Homo-Downloader'
    }

def main():
    headers = get_github_headers()
    print("Checking latest workflow run on GitHub Actions...")
    runs_url = 'https://api.github.com/repos/Lilian-BECQUIN/Homo_digitalis/actions/runs?per_page=10'
    
    # Wait for the run associated with the latest push to complete
    for attempt in range(50):
        req = urllib.request.Request(runs_url, headers=headers)
        with urllib.request.urlopen(req) as resp:
            data = json.loads(resp.read().decode())
            runs = data.get('workflow_runs', [])
            web_runs = [r for r in runs if r.get('name') == 'Build Web (WASM)']
            if not web_runs:
                print("No Build Web (WASM) runs found yet...", flush=True)
            else:
                latest = web_runs[0]
                status = latest.get('status')
                conclusion = latest.get('conclusion')
                head_commit = latest.get('head_commit', {}).get('message', '').strip().split('\n')[0]
                print(f"[{attempt+1}/50] Web Run #{latest['run_number']}: '{head_commit}' -> status: {status}, conclusion: {conclusion}", flush=True)
                
                if status == 'completed':
                    if conclusion == 'success':
                        print("Build succeeded! Fetching latest artifact...", flush=True)
                        break
                    else:
                        print(f"Workflow finished with failure conclusion: {conclusion}", flush=True)
                        sys.exit(1)
        time.sleep(15)

    # Download artifact
    art_url = 'https://api.github.com/repos/Lilian-BECQUIN/Homo_digitalis/actions/artifacts?name=homo-digitalis-web&per_page=3'
    req = urllib.request.Request(art_url, headers=headers)
    with urllib.request.urlopen(req) as resp:
        data = json.loads(resp.read().decode())
        art = data['artifacts'][0]
        art_id = art['id']
        size_mb = art['size_in_bytes'] / (1024 * 1024)
        print(f"Downloading artifact {art_id} ({size_mb:.1f} MB)...")

    down_req = urllib.request.Request(f'https://api.github.com/repos/Lilian-BECQUIN/Homo_digitalis/actions/artifacts/{art_id}/zip', headers=headers)

    class NoRedirect(urllib.request.HTTPRedirectHandler):
        def redirect_request(self, req, fp, code, msg, hdrs, newurl):
            return None

    opener = urllib.request.build_opener(NoRedirect())
    signed_url = None
    try:
        opener.open(down_req)
    except urllib.error.HTTPError as e:
        if e.code in (301, 302, 303, 307):
            signed_url = e.headers.get('Location')
            print("Got signed redirect URL")

    if signed_url:
        req_blob = urllib.request.Request(signed_url, headers={'User-Agent': 'Homo-Downloader'})
        with urllib.request.urlopen(req_blob) as resp:
            content = resp.read()
            print(f"Downloaded {len(content)/(1024*1024):.1f} MB. Extracting to dist/ ...")
            with zipfile.ZipFile(io.BytesIO(content)) as z:
                z.extractall('dist')
        print("SUCCESS: Extraction complete!")
    else:
        print("Error: Could not retrieve signed URL")

if __name__ == '__main__':
    main()
