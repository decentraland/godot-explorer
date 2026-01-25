# Asset Optimization Server API

The Asset Optimization Server processes Decentraland assets (GLTFs, textures) and packages them into optimized ZIP files for efficient loading on mobile/low-end devices.

## Starting the Server

```bash
cargo run -- run --asset-server
```

By default, the server runs on `http://localhost:8080`.

## Endpoints

### Health Check

```
GET /health
```

Returns the server health status.

**Response:**
```json
{
  "status": "ok"
}
```

---

### List All Jobs

```
GET /jobs
```

Returns all active jobs and batches.

**Response:**
```json
{
  "jobs": [
    {
      "job_id": "uuid",
      "hash": "bafkrei...",
      "asset_type": "scene",
      "status": "completed",
      "progress": 1.0,
      "elapsed_secs": 12.5,
      "optimized_path": "/path/to/file.scn"
    }
  ],
  "batches": [
    {
      "batch_id": "uuid",
      "output_hash": "bafkrei...",
      "status": "completed",
      "job_count": 10,
      "zip_path": "/path/to/file-mobile.zip",
      "elapsed_secs": 45.2
    }
  ]
}
```

---

### Get Job Status

```
GET /status/job/{job_id}
```

Returns the status of a single processing job.

**Response:**
```json
{
  "job_id": "uuid",
  "hash": "bafkrei...",
  "asset_type": "texture",
  "status": "processing",
  "progress": 0.5,
  "elapsed_secs": 3.2,
  "optimized_path": null,
  "error": null
}
```

---

### Get Batch Status

```
GET /status/{batch_id}
```

Returns the status of a batch and all its jobs.

**Response:**
```json
{
  "batch_id": "uuid",
  "output_hash": "bafkrei...",
  "status": "completed",
  "progress": 1.0,
  "jobs": [
    {
      "job_id": "uuid",
      "hash": "bafkrei...",
      "asset_type": "scene",
      "status": "completed",
      "progress": 1.0,
      "elapsed_secs": 10.5
    }
  ],
  "zip_path": "/path/to/output-mobile.zip",
  "error": null
}
```

---

### Process Assets

```
POST /process
```

Submit individual assets for processing. Creates a batch that packages all assets into a single ZIP.

**Request:**
```json
{
  "output_hash": "my-bundle-v1",
  "assets": [
    {
      "url": "https://peer.decentraland.org/content/contents/bafkrei...",
      "type": "scene",
      "hash": "bafkrei...",
      "base_url": "https://peer.decentraland.org/content/contents/",
      "content_mapping": {
        "models/tree.glb": "bafkrei...",
        "textures/bark.png": "bafkrei..."
      }
    },
    {
      "url": "https://peer.decentraland.org/content/contents/bafkrei...",
      "type": "texture",
      "hash": "bafkrei...",
      "base_url": "https://peer.decentraland.org/content/contents/",
      "content_mapping": {}
    }
  ]
}
```

**Response:**
```json
{
  "batch_id": "uuid",
  "output_hash": "my-bundle-v1",
  "jobs": [
    {
      "job_id": "uuid",
      "hash": "bafkrei...",
      "status": "queued"
    }
  ],
  "total": 2
}
```

---

### Process Scene

```
POST /process-scene
```

Process an entire Decentraland scene by its entity hash. The server automatically discovers all assets (GLTFs, textures) in the scene and processes them.

**Request:**
```json
{
  "scene_hash": "bafkreicnqmtrwpqxgkp5qpa7tka6tq3ef5qm2jfgvqenxhxhvvp4j5odam",
  "content_base_url": "https://peer.decentraland.org/content/contents/",
  "output_hash": "my-scene-v1",
  "pack_hashes": ["bafkrei...", "bafkrei..."]
}
```

**Parameters:**

| Field | Required | Description |
|-------|----------|-------------|
| `scene_hash` | Yes | The scene entity hash from the content server |
| `content_base_url` | Yes | Base URL for fetching content (must end with `/`) |
| `output_hash` | No | Custom output filename (defaults to `scene_hash`) |
| `pack_hashes` | No | Filter which assets to include in ZIP. If omitted, all assets are included. If empty array `[]`, only metadata is included (no assets). |

**Response:**
```json
{
  "batch_id": "uuid",
  "output_hash": "my-scene-v1",
  "scene_hash": "bafkreicnqmtrwpqxgkp5qpa7tka6tq3ef5qm2jfgvqenxhxhvvp4j5odam",
  "total_assets": 186,
  "pack_assets": 10,
  "jobs": [
    {
      "job_id": "uuid",
      "hash": "bafkrei...",
      "status": "queued"
    }
  ]
}
```

---

## Asset Types

| Type | Description | Output |
|------|-------------|--------|
| `scene` | Scene GLTF/GLB with colliders | `.scn` (Godot PackedScene) |
| `wearable` | Wearable GLTF/GLB without colliders | `.scn` (Godot PackedScene) |
| `emote` | Emote GLTF/GLB with animation extraction | `.scn` (Godot PackedScene) |
| `texture` | Image (PNG, JPG, WebP) | `.res` (Godot CompressedTexture2D) |

---

## Job Statuses

| Status | Description |
|--------|-------------|
| `queued` | Job is waiting to be processed |
| `downloading` | Downloading the source asset |
| `processing` | Converting/optimizing the asset |
| `completed` | Successfully processed |
| `failed` | Error occurred (check `error` field) |

---

## Batch Statuses

| Status | Description |
|--------|-------------|
| `processing` | Jobs are still being processed |
| `packing` | All jobs done, creating ZIP file |
| `completed` | ZIP created successfully |
| `failed` | Error occurred (check `error` field) |

---

## ZIP Output Structure

### Full Scene Pack

When processing a scene with all assets:

```
{output_hash}-mobile.zip
├── {scene_hash}-optimized.json    # Metadata
├── glbs/
│   ├── {gltf_hash_1}.scn          # Processed GLTFs
│   ├── {gltf_hash_2}.scn
│   └── ...
└── content/
    ├── {texture_hash_1}.res       # Processed textures
    ├── {texture_hash_2}.res
    └── ...
```

### Metadata-Only Pack

When `pack_hashes` is an empty array:

```
{output_hash}-mobile.zip
└── {scene_hash}-optimized.json    # Metadata only
```

### Individual Asset Pack

When processing a single asset:

```
{hash}-mobile.zip
└── glbs/{hash}.scn                # For GLTFs
    OR
└── content/{hash}.res             # For textures
```

---

## Metadata JSON Format

The `{scene_hash}-optimized.json` file contains:

```json
{
  "optimizedContent": [
    "bafkrei...",
    "bafkrei..."
  ],
  "externalSceneDependencies": {
    "bafkrei-gltf-hash": ["bafkrei-texture-1", "bafkrei-texture-2"]
  },
  "originalSizes": {
    "bafkrei-texture-hash": {
      "width": 2048,
      "height": 2048
    }
  },
  "hashSizeMap": {
    "bafkrei...": 125000,
    "bafkrei...": 45000
  }
}
```

| Field | Description |
|-------|-------------|
| `optimizedContent` | List of all successfully optimized asset hashes |
| `externalSceneDependencies` | Map of GLTF hash to its texture dependencies |
| `originalSizes` | Original dimensions of textures (before optimization) |
| `hashSizeMap` | Optimized file sizes in bytes |

---

## Example Usage

### Process a Scene (Full Pack)

```bash
# Submit scene for processing
curl -X POST http://localhost:8080/process-scene \
  -H "Content-Type: application/json" \
  -d '{
    "scene_hash": "bafkreicnqmtrwpqxgkp5qpa7tka6tq3ef5qm2jfgvqenxhxhvvp4j5odam",
    "content_base_url": "https://peer.decentraland.org/content/contents/"
  }'

# Response: {"batch_id": "abc-123", ...}

# Poll for completion
curl http://localhost:8080/status/abc-123

# When status is "completed", zip_path contains the output file
```

### Process Scene (Metadata Only)

```bash
curl -X POST http://localhost:8080/process-scene \
  -H "Content-Type: application/json" \
  -d '{
    "scene_hash": "bafkrei...",
    "content_base_url": "https://peer.decentraland.org/content/contents/",
    "pack_hashes": []
  }'
```

### Process Scene (Selective Packing)

```bash
curl -X POST http://localhost:8080/process-scene \
  -H "Content-Type: application/json" \
  -d '{
    "scene_hash": "bafkrei...",
    "content_base_url": "https://peer.decentraland.org/content/contents/",
    "pack_hashes": ["bafkrei-gltf-1", "bafkrei-texture-1"]
  }'
```

---

## Test Script

A Python test script is provided for testing the server:

```bash
# Process Genesis Plaza (full pack)
./scripts/test_asset_server.py --full 0,0

# Process with metadata only, then create individual bundles
./scripts/test_asset_server.py --individual 0,0

# Use custom port
./scripts/test_asset_server.py --port 9000 0,0

# Process by scene hash directly
./scripts/test_asset_server.py --scene-hash bafkrei...
```
