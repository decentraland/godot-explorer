#!/usr/bin/env python3
"""
Test script for the Asset Optimization Server.

Fetches a Decentraland scene and submits it for processing.

Usage:
    ./test_asset_server.py [pointer] [count]

Examples:
    ./test_asset_server.py 0,0        # Process first GLB from Genesis Plaza
    ./test_asset_server.py 0,0 all    # Process ALL GLBs from Genesis Plaza
    ./test_asset_server.py 0,0 5      # Process first 5 GLBs

Output:
    Creates a ZIP file at {content_folder}/{entity_id}-mobile.zip
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
    headers = {"Content-Type": "application/json"}

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


def submit_assets(output_hash: str, assets: list) -> dict:
    """Submit assets to asset server for processing."""
    url = f"{ASSET_SERVER_URL}/process"
    return fetch_json(url, {"output_hash": output_hash, "assets": assets})


def get_batch_status(batch_id: str) -> dict:
    """Get batch status from asset server."""
    url = f"{ASSET_SERVER_URL}/status/{batch_id}"
    return fetch_json(url)


def main():
    parser = argparse.ArgumentParser(description="Test the Asset Optimization Server")
    parser.add_argument("pointer", nargs="?", default="0,0", help="Scene pointer (default: 0,0)")
    parser.add_argument("count", nargs="?", default="1", help="Number of GLBs to process (default: 1, use 'all' for all)")
    parser.add_argument("--server", default=ASSET_SERVER_URL, help="Asset server URL")
    args = parser.parse_args()

    global ASSET_SERVER_URL
    ASSET_SERVER_URL = args.server

    print("=== Asset Server Test Script ===")
    print(f"Asset Server: {ASSET_SERVER_URL}")
    print(f"Content Server: {CONTENT_SERVER}")
    print(f"Pointer: {args.pointer}")
    print(f"Count: {args.count}")
    print()

    # Check health
    print("Checking asset server health...")
    if not check_health():
        print(f"Error: Asset server is not running at {ASSET_SERVER_URL}")
        print("Start it with: cargo run -- run --asset-server")
        sys.exit(1)
    print("✓ Asset server is healthy")
    print()

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
    entity_type = entity["type"]
    print(f"✓ Found entity")
    print()
    print(f"Entity ID: {entity_id}")
    print(f"Entity Type: {entity_type}")
    print()

    # Build content mapping
    content_mapping = {item["file"]: item["hash"] for item in entity.get("content", [])}

    # Find GLB/GLTF files
    print("Looking for GLTF/GLB files...")
    gltf_files = [
        item for item in entity.get("content", [])
        if item["file"].lower().endswith((".glb", ".gltf"))
    ]

    if not gltf_files:
        print("No GLTF/GLB files found in this scene")
        sys.exit(1)

    print(f"Found {len(gltf_files)} GLTF/GLB files")
    print()

    # Determine how many to process
    if args.count.lower() == "all":
        limit = len(gltf_files)
    else:
        limit = min(int(args.count), len(gltf_files))

    print(f"=== Processing {limit} GLTF file(s) ===")
    print()

    # Build assets list
    base_url = f"{CONTENT_SERVER}/contents/"
    assets = []
    for item in gltf_files[:limit]:
        assets.append({
            "url": base_url + item["hash"],
            "type": "scene",
            "hash": item["hash"],
            "base_url": base_url,
            "content_mapping": content_mapping,
        })

    # Show first few assets
    print(f"Submitting {len(assets)} asset(s) to asset server...")
    print()
    if len(assets) > 3:
        print("First 3 assets:")
        for asset in assets[:3]:
            print(f"  {asset['hash']}")
        print(f"  ... and {len(assets) - 3} more")
    else:
        print("Assets:")
        for asset in assets:
            print(f"  {asset['hash']}")
    print()

    # Submit to asset server
    try:
        response = submit_assets(entity_id, assets)
    except (URLError, HTTPError) as e:
        print(f"Error submitting assets: {e}")
        sys.exit(1)

    batch_id = response["batch_id"]
    output_hash = response["output_hash"]
    total = response["total"]

    print(f"Batch ID: {batch_id}")
    print(f"Output Hash: {output_hash}")
    print(f"Submitted {total} jobs")
    print()

    # Poll batch status
    print("=== Polling batch status ===")
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

        sys.stdout.write(
            f"\rBatch: {batch_status:<10} Progress: {progress_pct:3d}%  "
            f"Q: {queued}  D: {downloading}  P: {processing}  ✓: {completed}  ✗: {failed}"
        )
        sys.stdout.flush()

        if batch_status in ("completed", "failed"):
            print()
            print()
            break

        time.sleep(1)

    # Show final status
    print("=== Batch Complete ===")
    final_status = get_batch_status(batch_id)

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

    print()
    print(f"Jobs completed: {completed}")
    print(f"Jobs failed: {failed}")

    if failed > 0:
        print()
        print("Failed jobs:")
        for job in jobs:
            if job["status"] == "failed":
                print(f"  {job['job_id']}: {job.get('error', 'Unknown error')}")


if __name__ == "__main__":
    main()
