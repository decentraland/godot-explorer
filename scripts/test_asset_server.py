#!/usr/bin/env python3
"""
Test script for the Asset Optimization Server.

Fetches a Decentraland scene and processes assets.

Usage:
    ./test_asset_server.py [pointer]
    ./test_asset_server.py --scene-hash <hash>
    ./test_asset_server.py --individual    # Create separate ZIP per asset
    ./test_asset_server.py --port 9000     # Use custom port

Examples:
    ./test_asset_server.py 0,0              # Process Genesis Plaza (metadata only)
    ./test_asset_server.py 0,0 --individual # One ZIP per GLTF (with deps) + one ZIP per texture
    ./test_asset_server.py --scene-hash bafkreifdm7l...  # Process by scene hash
    ./test_asset_server.py --port 9000 0,0  # Use server on port 9000

Output:
    Creates ZIP files in output folder:
    - {scene_hash}-mobile.zip (metadata only when using --individual)
    - {gltf_hash}-mobile.zip (GLTF + texture dependencies)
    - {texture_hash}-mobile.zip (individual textures)
"""

import argparse
import json
import os
import sys
import time
import zipfile
from concurrent.futures import ThreadPoolExecutor, as_completed
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


def process_scene_full(scene_hash: str, content_base_url: str):
    """Process scene with ALL assets + metadata in one ZIP."""
    print(f"=== Processing Scene (Full Pack) ===")
    print(f"Scene Hash: {scene_hash}")
    print()

    # Submit WITHOUT pack_hashes to get full scene pack
    try:
        response = submit_scene(scene_hash, content_base_url)
    except (URLError, HTTPError) as e:
        print(f"Error submitting scene: {e}")
        if hasattr(e, 'read'):
            print(f"Response: {e.read().decode()}")
        sys.exit(1)

    batch_id = response["batch_id"]
    total_assets = response["total_assets"]

    print(f"Batch ID: {batch_id}")
    print(f"Total assets to process: {total_assets}")
    print()

    # Wait for batch completion
    final_status = wait_for_batch(batch_id)

    print()
    print(f"=== Result ===")
    print(f"Status: {final_status['status']}")

    zip_path = final_status.get("zip_path")
    if zip_path:
        print(f"ZIP: {zip_path}")
        # Check ZIP contents
        if os.path.exists(zip_path):
            with zipfile.ZipFile(zip_path, 'r') as zf:
                files = zf.namelist()
                print(f"ZIP contains {len(files)} files")
                # Count by type
                scn_files = [f for f in files if f.endswith('.scn')]
                res_files = [f for f in files if f.endswith('.res')]
                json_files = [f for f in files if f.endswith('.json')]
                print(f"  - Scenes (.scn): {len(scn_files)}")
                print(f"  - Resources (.res): {len(res_files)}")
                print(f"  - Metadata (.json): {len(json_files)}")
    else:
        print("No ZIP file created")

    return final_status, zip_path


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


def create_individual_bundles(scene_hash: str, content_base_url: str, zip_path: str):
    """Create individual ZIP files for each GLTF and each texture (one asset per ZIP)."""
    print(f"=== Creating Individual Asset Bundles ===")
    print()

    # Extract metadata from the scene ZIP
    if not zip_path or not os.path.exists(zip_path):
        print(f"Error: ZIP file not found at {zip_path}")
        return

    metadata = extract_metadata_from_zip(zip_path, scene_hash)
    if not metadata:
        print("Error: Could not extract metadata from ZIP")
        return

    dependencies = metadata.get("externalSceneDependencies", {})
    optimized_content = set(metadata.get("optimizedContent", []))

    # Get all GLTF hashes (keys in dependencies)
    gltf_hashes = set(dependencies.keys())

    # All textures = optimized content - GLTFs
    texture_hashes = optimized_content - gltf_hashes

    all_hashes = list(gltf_hashes) + list(texture_hashes)
    print(f"Total assets to bundle: {len(all_hashes)} ({len(gltf_hashes)} GLTFs, {len(texture_hashes)} textures)")
    print()

    start_time = time.time()

    # Submit ALL requests concurrently using thread pool
    print(f"--- Submitting {len(all_hashes)} bundle requests (concurrent) ---")
    pending_batches = []  # List of (hash, batch_id)
    submit_failed = 0
    submitted = 0

    def submit_one(asset_hash):
        """Submit a single bundle request."""
        response = submit_scene(
            scene_hash=scene_hash,
            content_base_url=content_base_url,
            output_hash=asset_hash,
            pack_hashes=[asset_hash],
        )
        return (asset_hash, response["batch_id"])

    with ThreadPoolExecutor(max_workers=32) as executor:
        futures = {executor.submit(submit_one, h): h for h in all_hashes}

        for future in as_completed(futures):
            asset_hash = futures[future]
            try:
                result = future.result()
                pending_batches.append(result)
                submitted += 1
            except Exception as e:
                submit_failed += 1

            sys.stdout.write(f"\r[{submitted + submit_failed}/{len(all_hashes)}] Submitted: {submitted} ✓, {submit_failed} ✗ ")
            sys.stdout.flush()

    print(f"\r[{len(all_hashes)}/{len(all_hashes)}] Submitted: {len(pending_batches)} ✓, {submit_failed} ✗" + " " * 20)
    print()

    # Wait for ALL batches to complete
    print(f"--- Waiting for {len(pending_batches)} batches to complete ---")
    successful = 0
    failed = 0

    while pending_batches:
        still_pending = []
        for asset_hash, batch_id in pending_batches:
            try:
                status = get_batch_status(batch_id)
                batch_status = status["status"]

                if batch_status == "completed":
                    successful += 1
                elif batch_status == "failed":
                    failed += 1
                else:
                    still_pending.append((asset_hash, batch_id))
            except (URLError, HTTPError):
                still_pending.append((asset_hash, batch_id))

        pending_batches = still_pending

        completed = successful + failed
        total = completed + len(pending_batches)
        elapsed = time.time() - start_time
        elapsed_str = f"{int(elapsed // 60):02d}:{int(elapsed % 60):02d}"

        sys.stdout.write(
            f"\r[{elapsed_str}] Completed: {completed}/{total}  ✓: {successful}  ✗: {failed}  Pending: {len(pending_batches)}  "
        )
        sys.stdout.flush()

        if pending_batches:
            time.sleep(0.5)

    print()
    total_elapsed = time.time() - start_time
    print()
    print(f"=== Bundle Creation Complete ===")
    print(f"Total time: {format_time(total_elapsed)}")
    print(f"Successful: {successful}")
    print(f"Failed: {failed + submit_failed}")
    print(f"Total ZIPs created: {successful}")


def main():
    global ASSET_SERVER_URL, CONTENT_SERVER

    parser = argparse.ArgumentParser(description="Test the Asset Optimization Server")
    parser.add_argument("pointer", nargs="?", default="0,0", help="Scene pointer (default: 0,0)")
    parser.add_argument("--scene-hash", help="Process a scene by hash directly")
    parser.add_argument("--full", action="store_true", help="Create FULL scene pack (all assets + metadata in one ZIP)")
    parser.add_argument("--individual", action="store_true", help="Create separate ZIP per asset (after metadata-only pack)")
    parser.add_argument("--server", default=ASSET_SERVER_URL, help="Asset server URL")
    parser.add_argument("--port", type=int, help="Asset server port (shorthand for --server http://localhost:PORT)")
    parser.add_argument("--content-server", default=CONTENT_SERVER, help="Content server URL")
    args = parser.parse_args()

    ASSET_SERVER_URL = f"http://localhost:{args.port}" if args.port else args.server
    CONTENT_SERVER = args.content_server
    content_base_url = f"{CONTENT_SERVER}/contents/"

    if args.full:
        mode_str = "Full scene pack (all assets + metadata)"
    elif args.individual:
        mode_str = "Individual asset bundles"
    else:
        mode_str = "Scene metadata only"

    print("=== Asset Server Test Script ===")
    print(f"Asset Server: {ASSET_SERVER_URL}")
    print(f"Content Server: {CONTENT_SERVER}")
    print(f"Mode: {mode_str}")
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

    if args.full:
        # Full scene pack: all assets + metadata in one ZIP
        final_status, zip_path = process_scene_full(scene_hash, content_base_url)
    else:
        # Process scene (metadata only first)
        final_status, zip_path = process_scene_metadata_only(scene_hash, content_base_url)

        # Create individual bundles if requested
        if args.individual:
            if final_status["status"] == "completed" and zip_path:
                create_individual_bundles(scene_hash, content_base_url, zip_path)
            else:
                print("Cannot create individual bundles: scene processing failed or no ZIP created")


if __name__ == "__main__":
    main()
