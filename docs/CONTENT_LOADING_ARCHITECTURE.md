# Content Loading Architecture

This document describes the architecture of the content loading system in the Decentraland Godot Explorer, including how assets (GLTF models, textures, audio, wearables, emotes) are downloaded, processed, cached, and loaded.

## Overview

The content loading system is designed for:
- **Performance**: Background thread processing, parallel downloads, disk caching
- **Stability**: Signal-based APIs prevent orphan nodes and memory leaks
- **Efficiency**: LRU cache management, optimized asset support, deduplication

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                              CONTENT LOADING FLOW                           │
└─────────────────────────────────────────────────────────────────────────────┘

┌─────────────────┐     ┌─────────────────┐     ┌─────────────────┐
│   GDScript      │     │   Rust          │     │   Disk Cache    │
│   (Consumers)   │     │   (Processing)  │     │   (Storage)     │
└────────┬────────┘     └────────┬────────┘     └────────┬────────┘
         │                       │                       │
         │  1. Request asset     │                       │
         │─────────────────────▶ │                       │
         │                       │  2. Check cache       │
         │                       │─────────────────────▶ │
         │                       │                       │
         │                       │  3a. Cache HIT        │
         │                       │◀─────────────────────│
         │                       │                       │
         │                       │  3b. Cache MISS       │
         │                       │  Download + Process   │
         │                       │  Save to cache        │
         │                       │─────────────────────▶ │
         │                       │                       │
         │  4. Signal: ready     │                       │
         │◀───────────────────── │                       │
         │                       │                       │
         │  5. Load from cache   │                       │
         │─────────────────────▶ │                       │
```

## Key Components

### 1. ContentProvider (Rust)

**File**: `lib/src/content/content_provider.rs`

The central node for all content loading operations:

```rust
pub struct ContentProvider {
    // File system cache
    content_folder: Arc<String>,           // e.g., "user://content/"
    resource_provider: Arc<ResourceProvider>,

    // In-memory cache for promises (textures, audio, profiles)
    cached: HashMap<String, ContentEntry>,

    // Thread safety for Godot API access
    godot_single_thread: Arc<Semaphore>,

    // Deduplication: prevent loading same asset twice
    loading_scene_hashes: HashSet<String>,
    loading_wearable_hashes: HashSet<String>,
    loading_emote_hashes: HashSet<String>,
}
```

### 2. ResourceProvider (Rust)

**File**: `lib/src/content/resource_provider.rs`

Manages the disk cache with LRU eviction:

```rust
pub struct ResourceProvider {
    cache_root: String,
    max_cache_size: i64,
    current_cache_size: AtomicI64,
    file_access_times: RwLock<HashMap<String, FileAccessInfo>>,
    download_semaphore: Semaphore,
}
```

Key responsibilities:
- Download files with concurrency limiting
- Track file access times for LRU eviction
- Manage total cache size
- Provide async file operations

### 3. GLTF Loader (Rust)

**File**: `lib/src/content/gltf.rs`

Handles GLTF loading and processing:

```rust
// Scene GLTF: Full processing with colliders
pub async fn load_and_save_scene_gltf(...) -> Result<String, anyhow::Error>

// Wearable GLTF: Basic processing, no colliders
pub async fn load_and_save_wearable_gltf(...) -> Result<String, anyhow::Error>

// Emote GLTF: Animation extraction and embedding
pub async fn load_and_save_emote_gltf(...) -> Result<String, anyhow::Error>
```

### 4. Scene Saver (Rust)

**File**: `lib/src/content/scene_saver.rs`

Utilities for saving processed nodes as PackedScene:

```rust
// Save a Node3D as .scn file
pub fn save_node_as_scene(node: Gd<Node3D>, file_path: &str) -> Result<(), String>

// Path generation for different asset types
pub fn get_scene_path_for_hash(content_folder: &str, hash: &str) -> String
pub fn get_wearable_path_for_hash(content_folder: &str, hash: &str) -> String
pub fn get_emote_path_for_hash(content_folder: &str, hash: &str) -> String
```

## Loading APIs

### Scene GLTF Loading

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                          SCENE GLTF LOADING FLOW                            │
└─────────────────────────────────────────────────────────────────────────────┘

gltf_container.gd                    ContentProvider                     Disk
       │                                    │                              │
       │  load_scene_gltf(path, mapping)    │                              │
       │───────────────────────────────────▶│                              │
       │                                    │                              │
       │                                    │  (Background Thread)         │
       │                                    │  Check: {hash}.scn exists?   │
       │                                    │─────────────────────────────▶│
       │                                    │                              │
       │                                    │  Cache HIT: Touch file       │
       │                                    │◀─────────────────────────────│
       │                                    │                              │
       │                                    │  Cache MISS:                 │
       │                                    │  1. Download GLTF + deps     │
       │                                    │  2. Load into Godot          │
       │                                    │  3. Process textures         │
       │                                    │  4. Create colliders         │
       │                                    │  5. Save as .scn             │
       │                                    │─────────────────────────────▶│
       │                                    │                              │
       │  Signal: scene_gltf_ready          │                              │
       │◀───────────────────────────────────│                              │
       │                                    │                              │
       │  ResourceLoader.load_threaded()    │                              │
       │─────────────────────────────────────────────────────────────────▶│
       │                                    │                              │
       │  PackedScene.instantiate()         │                              │
       │◀─────────────────────────────────────────────────────────────────│
```

**GDScript usage** (`gltf_container.gd`):

```gdscript
func _ready():
    Global.content_provider.scene_gltf_ready.connect(_on_gltf_ready)
    Global.content_provider.scene_gltf_error.connect(_on_gltf_error)
    _start_runtime_gltf_load()

func _start_runtime_gltf_load():
    var content_mapping := Global.scene_runner.get_scene_content_mapping(dcl_scene_id)
    Global.content_provider.load_scene_gltf(dcl_gltf_src, content_mapping)

func _on_gltf_ready(file_hash: String, scene_path: String):
    if file_hash != dcl_gltf_hash:
        return
    var gltf_node := await _async_load_and_instantiate(scene_path)
    add_child(gltf_node)
```

### Wearable GLTF Loading

Similar to scene loading but:
- No colliders created
- Cached with prefix `wearable_`
- Signals: `wearable_gltf_ready`, `wearable_gltf_error`

```rust
// Cache path: {content_folder}wearable_{hash}.scn
pub fn get_wearable_path_for_hash(content_folder: &str, hash: &str) -> String {
    format!("{}wearable_{}.scn", content_folder, hash)
}
```

### Emote GLTF Loading

Emotes have special handling because they contain animations that need processing:

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                          EMOTE SCENE STRUCTURE                              │
└─────────────────────────────────────────────────────────────────────────────┘

Saved to disk:
┌─────────────────────────────────────┐
│ EmoteRoot (Node3D)                  │
│ ├── Armature_Prop_{hash} (Node3D)   │  ← Optional prop mesh
│ └── EmoteAnimations (AnimationPlayer)│
│     └── AnimationLibrary ""         │
│         ├── "{hash_suffix}"         │  ← Default animation
│         └── "{hash_suffix}_prop"    │  ← Prop animation (optional)
└─────────────────────────────────────┘
```

**Processing in background thread** (`gltf.rs:load_and_save_emote_gltf`):

1. Download GLTF and dependencies
2. Load into Godot
3. Extract animations via `process_emote_animations()`
4. Create EmoteRoot with:
   - Armature_Prop as child (if present)
   - AnimationPlayer with processed animations
5. Save to disk as `emote_{hash}.scn`

**Loading from cache** (`content_provider.rs:load_cached_emote`):

1. Load PackedScene
2. Read armature_prop (first non-AnimationPlayer child)
3. Read animations from "EmoteAnimations" AnimationPlayer
4. Build `DclEmoteGltf` struct
5. Free the loaded scene root

```rust
#[derive(GodotClass)]
pub struct DclEmoteGltf {
    armature_prop: Option<Gd<Node3D>>,
    default_animation: Option<Gd<Animation>>,
    prop_animation: Option<Gd<Animation>>,
}
```

## Caching Strategy

### Disk Cache

| Asset Type | Path Pattern | Prefix |
|------------|--------------|--------|
| Scene GLTF | `{hash}.scn` | None |
| Wearable | `wearable_{hash}.scn` | `wearable_` |
| Emote | `emote_{hash}.scn` | `emote_` |
| Textures | `{hash}` | None |
| Audio | `{hash}` | None |

### LRU Eviction

The `ResourceProvider` tracks file access times and evicts least-recently-used files when cache exceeds `max_cache_size` (default 2GB):

```rust
// Touch file on access (updates access time)
resource_provider.touch_file_async(&scene_path).await;

// Eviction runs on new file writes
async fn evict_if_needed(&self) {
    while self.current_cache_size.load() > self.max_cache_size {
        // Remove least recently used file
    }
}
```

### Deduplication

Each asset type has a `loading_*_hashes` set to prevent duplicate loads:

```rust
// Check if already loading
if self.loading_scene_hashes.contains(&file_hash) {
    return true;  // Signal will be emitted when complete
}

// Mark as loading
self.loading_scene_hashes.insert(file_hash.clone());

// On completion, remove from set
self.loading_scene_hashes.remove(&hash_str);
```

## Optimized Assets

Pre-processed assets are available from `optimized-assets.dclexplorer.com`:

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                        OPTIMIZED ASSET LOADING                              │
└─────────────────────────────────────────────────────────────────────────────┘

1. Check: optimized_asset_exists(hash)?
2. Download: {hash}-mobile.zip from CDN
3. Load: ProjectSettings.load_resource_pack(zip_path)
4. Access: ResourceLoader.load("res://glbs/{hash}.tscn")
```

Benefits:
- Pre-compressed textures
- Pre-baked colliders
- Faster loading (no runtime processing)

## Thread Safety

### Godot API Access

A semaphore ensures only one thread accesses Godot APIs at a time:

```rust
pub struct GodotThreadSafetyGuard {
    _guard: tokio::sync::OwnedSemaphorePermit,
}

impl GodotThreadSafetyGuard {
    pub async fn acquire(godot_single_thread: &Arc<Semaphore>) -> Option<Self> {
        let guard = godot_single_thread.clone().acquire_owned().await.ok()?;
        set_thread_safety_checks_enabled(false);
        Some(Self { _guard: guard })
    }
}

impl Drop for GodotThreadSafetyGuard {
    fn drop(&mut self) {
        set_thread_safety_checks_enabled(true);
    }
}
```

### Signal-Based Communication

Signals ensure main thread handles Godot node instantiation:

```rust
// Background thread saves to disk, then signals
TokioRuntime::spawn(async move {
    let result = load_and_save_scene_gltf(...).await;

    // Callback to main thread via call_deferred
    provider.call_deferred("on_scene_gltf_load_complete", &[...]);
});

// Main thread receives signal, instantiates scene
#[func]
fn on_scene_gltf_load_complete(&mut self, file_hash: GString, scene_path: GString, error: GString) {
    self.loading_scene_hashes.remove(&hash_str);
    self.base_mut().emit_signal("scene_gltf_ready", &[...]);
}
```

## Collider Creation

Scene GLTFs have colliders created during processing:

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                          COLLIDER CREATION FLOW                             │
└─────────────────────────────────────────────────────────────────────────────┘

1. For each MeshInstance3D:
   ├── Create trimesh collision
   ├── Replace StaticBody3D with AnimatableBody3D
   ├── Set collision_layer = 0, collision_mask = 0
   ├── Set process_mode = DISABLED
   └── Set metadata: dcl_col, invisible_mesh

2. After instantiation (gltf_container.gd):
   ├── set_mask_colliders() assigns proper masks
   ├── Set PhysicsServer3D.BODY_MODE_STATIC
   └── Enable transform tracking if has colliders

3. On entity movement (Rust detects via transform tracking):
   ├── Emit switch_to_kinematic signal
   └── GDScript switches colliders to BODY_MODE_KINEMATIC
```

## Error Handling

| Error | Location | Recovery |
|-------|----------|----------|
| File not in content mapping | ContentProvider | Emit error signal immediately |
| Download failed | ResourceProvider | Emit error signal with message |
| GLTF parse error | gltf.rs | Emit error signal with details |
| Save failed | scene_saver.rs | Emit error signal |
| Load timeout | gltf_container.gd | Timer fires, finishes with error |

## Performance Optimizations

1. **Parallel Downloads**: Up to 32 concurrent downloads (configurable)
2. **Throttled Loading**: MAX_CONCURRENT_LOADS = 10 for scene instantiation
3. **Priority Queue**: Current scene's assets loaded first
4. **Texture Resizing**: Based on texture quality setting
5. **iOS Compression**: Creates compressed textures on iOS for memory efficiency

## Files Reference

| File | Purpose |
|------|---------|
| `lib/src/content/content_provider.rs` | Main content loading node |
| `lib/src/content/gltf.rs` | GLTF loading and processing |
| `lib/src/content/scene_saver.rs` | PackedScene saving utilities |
| `lib/src/content/resource_provider.rs` | Disk cache management |
| `lib/src/content/texture.rs` | Texture loading and processing |
| `lib/src/content/audio.rs` | Audio loading |
| `godot/src/decentraland_components/gltf_container.gd` | Scene GLTF component |

## Avatar Wearable Loading

Avatar wearables have two loading paths:

### 1. Promise-Based API (Current for Avatars)

Used by `avatar.gd` for loading avatar wearables:

```gdscript
# In avatar.gd async_fetch_wearables_dependencies
wearable_promises = await Wearables.async_load_wearables(...)
# Then later in async_load_wearables
var obj = Global.content_provider.get_gltf_from_hash(file_hash)
```

This flow:
- Uses `fetch_wearable_gltf()` which loads in background thread
- Keeps the loaded Node3D in memory cache (not disk)
- Uses `duplicate()` to create copies for each avatar

### 2. Signal-Based API (Available but unused for avatars)

The disk-caching signal-based API exists for wearables:

```gdscript
# Start loading
Global.content_provider.load_wearable_gltf(file_path, content_mapping)
# Wait for signal
await Global.content_provider.wearable_gltf_ready
# Load from disk using ResourceLoader
ResourceLoader.load_threaded_request(scene_path)
```

### Future Improvement: Threaded Avatar Wearable Loading

Currently, avatar wearables can cause minor hiccups because:
1. The Promise-based flow doesn't use disk caching
2. Wearables are loaded from network even if cached on disk

To improve this, the avatar loading could be refactored to:
1. Use `load_wearable_gltf()` (signal-based, saves to disk)
2. Wait for `wearable_gltf_ready` signal with scene_path
3. Use `ResourceLoader.load_threaded_request()` for non-blocking disk loading
4. Instantiate and add to avatar

This is a complex refactor because:
- Need to track multiple wearable loads in parallel
- Need to maintain backward compatibility
- Avatar loading has complex dependencies (textures, emotes, etc.)

## Debugging

Enable detailed logging:
```bash
RUST_LOG="warn,dclgodot::content=debug" cargo run -- run
```

Key log messages:
- `Scene GLTF cache HIT/MISS` - Cache status for scene loading
- `Wearable GLTF cache HIT/MISS` - Cache status for wearables
- `Emote GLTF cache HIT/MISS` - Cache status for emotes
- `GLTF processed: {hash} with {n} nodes` - Processing complete
- `Failed to...` - Error conditions
