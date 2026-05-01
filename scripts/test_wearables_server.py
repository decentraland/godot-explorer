#!/usr/bin/env python3
"""
Test script for optimizing Wearables and Emotes via the Asset Optimization Server.

Fetches a user's profile and processes all their wearables and emotes.

Usage:
    ./test_wearables_server.py <eth_address>
    ./test_wearables_server.py --port 9000 <eth_address>

Examples:
    ./test_wearables_server.py 0x481bed8645804714Efd1dE3f25467f78E7Ba07d6
    ./test_wearables_server.py --port 9000 0x481bed8645804714Efd1dE3f25467f78E7Ba07d6

Output:
    Creates ZIP files in output folder for each wearable and emote.
"""

import argparse
import json
import os
import sys
import time
from concurrent.futures import ThreadPoolExecutor, as_completed
from urllib.request import Request, urlopen
from urllib.error import URLError, HTTPError


ASSET_SERVER_URL = "http://localhost:8080"
CONTENT_SERVER = "https://peer.decentraland.org/content"
LAMBDAS_SERVER = "https://peer.decentraland.org/lambdas"


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


def fetch_profile(eth_address: str) -> dict:
    """Fetch user profile from lambdas server."""
    url = f"{LAMBDAS_SERVER}/profiles/{eth_address}"
    result = fetch_json(url)

    if not result.get("avatars"):
        raise ValueError(f"No profile found for address '{eth_address}'")

    return result


def fetch_wearable_entities(urns: list) -> list:
    """Fetch wearable/emote entities from content server."""
    if not urns:
        return []

    url = f"{CONTENT_SERVER}/entities/active"
    try:
        result = fetch_json(url, {"pointers": urns}, timeout=60)
        return result if result else []
    except (URLError, HTTPError) as e:
        print(f"Warning: Failed to fetch entities: {e}")
        return []


def urn_to_pointer(urn: str) -> str:
    """Convert a full URN (with token ID) to an entity pointer (without token ID).

    Full URN: urn:decentraland:matic:collections-v2:CONTRACT:ITEM_ID:TOKEN_ID
    Pointer:  urn:decentraland:matic:collections-v2:CONTRACT:ITEM_ID
    """
    parts = urn.split(":")
    # collections-v2 URNs have format: urn:decentraland:NETWORK:collections-v2:CONTRACT:ITEM_ID[:TOKEN_ID]
    # We need to keep only up to ITEM_ID (6 parts)
    if "collections-v2" in urn and len(parts) > 6:
        return ":".join(parts[:6])
    return urn


def extract_wearables_and_emotes(profile: dict) -> tuple:
    """Extract wearable and emote URNs from profile."""
    avatars = profile.get("avatars", [])
    if not avatars:
        return [], []

    avatar = avatars[0].get("avatar", {})

    # Extract wearables (list of URN strings)
    wearables_raw = avatar.get("wearables", [])

    # Filter out base-avatars (they don't have GLTFs to process)
    # Convert to pointers (strip token ID if present)
    wearables = []
    seen_wearables = set()
    for w in wearables_raw:
        if "base-avatars" not in w:
            pointer = urn_to_pointer(w)
            if pointer not in seen_wearables:
                wearables.append(pointer)
                seen_wearables.add(pointer)

    # Extract emotes (list of {slot, urn} objects)
    emotes_raw = avatar.get("emotes", [])
    emotes = []
    seen_emotes = set()
    for emote in emotes_raw:
        urn = emote.get("urn", "")
        # Skip simple emotes like "kiss", "wave" - they're built-in
        if urn and ":" in urn and "base-emotes" not in urn:
            pointer = urn_to_pointer(urn)
            if pointer not in seen_emotes:
                emotes.append(pointer)
                seen_emotes.add(pointer)

    return wearables, emotes


def get_batch_status(batch_id: str) -> dict:
    """Get batch status from asset server."""
    url = f"{ASSET_SERVER_URL}/status/{batch_id}"
    return fetch_json(url)


def submit_asset(hash: str, asset_type: str, content_mapping: dict, base_url: str) -> dict:
    """Submit a single asset to the asset server."""
    url = f"{ASSET_SERVER_URL}/process"
    data = {
        "output_hash": hash,
        "assets": [{
            "url": f"{base_url}{hash}",
            "type": asset_type,
            "hash": hash,
            "base_url": base_url,
            "content_mapping": content_mapping,
        }]
    }
    return fetch_json(url, data)


def wait_for_batch(batch_id: str, show_progress: bool = True) -> dict:
    """Wait for a batch to complete."""
    start_time = time.time()
    while True:
        try:
            status = get_batch_status(batch_id)
        except (URLError, HTTPError) as e:
            print(f"\nError getting status: {e}")
            return {"status": "failed", "error": str(e)}

        batch_status = status["status"]
        progress = status.get("progress", 0)

        if batch_status in ("completed", "failed"):
            return status

        if show_progress:
            elapsed = time.time() - start_time
            sys.stdout.write(f"\r  Progress: {int(progress * 100):3d}% ({elapsed:.1f}s)")
            sys.stdout.flush()

        time.sleep(0.5)


def format_time(seconds: float) -> str:
    """Format seconds as human-readable time."""
    minutes = int(seconds // 60)
    secs = int(seconds % 60)
    if minutes > 0:
        return f"{minutes}m {secs}s"
    return f"{secs}s"


def process_entities(entities: list, asset_type: str, base_url: str) -> tuple:
    """Process a list of entities and return (successful, failed) counts."""
    if not entities:
        return 0, 0

    # Count total GLTFs across all entities
    total_gltfs = sum(
        len([c for c in e.get("content", []) if c["file"].lower().endswith((".glb", ".gltf"))])
        for e in entities
    )

    print(f"\n=== Processing {len(entities)} {asset_type} entities ({total_gltfs} GLTFs) ===")

    successful = 0
    failed = 0
    processed = 0

    for i, entity in enumerate(entities, 1):
        entity_id = entity.get("id", "unknown")
        pointers = entity.get("pointers", [])
        content = entity.get("content", [])

        # Find all GLTF files (male/female variants)
        gltf_files = [c for c in content if c["file"].lower().endswith((".glb", ".gltf"))]
        if not gltf_files:
            print(f"  [{i}/{len(entities)}] {pointers[0] if pointers else entity_id}: No GLTF found, skipping")
            continue

        # Build content mapping
        content_mapping = {c["file"]: c["hash"] for c in content}

        pointer_name = pointers[0].split(":")[-1] if pointers else entity_id[:12]

        # Process each GLTF variant (male/female)
        for gltf in gltf_files:
            processed += 1
            gltf_hash = gltf["hash"]
            gltf_name = gltf["file"]

            # Determine variant (male/female)
            variant = ""
            if "male/" in gltf_name.lower():
                variant = " (male)"
            elif "female/" in gltf_name.lower():
                variant = " (female)"

            print(f"  [{processed}/{total_gltfs}] {pointer_name}{variant} ({gltf_hash[:12]}...)")

            try:
                # Submit to asset server
                response = submit_asset(gltf_hash, asset_type, content_mapping, base_url)
                batch_id = response.get("batch_id")

                if not batch_id:
                    print(f"    Error: No batch_id in response")
                    failed += 1
                    continue

                # Wait for completion
                result = wait_for_batch(batch_id, show_progress=True)

                if result["status"] == "completed":
                    zip_path = result.get('zip_path', 'N/A')
                    # Shorten path for display
                    if zip_path and len(zip_path) > 60:
                        zip_path = "..." + zip_path[-57:]
                    print(f"\r    Completed: {zip_path}" + " " * 10)
                    successful += 1
                else:
                    print(f"\r    Failed: {result.get('error', 'Unknown error')}" + " " * 10)
                    failed += 1

            except (URLError, HTTPError) as e:
                print(f"    Error submitting: {e}")
                failed += 1

    return successful, failed


def main():
    global ASSET_SERVER_URL, CONTENT_SERVER, LAMBDAS_SERVER

    parser = argparse.ArgumentParser(description="Process wearables and emotes via Asset Optimization Server")
    parser.add_argument("address", help="Ethereum address of the profile to process")
    parser.add_argument("--server", default=ASSET_SERVER_URL, help="Asset server URL")
    parser.add_argument("--port", type=int, help="Asset server port (shorthand for --server http://localhost:PORT)")
    parser.add_argument("--content-server", default=CONTENT_SERVER, help="Content server URL")
    parser.add_argument("--lambdas-server", default=LAMBDAS_SERVER, help="Lambdas server URL")
    parser.add_argument("--wearables-only", action="store_true", help="Only process wearables")
    parser.add_argument("--emotes-only", action="store_true", help="Only process emotes")
    parser.add_argument("--concurrent", type=int, default=1, help="Number of concurrent submissions (default: 1)")
    args = parser.parse_args()

    ASSET_SERVER_URL = f"http://localhost:{args.port}" if args.port else args.server
    CONTENT_SERVER = args.content_server
    LAMBDAS_SERVER = args.lambdas_server
    content_base_url = f"{CONTENT_SERVER}/contents/"

    print("=== Wearables & Emotes Optimization Script ===")
    print(f"Asset Server: {ASSET_SERVER_URL}")
    print(f"Content Server: {CONTENT_SERVER}")
    print(f"Profile: {args.address}")
    print()

    # Check health
    print("Checking asset server health...")
    if not check_health():
        print(f"Error: Asset server is not running at {ASSET_SERVER_URL}")
        print("Start it with: cargo run -- run --asset-server")
        sys.exit(1)
    print("Asset server is healthy")
    print()

    # Fetch profile
    print(f"Fetching profile for {args.address}...")
    try:
        profile = fetch_profile(args.address)
    except (ValueError, URLError, HTTPError) as e:
        print(f"Error: {e}")
        sys.exit(1)

    avatar_name = profile.get("avatars", [{}])[0].get("name", "Unknown")
    print(f"Found profile: {avatar_name}")
    print()

    # Extract wearables and emotes
    wearables, emotes = extract_wearables_and_emotes(profile)
    print(f"Found {len(wearables)} wearables (excluding base-avatars)")
    print(f"Found {len(emotes)} emotes (excluding base-emotes)")
    print()

    if not wearables and not emotes:
        print("No wearables or emotes to process.")
        sys.exit(0)

    # Fetch wearable entities
    wearable_entities = []
    emote_entities = []

    if wearables and not args.emotes_only:
        print(f"Fetching {len(wearables)} wearable entities...")
        wearable_entities = fetch_wearable_entities(wearables)
        print(f"  Retrieved {len(wearable_entities)} wearable entities")

    if emotes and not args.wearables_only:
        print(f"Fetching {len(emotes)} emote entities...")
        emote_entities = fetch_wearable_entities(emotes)
        print(f"  Retrieved {len(emote_entities)} emote entities")

    # Process wearables
    start_time = time.time()
    total_successful = 0
    total_failed = 0

    if wearable_entities and not args.emotes_only:
        successful, failed = process_entities(wearable_entities, "wearable", content_base_url)
        total_successful += successful
        total_failed += failed

    if emote_entities and not args.wearables_only:
        successful, failed = process_entities(emote_entities, "emote", content_base_url)
        total_successful += successful
        total_failed += failed

    # Summary
    total_time = time.time() - start_time
    print()
    print("=== Summary ===")
    print(f"Total time: {format_time(total_time)}")
    print(f"Successful: {total_successful}")
    print(f"Failed: {total_failed}")


if __name__ == "__main__":
    main()
