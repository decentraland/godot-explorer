#!/usr/bin/env python3
"""
Test script for the Asset Optimization Server.

Fetches a Decentraland scene and processes assets.

Usage:
    ./test_asset_server.py [pointer]
    ./test_asset_server.py --scene-hash <hash>
    ./test_asset_server.py --individual    # Create separate ZIP per GLTF+dependencies

Examples:
    ./test_asset_server.py 0,0              # Process Genesis Plaza (metadata only)
    ./test_asset_server.py 0,0 --individual # One ZIP per GLTF with its textures
    ./test_asset_server.py --scene-hash bafkreifdm7l...  # Process by scene hash

Output:
    Creates ZIP files in ./output/:
    - {scene_hash}-mobile.zip (metadata + all assets, or metadata only with --individual)
    - {gltf_hash}-mobile.zip (GLTF + texture dependencies when using --individual)
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
OUTPUT_DIR = "./output"


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


def extract_metadata_from_zip(zip_path: str, scene_hash: str) -> dict:
    """Extract metadata JSON from the scene ZIP file."""
    metadata_filename = f"{scene_hash}-optimized.json"

    try:
        with zipfile.ZipFile(zip_path, 'r') as zf:
            with zf.open(metadata_filename) as f:
                return json.loads(f.read().decode())
    except (zipfile.BadZipFile, KeyError, json.JSONDecodeError) as e:
        print(f"Error extracting metadata from {zip_path}: {e}")
        return None


def process_scene_metadata_only(scene_hash: str, content_base_url: str):
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

    zip_path = final_status.get('zip_path')
    if final_status["status"] == "completed":
        print(f"Metadata ZIP: {zip_path or 'N/A'}")

    jobs = final_status.get("jobs", [])
    completed = sum(1 for j in jobs if j["status"] == "completed")
    failed = sum(1 for j in jobs if j["status"] == "failed")

    print(f"Completed: {completed}, Failed: {failed}")
    print()

    return final_status, zip_path


def process_gltf_bundles(scene_hash: str, content_base_url: str, zip_path: str):
    """Create individual ZIP files for each GLTF with its texture dependencies."""
    print(f"=== Creating Individual GLTF Bundles ===")
    print()

    # Extract metadata from the scene ZIP
    if not zip_path or not os.path.exists(zip_path):
        print(f"Error: ZIP file not found at {zip_path}")
        return

    metadata = extract_metadata_from_zip(zip_path, scene_hash)
    if not metadata:
        print("Error: Could not extract metadata from ZIP")
        return

    dependencies = metadata.get("external_scene_dependencies", {})
    if not dependencies:
        print("No GLTF dependencies found in metadata")
        return

    print(f"Found {len(dependencies)} GLTFs with external dependencies")
    print()

    start_time = time.time()
    successful = 0
    failed = 0

    for i, (gltf_hash, texture_deps) in enumerate(dependencies.items()):
        # Pack the GLTF + all its texture dependencies
        pack_hashes = [gltf_hash] + texture_deps

        print(f"[{i+1}/{len(dependencies)}] {gltf_hash[:16]}... ({len(texture_deps)} textures)")

        try:
            # Submit with the GLTF hash as output_hash so ZIP is named after the GLTF
            response = submit_scene(
                scene_hash=scene_hash,
                content_base_url=content_base_url,
                output_hash=gltf_hash,
                pack_hashes=pack_hashes,
            )

            batch_id = response["batch_id"]
            final_status = wait_for_batch(batch_id, show_progress=False)

            if final_status["status"] == "completed":
                successful += 1
                bundle_zip = final_status.get('zip_path', 'N/A')
                print(f"    ✓ Created: {os.path.basename(bundle_zip)}")
            else:
                failed += 1
                print(f"    ✗ Failed: {final_status.get('error', 'Unknown error')}")

        except (URLError, HTTPError) as e:
            failed += 1
            print(f"    ✗ Error: {e}")

    total_elapsed = time.time() - start_time
    print()
    print(f"=== Bundle Creation Complete ===")
    print(f"Total time: {format_time(total_elapsed)}")
    print(f"Successful: {successful}")
    print(f"Failed: {failed}")


def main():
    global ASSET_SERVER_URL, CONTENT_SERVER, OUTPUT_DIR

    parser = argparse.ArgumentParser(description="Test the Asset Optimization Server")
    parser.add_argument("pointer", nargs="?", default="0,0", help="Scene pointer (default: 0,0)")
    parser.add_argument("--scene-hash", help="Process a scene by hash directly")
    parser.add_argument("--individual", action="store_true", help="Create separate ZIP per GLTF with dependencies")
    parser.add_argument("--server", default=ASSET_SERVER_URL, help="Asset server URL")
    parser.add_argument("--content-server", default=CONTENT_SERVER, help="Content server URL")
    parser.add_argument("--output-dir", default=OUTPUT_DIR, help="Output directory for ZIPs")
    args = parser.parse_args()

    ASSET_SERVER_URL = args.server
    CONTENT_SERVER = args.content_server
    OUTPUT_DIR = args.output_dir
    content_base_url = f"{CONTENT_SERVER}/contents/"

    print("=== Asset Server Test Script ===")
    print(f"Asset Server: {ASSET_SERVER_URL}")
    print(f"Content Server: {CONTENT_SERVER}")
    print(f"Output Directory: {OUTPUT_DIR}")
    print(f"Mode: {'Individual GLTF bundles' if args.individual else 'Scene metadata only'}")
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

    # Process scene (metadata only first)
    final_status, zip_path = process_scene_metadata_only(scene_hash, content_base_url)

    # Create individual GLTF bundles if requested
    if args.individual:
        if final_status["status"] == "completed" and zip_path:
            process_gltf_bundles(scene_hash, content_base_url, zip_path)
        else:
            print("Cannot create individual bundles: scene processing failed or no ZIP created")


if __name__ == "__main__":
    main()
