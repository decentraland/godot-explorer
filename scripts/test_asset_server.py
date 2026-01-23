#!/usr/bin/env python3
"""
Test script for the Asset Optimization Server.

Fetches a Decentraland scene and processes assets individually.

Usage:
    ./test_asset_server.py [pointer]
    ./test_asset_server.py --scene-hash <hash>
    ./test_asset_server.py --individual    # Process each asset into separate ZIP

Examples:
    ./test_asset_server.py 0,0              # Process Genesis Plaza (metadata only)
    ./test_asset_server.py 0,0 --individual # Process each asset separately
    ./test_asset_server.py --scene-hash bafkreifdm7l...  # Process by scene hash

Output:
    Creates ZIP files in ./output/:
    - {scene_hash}-mobile.zip (with metadata.json only when using --individual)
    - {asset_hash}-mobile.zip (individual asset ZIPs when using --individual)
"""

import argparse
import json
import sys
import time
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


def submit_scene(scene_hash: str, content_base_url: str, output_hash: str = None, pack_hashes: list = None) -> dict:
    """Submit scene to asset server for processing."""
    url = f"{ASSET_SERVER_URL}/process-scene"
    data = {
        "scene_hash": scene_hash,
        "content_base_url": content_base_url,
    }
    if output_hash:
        data["output_hash"] = output_hash
    if pack_hashes is not None:  # Allow empty list
        data["pack_hashes"] = pack_hashes
    return fetch_json(url, data)


def submit_asset(asset_hash: str, asset_url: str, asset_type: str, base_url: str, content_mapping: dict) -> dict:
    """Submit a single asset to asset server for processing."""
    url = f"{ASSET_SERVER_URL}/process"
    data = {
        "assets": [{
            "hash": asset_hash,
            "url": asset_url,
            "asset_type": asset_type,
            "base_url": base_url,
            "content_mapping": content_mapping,
        }],
        "output_hash": asset_hash,
    }
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

        elapsed = time.time() - start_time
        elapsed_str = f"{int(elapsed // 60):02d}:{int(elapsed % 60):02d}"

        if show_progress:
            sys.stdout.write(
                f"\r[{elapsed_str}] Batch: {batch_status:<10} Progress: {progress_pct:3d}%  "
                f"Q: {queued}  D: {downloading}  P: {processing}  ✓: {completed}  ✗: {failed}  "
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


def process_scene_metadata_only(scene_hash: str, content_base_url: str, entity: dict = None):
    """Process scene to get metadata only (no assets packed)."""
    print(f"=== Processing Scene (Metadata Only) ===")
    print(f"Scene Hash: {scene_hash}")
    print()

    # Submit with empty pack_hashes to get metadata only
    try:
        response = submit_scene(scene_hash, content_base_url, pack_hashes=[])
    except (URLError, HTTPError) as e:
        print(f"Error submitting scene: {e}")
        if hasattr(e, 'read'):
            print(f"Response: {e.read().decode()}")
        sys.exit(1)

    batch_id = response["batch_id"]
    total_assets = response["total_assets"]

    print(f"Batch ID: {batch_id}")
    print(f"Total assets discovered: {total_assets}")
    print()

    # Wait for completion
    print("=== Processing Assets ===")
    start_time = time.time()
    final_status = wait_for_batch(batch_id)
    total_elapsed = time.time() - start_time

    print()
    print(f"=== Scene Processing Complete ===")
    print(f"Total time: {format_time(total_elapsed)}")
    print(f"Status: {final_status['status']}")

    if final_status["status"] == "completed":
        print(f"Metadata ZIP: {final_status.get('zip_path', 'N/A')}")

    jobs = final_status.get("jobs", [])
    completed = sum(1 for j in jobs if j["status"] == "completed")
    failed = sum(1 for j in jobs if j["status"] == "failed")

    print(f"Completed: {completed}, Failed: {failed}")
    print()

    return final_status, jobs


def process_assets_individually(jobs: list, content_base_url: str, content_mapping: dict):
    """Process each asset individually to create separate ZIP files."""
    print(f"=== Processing Assets Individually ===")
    print(f"Processing {len(jobs)} completed assets...")
    print()

    start_time = time.time()
    successful = 0
    failed = 0

    for i, job in enumerate(jobs):
        if job["status"] != "completed":
            continue

        asset_hash = job["hash"]
        asset_type = job["asset_type"]
        asset_url = f"{content_base_url}{asset_hash}"

        # Progress indicator
        sys.stdout.write(f"\r[{i+1}/{len(jobs)}] Processing {asset_hash[:16]}... ")
        sys.stdout.flush()

        try:
            response = submit_asset(
                asset_hash=asset_hash,
                asset_url=asset_url,
                asset_type=asset_type,
                base_url=content_base_url,
                content_mapping=content_mapping,
            )

            batch_id = response["batch_id"]
            final_status = wait_for_batch(batch_id, show_progress=False)

            if final_status["status"] == "completed":
                successful += 1
            else:
                failed += 1
                print(f"\n  Failed: {final_status.get('error', 'Unknown error')}")

        except (URLError, HTTPError) as e:
            failed += 1
            print(f"\n  Error: {e}")

    total_elapsed = time.time() - start_time
    print()
    print()
    print(f"=== Individual Processing Complete ===")
    print(f"Total time: {format_time(total_elapsed)}")
    print(f"Successful: {successful}")
    print(f"Failed: {failed}")


def main():
    global ASSET_SERVER_URL, CONTENT_SERVER

    parser = argparse.ArgumentParser(description="Test the Asset Optimization Server")
    parser.add_argument("pointer", nargs="?", default="0,0", help="Scene pointer (default: 0,0)")
    parser.add_argument("--scene-hash", help="Process a scene by hash directly")
    parser.add_argument("--individual", action="store_true", help="Process each asset into separate ZIP files")
    parser.add_argument("--server", default=ASSET_SERVER_URL, help="Asset server URL")
    parser.add_argument("--content-server", default=CONTENT_SERVER, help="Content server URL")
    args = parser.parse_args()

    ASSET_SERVER_URL = args.server
    CONTENT_SERVER = args.content_server
    content_base_url = f"{CONTENT_SERVER}/contents/"

    print("=== Asset Server Test Script ===")
    print(f"Asset Server: {ASSET_SERVER_URL}")
    print(f"Content Server: {CONTENT_SERVER}")
    print(f"Mode: {'Individual ZIPs' if args.individual else 'Scene metadata only'}")
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
    entity = None
    content_mapping = {}

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

        # Build content mapping
        content_mapping = {item["file"]: item["hash"] for item in content}

        gltf_count = sum(1 for item in content if item["file"].lower().endswith((".glb", ".gltf")))
        image_count = sum(1 for item in content if item["file"].lower().endswith((".png", ".jpg", ".jpeg", ".webp")))

        print(f"✓ Found entity: {scene_hash}")
        print(f"  GLTFs: {gltf_count}, Images: {image_count}")

    print()

    # Process scene (metadata only)
    final_status, jobs = process_scene_metadata_only(scene_hash, content_base_url, entity)

    # Process individually if requested
    if args.individual:
        completed_jobs = [j for j in jobs if j["status"] == "completed"]
        if completed_jobs:
            process_assets_individually(completed_jobs, content_base_url, content_mapping)
        else:
            print("No completed jobs to process individually.")


if __name__ == "__main__":
    main()
