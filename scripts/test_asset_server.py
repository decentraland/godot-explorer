#!/usr/bin/env python3
"""
Test script for the Asset Optimization Server.

Fetches a Decentraland scene and processes assets. The server creates one ZIP
per asset plus a main metadata ZIP.

Usage:
    ./test_asset_server.py [pointer]
    ./test_asset_server.py --scene-hash <hash>
    ./test_asset_server.py --preloaded-hashes hash1,hash2  # Include assets in main ZIP
    ./test_asset_server.py --port 9000                     # Use custom port

Examples:
    ./test_asset_server.py 0,0
    ./test_asset_server.py --scene-hash bafkreifdm7l...
    ./test_asset_server.py --preloaded-hashes abc123,def456 0,0
    ./test_asset_server.py --port 9000 0,0

Output:
    Creates ZIP files in output folder:
    - {hash}-mobile.zip per asset (each with single .scn or .res)
    - {output_hash}-mobile.zip (metadata JSON + optional preloaded assets)
"""

import argparse
import json
import os
import sys
import time
import zipfile
from urllib.request import Request, urlopen
from urllib.error import URLError, HTTPError


ASSET_SERVER_URL = "http://localhost:8080"
CONTENT_SERVER = "https://peer.decentraland.org/content"


def fetch_json(url: str, data: dict = None, timeout: int = 30) -> dict:
    """Fetch JSON from URL, optionally POSTing data."""
    headers = {
        "Content-Type": "application/json",
        "User-Agent": "DecentralandAssetServerTest/1.0",
    }

    if data:
        req = Request(url, data=json.dumps(data).encode(), headers=headers, method="POST")
    else:
        req = Request(url, headers=headers)

    with urlopen(req, timeout=timeout) as response:
        return json.loads(response.read().decode())


def check_health() -> bool:
    """Check if asset server is healthy."""
    try:
        result = fetch_json(f"{ASSET_SERVER_URL}/health")
        return result.get("status") == "ok"
    except (URLError, HTTPError):
        return False


def fetch_scene_entity(pointer: str) -> dict:
    """Fetch scene entity from content server."""
    url = f"{CONTENT_SERVER}/entities/active"
    result = fetch_json(url, {"pointers": [pointer]})

    if not result:
        raise ValueError(f"No scene found at pointer '{pointer}'")

    return result[0]


def submit_scene(scene_hash: str, content_base_url: str, output_hash: str = None, preloaded_hashes: list = None) -> dict:
    """Submit scene to asset server for processing."""
    url = f"{ASSET_SERVER_URL}/process-scene"
    data = {
        "scene_hash": scene_hash,
        "content_base_url": content_base_url,
    }
    if output_hash:
        data["output_hash"] = output_hash
    if preloaded_hashes is not None:
        data["preloaded_hashes"] = preloaded_hashes
    return fetch_json(url, data)


def get_batch_status(batch_id: str) -> dict:
    """Get batch status from asset server."""
    url = f"{ASSET_SERVER_URL}/status/{batch_id}"
    return fetch_json(url)


def wait_for_batch(batch_id: str, show_progress: bool = True) -> dict:
    """Wait for a batch to complete, showing progress."""
    start_time = time.time()
    while True:
        try:
            status = get_batch_status(batch_id)
        except (URLError, HTTPError) as e:
            print(f"\nError getting status: {e}")
            sys.exit(1)

        batch_status = status["status"]
        progress = status.get("progress", 0)
        progress_pct = int(progress * 100)

        jobs = status.get("jobs", [])
        queued = sum(1 for j in jobs if j["status"] == "queued")
        downloading = sum(1 for j in jobs if j["status"] == "downloading")
        processing = sum(1 for j in jobs if j["status"] == "processing")
        completed = sum(1 for j in jobs if j["status"] == "completed")
        failed = sum(1 for j in jobs if j["status"] == "failed")

        individual_zips = status.get("individual_zips", [])

        elapsed = time.time() - start_time
        elapsed_str = f"{int(elapsed // 60):02d}:{int(elapsed % 60):02d}"

        if show_progress:
            sys.stdout.write(
                f"\r[{elapsed_str}] Batch: {batch_status:<10} Progress: {progress_pct:3d}%  "
                f"Q: {queued}  D: {downloading}  P: {processing}  ✓: {completed}  ✗: {failed}  "
                f"ZIPs: {len(individual_zips)}  "
            )
            sys.stdout.flush()

        if batch_status in ("completed", "failed"):
            if show_progress:
                print()
            return status

        time.sleep(1)


def format_time(seconds: float) -> str:
    """Format seconds as human-readable time."""
    minutes = int(seconds // 60)
    secs = int(seconds % 60)
    if minutes > 0:
        return f"{minutes}m {secs}s"
    return f"{secs}s"


def process_scene(scene_hash: str, content_base_url: str, preloaded_hashes: list = None):
    """Process scene: creates individual ZIPs per asset + main metadata ZIP."""
    print(f"=== Processing Scene ===")
    print(f"Scene Hash: {scene_hash}")
    if preloaded_hashes:
        print(f"Preloaded hashes: {len(preloaded_hashes)}")
    print()

    try:
        response = submit_scene(scene_hash, content_base_url, preloaded_hashes=preloaded_hashes)
    except (URLError, HTTPError) as e:
        print(f"Error submitting scene: {e}")
        if hasattr(e, 'read'):
            print(f"Response: {e.read().decode()}")
        sys.exit(1)

    batch_id = response["batch_id"]
    total_assets = response["total_assets"]
    preloaded_count = response.get("preloaded_assets")

    print(f"Batch ID: {batch_id}")
    print(f"Total assets to process: {total_assets}")
    if preloaded_count is not None:
        print(f"Assets to preload in main ZIP: {preloaded_count}")
    print()

    # Wait for batch completion
    print("=== Processing Assets ===")
    start_time = time.time()
    final_status = wait_for_batch(batch_id)
    total_elapsed = time.time() - start_time

    print()
    print(f"=== Result ===")
    print(f"Status: {final_status['status']}")
    print(f"Total time: {format_time(total_elapsed)}")

    # Show individual ZIPs
    individual_zips = final_status.get("individual_zips", [])
    if individual_zips:
        print(f"\nIndividual asset ZIPs: {len(individual_zips)}")
        for info in individual_zips[:5]:
            print(f"  {info['hash']}: {info['zip_path']}")
        if len(individual_zips) > 5:
            print(f"  ... and {len(individual_zips) - 5} more")

    # Show main ZIP
    zip_path = final_status.get("zip_path")
    if zip_path:
        print(f"\nMain ZIP: {zip_path}")
        if os.path.exists(zip_path):
            with zipfile.ZipFile(zip_path, 'r') as zf:
                files = zf.namelist()
                print(f"Main ZIP contains {len(files)} files")
                scn_files = [f for f in files if f.endswith('.scn')]
                res_files = [f for f in files if f.endswith('.res')]
                json_files = [f for f in files if f.endswith('.json')]
                print(f"  - Scenes (.scn): {len(scn_files)}")
                print(f"  - Resources (.res): {len(res_files)}")
                print(f"  - Metadata (.json): {len(json_files)}")
    else:
        print("No main ZIP file created")

    jobs = final_status.get("jobs", [])
    completed = sum(1 for j in jobs if j["status"] == "completed")
    failed = sum(1 for j in jobs if j["status"] == "failed")
    print(f"\nJobs completed: {completed}, failed: {failed}")

    return final_status, zip_path


def main():
    global ASSET_SERVER_URL, CONTENT_SERVER

    parser = argparse.ArgumentParser(description="Test the Asset Optimization Server")
    parser.add_argument("pointer", nargs="?", default="0,0", help="Scene pointer (default: 0,0)")
    parser.add_argument("--scene-hash", help="Process a scene by hash directly")
    parser.add_argument("--preloaded-hashes", help="Comma-separated list of hashes to preload in main ZIP")
    parser.add_argument("--server", default=ASSET_SERVER_URL, help="Asset server URL")
    parser.add_argument("--port", type=int, help="Asset server port (shorthand for --server http://localhost:PORT)")
    parser.add_argument("--content-server", default=CONTENT_SERVER, help="Content server URL")
    args = parser.parse_args()

    ASSET_SERVER_URL = f"http://localhost:{args.port}" if args.port else args.server
    CONTENT_SERVER = args.content_server
    content_base_url = f"{CONTENT_SERVER}/contents/"

    preloaded_hashes = None
    if args.preloaded_hashes:
        preloaded_hashes = [h.strip() for h in args.preloaded_hashes.split(",") if h.strip()]

    print("=== Asset Server Test Script ===")
    print(f"Asset Server: {ASSET_SERVER_URL}")
    print(f"Content Server: {CONTENT_SERVER}")
    if preloaded_hashes:
        print(f"Preloaded hashes: {preloaded_hashes}")
    print()

    # Check health
    print("Checking asset server health...")
    if not check_health():
        print(f"Error: Asset server is not running at {ASSET_SERVER_URL}")
        print("Start it with: cargo run -- run --asset-server")
        sys.exit(1)
    print("✓ Asset server is healthy")
    print()

    # Get scene info
    if args.scene_hash:
        scene_hash = args.scene_hash
        print(f"Using provided scene hash: {scene_hash}")
    else:
        print(f"Fetching scene entity for pointer '{args.pointer}'...")
        try:
            entity = fetch_scene_entity(args.pointer)
        except ValueError as e:
            print(f"Error: {e}")
            sys.exit(1)
        except (URLError, HTTPError) as e:
            print(f"Error fetching entity: {e}")
            sys.exit(1)

        scene_hash = entity["id"]
        content = entity.get("content", [])

        gltf_count = sum(1 for item in content if item["file"].lower().endswith((".glb", ".gltf")))
        image_count = sum(1 for item in content if item["file"].lower().endswith((".png", ".jpg", ".jpeg", ".webp")))

        print(f"✓ Found entity: {scene_hash}")
        print(f"  GLTFs: {gltf_count}, Images: {image_count}")

    print()

    process_scene(scene_hash, content_base_url, preloaded_hashes=preloaded_hashes)


if __name__ == "__main__":
    main()
