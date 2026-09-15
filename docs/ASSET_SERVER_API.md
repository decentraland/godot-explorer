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
    { "hash": "bafkrei...", "zip_path": "/path/to/bafkrei....scn" },
    { "hash": "bafkrei...", "zip_path": "/path/to/bafkrei....res" },
    { "hash": "bafybei...", "zip_path": "/path/to/bafybei....js" },
    { "hash": "<output_hash>", "zip_path": "/path/to/<output_hash>-optimized.json" },
    { "hash": "<output_hash>", "zip_path": "/path/to/<output_hash>-boot.zip" }
  ]
}
```

The `individual_zips` field is present for scene batches and lists every file the batch published, in upload order (see *Uploader contract* under *Output Structure*). It is omitted when empty.

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

## Output Structure

### Scene assets (v6 layout: plain files)

Each processed scene asset is published as a plain Godot resource next to the
manifest — no zip, no `load_resource_pack` on the client. The client downloads
them into `user://content/{key}.opt.scn` / `{hash}.opt.res` and loads them by path;
a scene `.scn` references its textures as `user://content/{hash}.opt.res`
ExtResources, which is why every `externalSceneDependencies` entry must be on
disk before the `.scn` is loaded (the client awaits all of them).

```
{gltf_key}.scn                # PackedScene, zstd-compressed (RSCC); see "Scene GLB key" below
{texture_hash}.res            # PortableCompressedTexture2D, zstd-compressed
{main_js_hash}.js             # the scene's main script (scene.json `main`), verbatim
{main_crdt_hash}.crdt         # main.crdt, verbatim (when the scene has one)
{output_hash}-optimized.json  # manifest (see below); `bootFiles` lists the two above
{output_hash}-boot.zip        # manifest + main.js + main.crdt, Deflated — one request to boot
{output_hash}-static.zip      # every model main.crdt composes (+ the textures their .scn
                              # files reference), Stored — one stream instead of N requests
```

**Scene GLB key.** A GLB references its textures by file name, so the same
GLB hash maps to a different texture set in another deployment (a redeploy
that swaps one image, another scene mapping the model to its own textures),
and its baked `.scn` embeds that set as ExtResources. Keyed by the GLB hash
alone, the last bake would overwrite the others in the shared bucket and
every other scene's manifest would point at textures its `.scn` does not
reference. So the published name is `scene_bake_key(hash, deps)`
(`lib/src/content/content_provider.rs`): the GLB hash when the GLB has no
external textures, otherwise `{hash}-{first 16 hex of sha256(sorted unique
texture hashes, each followed by "\n")}`. The manifest keeps the plain GLB
hash in `optimizedContent` and the texture list in
`externalSceneDependencies`; the client derives the same key from them
(`{key}.opt.scn` on disk). Textures are content-addressed by their own hash.

Zip entry names are the client's cache file names (`{key}.opt.scn`,
`{hash}.opt.res`, the bare hash for main.js / main.crdt,
`{output_hash}-optimized.json`), so the client **extracts** them straight into
`user://content/` — never mounts them — and then loads by path like the
per-asset files. The client tries `{output_hash}-boot.zip` first (404 → the
three separate fetches) and, once the manifest is parsed, streams
`{output_hash}-static.zip` in the background while the scene starts: GLTF
requests for files inside it wait for the extraction instead of downloading
per-file; if the zip is missing or broken they download per-file as usual.
The manifest's `staticBundle.files` lists the entries so the client can skip
the download when every file is already cached. The static zip is written
only when `main.crdt` references at least one baked model (Genesis Plaza:
1 GLB; Creator Hub scenes: most of the world). Both zips are
`application/zip` — never CDN-compressed — and per-asset files stay the
source of truth (content-addressed, shared across scenes).

The client fetches the manifest, `main.js` and `main.crdt` from this bucket
first (in parallel, together with the content-server fallback on a 404), so a
whole scene comes from one CDN host. **Upload content types matter**: publish
`.js` as `application/javascript` and `.json` as `application/json` so the CDN
compresses them on the fly (the client sends `Accept-Encoding: gzip, br`);
Genesis Plaza's `main.js` is 2.26 MB raw, 634 KB as the content server's gzip,
390 KB as brotli-11, and its manifest 340 KB → 54 KB. `.scn`/`.res`/`.crdt` are
`application/octet-stream` (already compressed or tiny).

Mounting a zip per asset cost `refresh_global_class_list()` +
`ResourceUID::load_from_cache()` on the main thread for every mount (~2 ms ×
~900 per Genesis Plaza load), and deflate on top of the inner zstd saved only
0.3–4% of bytes.

**Uploader contract.** `individual_zips` in the batch status lists every file
above, in this order: `.scn`/`.res` assets, `.js`/`.crdt` boot files,
`-static.zip`, `-optimized.json`, `-boot.zip`. Publish the **basename of
`zip_path`** as the object key (the `hash` field is the source hash for
assets and boot files, the output hash for the scene-level files) and upload
**in list order** — the manifest and the boot zip are last on purpose, so a
client fetching mid-upload never sees a manifest whose dependencies are not
there yet. The legacy `zip_path` (`{output_hash}-mobile.zip`, below) is
uploaded after them as before.

### Main Metadata ZIP (legacy)

The main ZIP (`{output_hash}-mobile.zip`) is still written and always contains the metadata JSON. If `preloaded_hashes` are specified, those assets are also included:

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
| `bootFiles` | `{"main.js": hash, "main.crdt": hash}` — boot files published in the bucket as `{hash}.js` / `{hash}.crdt` (absent when none) |
| `bootBundle` | `{"file": "{output_hash}-boot.zip"}` (informational) |
| `staticBundle` | `{"file": "{output_hash}-static.zip", "files": [entry names], "bytes": N}` — absent when main.crdt references no baked model |

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
