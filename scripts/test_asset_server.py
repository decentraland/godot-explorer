#!/usr/bin/env python3
"""
Test script for the Asset Optimization Server.

Fetches a Decentraland scene and submits it for processing.

Usage:
    ./test_asset_server.py [pointer]
    ./test_asset_server.py --scene-hash <hash>

Examples:
    ./test_asset_server.py 0,0              # Process all assets from Genesis Plaza
    ./test_asset_server.py -52,-52          # Process all assets from a specific scene
    ./test_asset_server.py --scene-hash bafkreifdm7l...  # Process by scene hash directly

Output:
    Creates a ZIP file at {content_folder}/{entity_id}-mobile.zip with:
    - metadata.json (optimization metadata)
    - glbs/{hash}.scn (processed GLTFs)
    - content/{hash}.ctex (processed textures)
"""

import argparse
import json
import sys
import time
from urllib.request import Request, urlopen
from urllib.error import URLError, HTTPError


ASSET_SERVER_URL = "http://localhost:8080"
CONTENT_SERVER = "https://peer.decentraland.org/content"


def fetch_json(url: str, data: dict = None) -> dict:
    """Fetch JSON from URL, optionally POSTing data."""
    headers = {
        "Content-Type": "application/json",
        "User-Agent": "DecentralandAssetServerTest/1.0",
    }

    if data:
        req = Request(url, data=json.dumps(data).encode(), headers=headers, method="POST")
    else:
        req = Request(url, headers=headers)

    with urlopen(req, timeout=30) as response:
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
    if pack_hashes:
        data["pack_hashes"] = pack_hashes
    return fetch_json(url, data)


def get_batch_status(batch_id: str) -> dict:
    """Get batch status from asset server."""
    url = f"{ASSET_SERVER_URL}/status/{batch_id}"
    return fetch_json(url)


def main():
    global ASSET_SERVER_URL

    parser = argparse.ArgumentParser(description="Test the Asset Optimization Server")
    parser.add_argument("pointer", nargs="?", default="0,0", help="Scene pointer (default: 0,0)")
    parser.add_argument("--scene-hash", help="Process a scene by hash directly (skips pointer lookup)")
    parser.add_argument("--server", default=ASSET_SERVER_URL, help="Asset server URL")
    parser.add_argument("--content-server", default=CONTENT_SERVER, help="Content server URL")
    args = parser.parse_args()

    ASSET_SERVER_URL = args.server
    content_server = args.content_server

    print("=== Asset Server Test Script ===")
    print(f"Asset Server: {ASSET_SERVER_URL}")
    print(f"Content Server: {content_server}")
    print()

    # Check health
    print("Checking asset server health...")
    if not check_health():
        print(f"Error: Asset server is not running at {ASSET_SERVER_URL}")
        print("Start it with: cargo run -- run --asset-server")
        sys.exit(1)
    print("✓ Asset server is healthy")
    print()

    # Get scene hash
    if args.scene_hash:
        scene_hash = args.scene_hash
        entity_id = scene_hash
        print(f"Using provided scene hash: {scene_hash}")
    else:
        # Fetch scene entity
        print(f"Fetching scene entity for pointer '{args.pointer}'...")
        try:
            entity = fetch_scene_entity(args.pointer)
        except ValueError as e:
            print(f"Error: {e}")
            sys.exit(1)
        except (URLError, HTTPError) as e:
            print(f"Error fetching entity: {e}")
            sys.exit(1)

        entity_id = entity["id"]
        scene_hash = entity_id
        entity_type = entity["type"]

        # Count assets
        content = entity.get("content", [])
        gltf_count = sum(1 for item in content if item["file"].lower().endswith((".glb", ".gltf")))
        image_count = sum(1 for item in content if item["file"].lower().endswith((".png", ".jpg", ".jpeg", ".webp", ".bmp", ".tga")))

        print(f"✓ Found entity")
        print()
        print(f"Entity ID: {entity_id}")
        print(f"Entity Type: {entity_type}")
        print(f"Total content files: {len(content)}")
        print(f"  - GLTFs: {gltf_count}")
        print(f"  - Images: {image_count}")

    print()

    # Submit scene for processing
    content_base_url = f"{content_server}/contents/"
    print(f"=== Submitting scene for processing ===")
    print(f"Scene Hash: {scene_hash}")
    print(f"Content Base URL: {content_base_url}")
    print()

    try:
        response = submit_scene(scene_hash, content_base_url)
    except (URLError, HTTPError) as e:
        print(f"Error submitting scene: {e}")
        if hasattr(e, 'read'):
            error_body = e.read().decode()
            print(f"Response: {error_body}")
        sys.exit(1)

    batch_id = response["batch_id"]
    output_hash = response["output_hash"]
    total_assets = response["total_assets"]
    pack_assets = response.get("pack_assets")

    print(f"Batch ID: {batch_id}")
    print(f"Output Hash: {output_hash}")
    print(f"Total assets discovered: {total_assets}")
    if pack_assets is not None:
        print(f"Assets to pack: {pack_assets}")
    print()

    # Count jobs by type
    jobs = response.get("jobs", [])
    print(f"Created {len(jobs)} jobs")
    print()

    # Poll batch status
    print("=== Polling batch status ===")
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

        sys.stdout.write(
            f"\r[{elapsed_str}] Batch: {batch_status:<10} Progress: {progress_pct:3d}%  "
            f"Q: {queued}  D: {downloading}  P: {processing}  ✓: {completed}  ✗: {failed}  "
        )
        sys.stdout.flush()

        if batch_status in ("completed", "failed"):
            print()
            print()
            break

        time.sleep(1)

    total_elapsed = time.time() - start_time

    # Show final status
    print("=== Batch Complete ===")
    final_status = get_batch_status(batch_id)

    # Format total elapsed time
    minutes = int(total_elapsed // 60)
    seconds = int(total_elapsed % 60)
    if minutes > 0:
        time_str = f"{minutes}m {seconds}s"
    else:
        time_str = f"{seconds}s"
    print(f"Total time: {time_str}")
    print()

    if final_status["status"] == "completed":
        zip_path = final_status.get("zip_path", "N/A")
        print(f"✓ Status: {final_status['status']}")
        print(f"✓ ZIP Path: {zip_path}")
    else:
        error = final_status.get("error", "Unknown error")
        print(f"✗ Status: {final_status['status']}")
        print(f"✗ Error: {error}")

    jobs = final_status.get("jobs", [])
    completed = sum(1 for j in jobs if j["status"] == "completed")
    failed = sum(1 for j in jobs if j["status"] == "failed")

    # Count by type
    gltf_completed = sum(1 for j in jobs if j["status"] == "completed" and j["asset_type"] != "texture")
    texture_completed = sum(1 for j in jobs if j["status"] == "completed" and j["asset_type"] == "texture")

    print()
    print(f"Jobs completed: {completed}")
    print(f"  - GLTFs: {gltf_completed}")
    print(f"  - Textures: {texture_completed}")
    print(f"Jobs failed: {failed}")

    if failed > 0:
        print()
        print("Failed jobs:")
        for job in jobs:
            if job["status"] == "failed":
                print(f"  {job['job_id']}: {job.get('error', 'Unknown error')}")


if __name__ == "__main__":
    main()
