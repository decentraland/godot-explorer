#!/bin/bash
# Test script for the Asset Optimization Server
# Fetches a Decentraland scene and submits it for processing
#
# Usage:
#   ./test_asset_server.sh [pointer] [count]
#
# Examples:
#   ./test_asset_server.sh 0,0      # Process first GLB from Genesis Plaza
#   ./test_asset_server.sh 0,0 all  # Process ALL GLBs from Genesis Plaza
#   ./test_asset_server.sh 0,0 5    # Process first 5 GLBs
#
# Output:
#   Creates a ZIP file at {content_folder}/{entity_id}-mobile.zip

set -e

ASSET_SERVER_URL="${ASSET_SERVER_URL:-http://localhost:8080}"
CONTENT_SERVER="https://peer.decentraland.org/content"
POINTER="${1:-0,0}"
COUNT="${2:-1}"

echo "=== Asset Server Test Script ==="
echo "Asset Server: $ASSET_SERVER_URL"
echo "Content Server: $CONTENT_SERVER"
echo "Pointer: $POINTER"
echo "Count: $COUNT"
echo ""

# Check if jq is installed
if ! command -v jq &> /dev/null; then
    echo "Error: jq is required but not installed."
    echo "Install with: brew install jq (macOS) or apt install jq (Linux)"
    exit 1
fi

# Check if asset server is running
echo "Checking asset server health..."
if ! curl -s "$ASSET_SERVER_URL/health" | jq -e '.status == "ok"' > /dev/null 2>&1; then
    echo "Error: Asset server is not running at $ASSET_SERVER_URL"
    echo "Start it with: cargo run -- run --asset-server"
    exit 1
fi
echo "✓ Asset server is healthy"
echo ""

# Fetch scene entity
echo "Fetching scene entity for pointer '$POINTER'..."
ENTITY_RESPONSE=$(curl -s -X POST "$CONTENT_SERVER/entities/active" \
    -H "Content-Type: application/json" \
    -d "{\"pointers\": [\"$POINTER\"]}")

# Check if we got a valid response
ENTITY_COUNT=$(echo "$ENTITY_RESPONSE" | jq 'length')
if [ "$ENTITY_COUNT" -eq 0 ]; then
    echo "Error: No scene found at pointer '$POINTER'"
    exit 1
fi

echo "✓ Found $ENTITY_COUNT entity/entities"
echo ""

# Extract the first entity
ENTITY=$(echo "$ENTITY_RESPONSE" | jq '.[0]')
ENTITY_ID=$(echo "$ENTITY" | jq -r '.id')
ENTITY_TYPE=$(echo "$ENTITY" | jq -r '.type')

echo "Entity ID: $ENTITY_ID"
echo "Entity Type: $ENTITY_TYPE"
echo ""

# Build content mapping from the entity's content array
# Format: { "file_path": "hash", ... }
CONTENT_MAPPING=$(echo "$ENTITY" | jq '[.content[] | {(.file): .hash}] | add // {}')

# Find GLB/GLTF files in the content
echo "Looking for GLTF/GLB files..."
GLTF_FILES=$(echo "$ENTITY" | jq -r '.content[] | select(.file | test("\\.(glb|gltf)$"; "i")) | "\(.file) -> \(.hash)"')

if [ -z "$GLTF_FILES" ]; then
    echo "No GLTF/GLB files found in this scene"
    exit 1
fi

GLTF_TOTAL=$(echo "$ENTITY" | jq '[.content[] | select(.file | test("\\.(glb|gltf)$"; "i"))] | length')
echo "Found $GLTF_TOTAL GLTF/GLB files"
echo ""

# Build the base URL for content
BASE_URL="$CONTENT_SERVER/contents/"

# Determine how many to process
if [ "$COUNT" == "all" ]; then
    LIMIT=$GLTF_TOTAL
else
    LIMIT=$COUNT
fi

echo "=== Processing $LIMIT GLTF file(s) ==="
echo ""

# Build the process request with array of assets
# Use entity ID as output_hash for the ZIP filename
PROCESS_REQUEST=$(echo "$ENTITY" | jq --arg base_url "$BASE_URL" --argjson content_mapping "$CONTENT_MAPPING" --argjson limit "$LIMIT" --arg entity_id "$ENTITY_ID" '
    {
        output_hash: $entity_id,
        assets: [
            .content[] | select(.file | test("\\.(glb|gltf)$"; "i"))
        ][:$limit] | map({
            url: ($base_url + .hash),
            type: "scene",
            hash: .hash,
            base_url: $base_url,
            content_mapping: $content_mapping
        })
    }
')

ASSET_COUNT=$(echo "$PROCESS_REQUEST" | jq '.assets | length')
echo "Submitting $ASSET_COUNT asset(s) to asset server..."
echo ""

# Show first few assets
if [ "$ASSET_COUNT" -gt 3 ]; then
    echo "First 3 assets:"
    echo "$PROCESS_REQUEST" | jq '.assets[:3][] | {hash, url}'
    echo "... and $((ASSET_COUNT - 3)) more"
else
    echo "Assets:"
    echo "$PROCESS_REQUEST" | jq '.assets[] | {hash, url}'
fi
echo ""

# Submit to asset server
RESPONSE=$(curl -s -X POST "$ASSET_SERVER_URL/process" \
    -H "Content-Type: application/json" \
    -d "$PROCESS_REQUEST")

echo "Response:"
echo "$RESPONSE" | jq '.'
echo ""

BATCH_ID=$(echo "$RESPONSE" | jq -r '.batch_id')
OUTPUT_HASH=$(echo "$RESPONSE" | jq -r '.output_hash')
TOTAL=$(echo "$RESPONSE" | jq -r '.total')
echo "Batch ID: $BATCH_ID"
echo "Output Hash: $OUTPUT_HASH"
echo "Submitted $TOTAL jobs"
echo ""

# Poll batch status until complete
echo "=== Polling batch status ==="
while true; do
    BATCH_RESPONSE=$(curl -s "$ASSET_SERVER_URL/status/$BATCH_ID")

    BATCH_STATUS=$(echo "$BATCH_RESPONSE" | jq -r '.status')
    PROGRESS=$(echo "$BATCH_RESPONSE" | jq -r '.progress')
    PROGRESS_PCT=$(echo "$PROGRESS * 100" | bc -l | xargs printf "%.0f")

    QUEUED=$(echo "$BATCH_RESPONSE" | jq '[.jobs[] | select(.status == "queued")] | length')
    DOWNLOADING=$(echo "$BATCH_RESPONSE" | jq '[.jobs[] | select(.status == "downloading")] | length')
    PROCESSING=$(echo "$BATCH_RESPONSE" | jq '[.jobs[] | select(.status == "processing")] | length')
    COMPLETED=$(echo "$BATCH_RESPONSE" | jq '[.jobs[] | select(.status == "completed")] | length')
    FAILED=$(echo "$BATCH_RESPONSE" | jq '[.jobs[] | select(.status == "failed")] | length')

    printf "\rBatch: %-10s Progress: %3d%%  Q: %d  D: %d  P: %d  ✓: %d  ✗: %d" \
        "$BATCH_STATUS" "$PROGRESS_PCT" "$QUEUED" "$DOWNLOADING" "$PROCESSING" "$COMPLETED" "$FAILED"

    if [ "$BATCH_STATUS" == "completed" ] || [ "$BATCH_STATUS" == "failed" ]; then
        echo ""
        echo ""
        break
    fi

    sleep 1
done

# Show final batch status
echo "=== Batch Complete ==="
FINAL_RESPONSE=$(curl -s "$ASSET_SERVER_URL/status/$BATCH_ID")
FINAL_STATUS=$(echo "$FINAL_RESPONSE" | jq -r '.status')
ZIP_PATH=$(echo "$FINAL_RESPONSE" | jq -r '.zip_path // "N/A"')
ERROR=$(echo "$FINAL_RESPONSE" | jq -r '.error // "N/A"')

if [ "$FINAL_STATUS" == "completed" ]; then
    echo "✓ Status: $FINAL_STATUS"
    echo "✓ ZIP Path: $ZIP_PATH"
    echo ""
    # Check if ZIP exists
    if [ -f "$ZIP_PATH" ]; then
        ZIP_SIZE=$(ls -lh "$ZIP_PATH" | awk '{print $5}')
        echo "ZIP file created successfully: $ZIP_PATH ($ZIP_SIZE)"
        echo ""
        echo "ZIP contents:"
        unzip -l "$ZIP_PATH" | head -20
    fi
else
    echo "✗ Status: $FINAL_STATUS"
    echo "✗ Error: $ERROR"
fi

COMPLETED=$(echo "$FINAL_RESPONSE" | jq '[.jobs[] | select(.status == "completed")] | length')
FAILED=$(echo "$FINAL_RESPONSE" | jq '[.jobs[] | select(.status == "failed")] | length')
echo ""
echo "Jobs completed: $COMPLETED"
echo "Jobs failed: $FAILED"

if [ "$FAILED" -gt 0 ]; then
    echo ""
    echo "Failed jobs:"
    echo "$FINAL_RESPONSE" | jq '.jobs[] | select(.status == "failed") | {job_id, error}'
fi

echo ""
echo "=== All jobs in batch ==="
echo "$FINAL_RESPONSE" | jq '.jobs[] | {job_id, status, asset_type, elapsed_secs, optimized_path}'
