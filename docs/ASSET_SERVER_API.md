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
  "error": null,
  "individual_zips": [
    {
      "hash": "bafkrei...",
      "zip_path": "/path/to/bafkrei...-mobile.zip"
    }
  ]
}
```

The `individual_zips` field is present for scene batches and lists the per-asset ZIP files created. It is omitted when empty.

---

### Process Assets

```
POST /process
```

Submit individual assets for processing (wearables/emotes). Creates a batch that packages all assets into a single ZIP.

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

Process an entire Decentraland scene by its entity hash. The server automatically discovers all assets (GLTFs, textures) in the scene, processes them, creates **one ZIP per asset**, and creates a **main metadata ZIP**.

**Request:**
```json
{
  "scene_hash": "bafkreicnqmtrwpqxgkp5qpa7tka6tq3ef5qm2jfgvqenxhxhvvp4j5odam",
  "content_base_url": "https://peer.decentraland.org/content/contents/",
  "output_hash": "my-scene-v1",
  "preloaded_hashes": ["bafkrei...", "bafkrei..."]
}
```

**Parameters:**

| Field | Required | Description |
|-------|----------|-------------|
| `scene_hash` | Yes | The scene entity hash from the content server |
| `content_base_url` | Yes | Base URL for fetching content (must end with `/`) |
| `output_hash` | No | Custom output filename (defaults to `scene_hash`) |
| `preloaded_hashes` | No | Asset hashes to include in the main metadata ZIP alongside the JSON. If omitted, the main ZIP contains only metadata. |
| `cache_only` | No | If `true`, only use cached files — don't download anything. Default `false`. |

**Response:**
```json
{
  "batch_id": "uuid",
  "output_hash": "my-scene-v1",
  "scene_hash": "bafkreicnqmtrwpqxgkp5qpa7tka6tq3ef5qm2jfgvqenxhxhvvp4j5odam",
  "total_assets": 186,
  "preloaded_assets": 2,
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
| `packing` | All jobs done, creating ZIP files |
| `completed` | All ZIPs created successfully |
| `failed` | Error occurred (check `error` field) |

---

## ZIP Output Structure

### Individual Asset ZIPs

Each processed asset gets its own ZIP (`{hash}-mobile.zip`):

```
{gltf_hash}-mobile.zip
└── glbs/{gltf_hash}.scn

{texture_hash}-mobile.zip
└── content/{texture_hash}.res
```

### Main Metadata ZIP

The main ZIP (`{output_hash}-mobile.zip`) always contains the metadata JSON. If `preloaded_hashes` are specified, those assets are also included:

```
{output_hash}-mobile.zip
├── {output_hash}-optimized.json   # Always present
├── glbs/
│   └── {preloaded_gltf}.scn      # Only if in preloaded_hashes
└── content/
    └── {preloaded_texture}.res   # Only if in preloaded_hashes
```

### Wearable/Emote Pack (`/process`)

All assets packed together:

```
{output_hash}-mobile.zip
├── glbs/
│   ├── {hash_1}.scn
│   └── ...
└── content/
    ├── {hash_1}.res
    └── ...
```

---

## Metadata JSON Format

The `{output_hash}-optimized.json` file contains:

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

### Process a Scene

```bash
# Submit scene for processing (metadata only + individual ZIPs per asset)
curl -X POST http://localhost:8080/process-scene \
  -H "Content-Type: application/json" \
  -d '{
    "scene_hash": "bafkreicnqmtrwpqxgkp5qpa7tka6tq3ef5qm2jfgvqenxhxhvvp4j5odam",
    "content_base_url": "https://peer.decentraland.org/content/contents/"
  }'

# Response: {"batch_id": "abc-123", ...}

# Poll for completion
curl http://localhost:8080/status/abc-123

# When status is "completed":
# - zip_path contains the main metadata ZIP
# - individual_zips lists each per-asset ZIP
```

### Process Scene with Preloaded Assets

```bash
curl -X POST http://localhost:8080/process-scene \
  -H "Content-Type: application/json" \
  -d '{
    "scene_hash": "bafkrei...",
    "content_base_url": "https://peer.decentraland.org/content/contents/",
    "preloaded_hashes": ["bafkrei-gltf-1", "bafkrei-texture-1"]
  }'
```

The main metadata ZIP will include the specified assets alongside the JSON metadata.

---

## Test Script

A Python test script is provided for testing the server:

```bash
# Process a scene (individual ZIPs + metadata-only main ZIP)
./scripts/test_asset_server.py 0,0

# Process with preloaded assets in main ZIP
./scripts/test_asset_server.py --preloaded-hashes hash1,hash2 0,0

# Use custom port
./scripts/test_asset_server.py --port 9000 0,0

# Process by scene hash directly
./scripts/test_asset_server.py --scene-hash bafkrei...
```
