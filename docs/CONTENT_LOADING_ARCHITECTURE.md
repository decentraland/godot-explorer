# Content Loading Architecture

This document describes how the Decentraland Godot Explorer downloads, processes, caches, and loads assets (GLTF models, textures, audio, wearables, emotes).

## Overview

The content loading system is designed for:
- **Performance**: Background thread processing, parallel downloads, disk caching
- **Stability**: Promise-based APIs with proper cleanup
- **Efficiency**: LRU cache management, optimized asset support, deduplication

## How Content Loading Works

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                           CONTENT LOADING FLOW                              │
└─────────────────────────────────────────────────────────────────────────────┘

┌─────────────────┐     ┌─────────────────┐     ┌─────────────────┐
│   GDScript      │     │   Rust          │     │   Disk Cache    │
│   (Consumers)   │     │   (Processing)  │     │   (Storage)     │
└────────┬────────┘     └────────┬────────┘     └────────┬────────┘
         │                       │                       │
         │  1. Request asset     │                       │
         │   (returns Promise)   │                       │
         │──────────────────────▶│                       │
         │                       │                       │
         │                       │  2. Check disk cache  │
         │                       │──────────────────────▶│
         │                       │                       │
         │                       │  3a. Cache HIT        │
         │                       │   (touch + return)    │
         │                       │◀──────────────────────│
         │                       │                       │
         │                       │  3b. Cache MISS       │
         │                       │   Download → Process  │
         │                       │   → Save to cache     │
         │                       │──────────────────────▶│
         │                       │                       │
         │  4. Promise resolves  │                       │
         │   with scene_path     │                       │
         │◀──────────────────────│                       │
         │                       │                       │
         │  5. ResourceLoader    │                       │
         │   .load_threaded()    │                       │
         │──────────────────────────────────────────────▶│
         │                       │                       │
         │  6. PackedScene       │                       │
         │   .instantiate()      │                       │
         │◀──────────────────────────────────────────────│
```

**Key concept**: Rust processes assets in background threads, saves them to disk as `.scn` files (Godot's PackedScene format), then GDScript loads them using Godot's threaded ResourceLoader.

---

## Key Components

### ContentProvider (Rust)

**File**: `lib/src/content/content_provider.rs`

The central node for all content loading operations. Exposes functions to GDScript and handles background processing.

```rust
#[derive(GodotClass)]
pub struct ContentProvider {
    // Cache folder path (e.g., "{user_data}/content/")
    content_folder: Arc<String>,

    // Manages disk cache and downloads
    resource_provider: Arc<ResourceProvider>,

    // HTTP request queue with rate limiting
    http_queue_requester: Arc<HttpQueueRequester>,

    // Promise cache for deduplication: hash → Promise instance
    promises: HashMap<String, InstanceId>,

    // Semaphore ensuring only one thread accesses Godot APIs at a time
    godot_single_thread: Arc<Semaphore>,

    // Progress tracking
    loading_resources: Arc<AtomicU64>,
    loaded_resources: Arc<AtomicU64>,

    // Optimized asset metadata (pre-processed assets from CDN)
    optimized_data: Arc<OptimizedData>,
}
```

**Key methods**:
- `load_scene_gltf(path, mapping) → Promise` - Load a scene GLTF with colliders
- `load_wearable_gltf(path, mapping) → Promise` - Load a wearable GLTF (no colliders)
- `fetch_texture(path, mapping) → Promise` - Load a texture
- `optimized_asset_exists(hash) → bool` - Check if pre-processed asset available

### ResourceProvider (Rust)

**File**: `lib/src/content/resource_provider.rs`

Manages the disk cache with LRU (Least Recently Used) eviction:

```rust
pub struct ResourceProvider {
    // Root folder for cached files
    cache_folder: PathBuf,

    // Tracks all cached files and their metadata
    existing_files: RwLock<HashMap<String, FileMetadata>>,

    // Maximum cache size (default: 2GB)
    max_cache_size: AtomicI64,

    // Prevents duplicate concurrent downloads of same file
    pending_downloads: RwLock<HashMap<String, Arc<Notify>>>,

    // HTTP client for downloads
    client: Client,

    // Limits concurrent downloads (default: 32)
    semaphore: Arc<Semaphore>,
}
```

**Key responsibilities**:
- Download files with concurrency limiting
- Deduplicate in-flight downloads (if same file requested twice, second waits for first)
- Track file access times for LRU eviction
- Automatically evict old files when cache exceeds size limit

### GLTF Loader (Rust)

**File**: `lib/src/content/gltf.rs`

Three specialized loaders for different GLTF types:

```rust
// Scene GLTF: Full processing with collision shapes
pub async fn load_and_save_scene_gltf(...) -> Result<String, anyhow::Error>

// Wearable GLTF: Basic processing, NO collision shapes
pub async fn load_and_save_wearable_gltf(...) -> Result<String, anyhow::Error>

// Emote GLTF: Animation extraction and processing
pub async fn load_and_save_emote_gltf(...) -> Result<String, anyhow::Error>
```

---

## Loading APIs

### Scene GLTF Loading

Used by `gltf_container.gd` to load 3D models in Decentraland scenes.

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                          SCENE GLTF LOADING FLOW                            │
└─────────────────────────────────────────────────────────────────────────────┘

gltf_container.gd                    ContentProvider                      Disk
       │                                    │                               │
       │  load_scene_gltf(path, mapping)    │                               │
       │   → Returns Promise immediately    │                               │
       │───────────────────────────────────▶│                               │
       │                                    │                               │
       │                                    │  (Background Thread)          │
       │                                    │  Check: {hash}.scn exists?    │
       │                                    │──────────────────────────────▶│
       │                                    │                               │
       │                                    │  Cache HIT:                   │
       │                                    │  Touch file, resolve promise  │
       │                                    │◀──────────────────────────────│
       │                                    │                               │
       │                                    │  Cache MISS:                  │
       │                                    │  1. Download GLTF + deps      │
       │                                    │  2. Load into Godot           │
       │                                    │  3. Process textures          │
       │                                    │  4. Create collision shapes   │
       │                                    │  5. Save as .scn              │
       │                                    │──────────────────────────────▶│
       │                                    │                               │
       │  Promise resolves with scene_path  │                               │
       │◀───────────────────────────────────│                               │
       │                                    │                               │
       │  ResourceLoader.load_threaded()    │                               │
       │───────────────────────────────────────────────────────────────────▶│
       │                                    │                               │
       │  PackedScene.instantiate()         │                               │
       │◀───────────────────────────────────────────────────────────────────│
```

**GDScript usage** (`gltf_container.gd`):

```gdscript
func _async_load_runtime_gltf():
    var content_mapping := Global.scene_runner.get_scene_content_mapping(dcl_scene_id)

    # Request loading - returns Promise immediately
    var promise = Global.content_provider.load_scene_gltf(dcl_gltf_src, content_mapping)
    if promise == null:
        _finish_with_error()
        return

    # Wait for promise to resolve
    await PromiseUtils.async_awaiter(promise)

    if promise.is_rejected():
        _finish_with_error()
        return

    # Promise data is the path to the cached .scn file
    var scene_path = promise.get_data()

    # Load using Godot's threaded loader (non-blocking)
    ResourceLoader.load_threaded_request(scene_path)
    # ... poll until loaded ...
    var packed_scene = ResourceLoader.load_threaded_get(scene_path)
    var gltf_node = packed_scene.instantiate()
    add_child(gltf_node)
```

### Wearable GLTF Loading

Used by `wearable_loader.gd` for avatar equipment (clothing, accessories, etc.).

Similar to scene loading but:
- **No collision shapes** created (wearables don't need physics)
- Cached with prefix `wearable_` (e.g., `wearable_{hash}.scn`)

```gdscript
# In wearable_loader.gd
func async_load_wearables(wearable_keys: Array, body_shape_id: String) -> Dictionary:
    for wearable_key in wearable_keys:
        var content_mapping = wearable.get_content_mapping()
        var main_file = wearable.get_representation_main_file(body_shape_id)

        # Promise-based loading
        var promise = Global.content_provider.load_wearable_gltf(main_file, content_mapping)
        gltf_promises.push_back(promise)

    # Wait for all promises
    await PromiseUtils.async_all(gltf_promises)

    # Collect results
    for promise in gltf_promises:
        if promise.is_resolved() and not promise.is_rejected():
            var scene_path = promise.get_data()
            _completed_loads[file_hash] = scene_path

    return _completed_loads
```

### Emote GLTF Loading

Emotes have special handling because they contain animations that need processing:

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                          EMOTE SCENE STRUCTURE                              │
└─────────────────────────────────────────────────────────────────────────────┘

Saved to disk as emote_{hash}.scn:
┌─────────────────────────────────────┐
│ EmoteRoot (Node3D)                  │
│ ├── Armature_Prop_{hash} (Node3D)   │  ← Optional prop mesh (e.g., guitar)
│ └── EmoteAnimations (AnimationPlayer)│
│     └── AnimationLibrary ""         │
│         ├── "{hash_suffix}"         │  ← Main avatar animation
│         └── "{hash_suffix}_prop"    │  ← Prop animation (optional)
└─────────────────────────────────────┘
```

**Processing flow** (`gltf.rs:load_and_save_emote_gltf`):

1. Download GLTF and all dependencies
2. Load into Godot using GltfDocument
3. Extract animations via `process_emote_animations()`
4. Create EmoteRoot with:
   - Armature_Prop as child (if present)
   - AnimationPlayer with processed animations
5. Save to disk as `emote_{hash}.scn`

---

## Caching Strategy

### Disk Cache Layout

| Asset Type | Path Pattern | Example |
|------------|--------------|---------|
| Scene GLTF | `{content_folder}{hash}.scn` | `{user_data}/content/bafkrei...abc.scn` |
| Wearable | `{content_folder}wearable_{hash}.scn` | `{user_data}/content/wearable_bafkrei...def.scn` |
| Emote | `{content_folder}emote_{hash}.scn` | `{user_data}/content/emote_bafkrei...ghi.scn` |
| Raw files | `{content_folder}{hash}` | `{user_data}/content/bafkrei...xyz` |

### LRU Eviction

The `ResourceProvider` automatically evicts least-recently-used files when cache exceeds `max_cache_size` (default 2GB):

```rust
// On every file access, "touch" it to update access time
resource_provider.touch_file_async(&scene_path).await;

// Before adding new file, evict if needed
async fn ensure_space_for(&self, file_size: i64) {
    while self.total_size() + file_size > self.max_cache_size {
        self.remove_least_recently_used().await;
    }
}
```

### Deduplication

**Promise caching** prevents loading the same asset twice:

```rust
// In load_scene_gltf():

// Check if we already have a promise for this hash
if let Some(existing) = self.get_cached_promise(&file_hash) {
    // Still loading? Return the same promise
    if !existing.bind().is_resolved() {
        return Some(existing);
    }
    // Already loaded? Check file still exists
    if std::path::Path::new(&scene_path).exists() {
        return Some(existing);  // Cache hit!
    }
    // File was evicted - remove stale promise, re-download
    self.promises.remove(&file_hash);
}

// Create new promise and cache it
let (promise, get_promise) = Promise::make_to_async();
self.cache_promise(file_hash.clone(), &promise);
```

**Download deduplication** in ResourceProvider:

```rust
// If two requests for same file come in, second waits for first
let notify = {
    let mut pending = self.pending_downloads.write().await;
    if let Some(notify) = pending.get(&file_hash) {
        Some(notify.clone())  // Wait for existing download
    } else {
        let notify = Arc::new(Notify::new());
        pending.insert(file_hash.clone(), notify.clone());
        None  // Start new download
    }
};

if let Some(notify) = notify {
    notify.notified().await;  // Wait for download to complete
    return Ok(());
}

// ... download file ...

// Notify waiters
pending_downloads.remove(&file_hash).notify_waiters();
```

---

## Optimized Assets

Pre-processed assets are available from `optimized-assets.dclexplorer.com` for faster loading:

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                        OPTIMIZED ASSET LOADING                              │
└─────────────────────────────────────────────────────────────────────────────┘

1. Check: optimized_asset_exists(hash)?
2. Download: {hash}.zip from CDN
3. Load: ProjectSettings.load_resource_pack(zip_path)
4. Access: ResourceLoader.load("res://glbs/{hash}.tscn")
```

**Benefits**:
- Pre-compressed textures (reduced memory, faster load)
- Pre-baked collision shapes
- Faster loading (no runtime GLTF processing)

**CLI flags** for testing:
- `--only-optimized` - Only load optimized assets, skip if unavailable
- `--only-no-optimized` - Always use runtime processing, ignore optimized

---

## Thread Safety

### The Problem

Godot's scene tree and most APIs are **not thread-safe**. Since content loading happens in background threads (via Tokio), we need careful synchronization.

### The Solution: GodotSingleThreadSafety

A semaphore ensures only one background thread accesses Godot APIs at a time:

```rust
// In thread_safety.rs
pub struct GodotSingleThreadSafety {
    _guard: tokio::sync::OwnedSemaphorePermit,
}

impl GodotSingleThreadSafety {
    pub async fn acquire_owned(ctx: &ContentProviderContext) -> Option<Self> {
        // Wait for exclusive access
        let guard = ctx.godot_single_thread.clone().acquire_owned().await.ok()?;
        // Disable Godot's thread safety checks (we're managing it ourselves)
        set_thread_safety_checks_enabled(false);
        Some(Self { _guard: guard })
    }
}

impl Drop for GodotSingleThreadSafety {
    fn drop(&mut self) {
        // Re-enable thread safety checks
        set_thread_safety_checks_enabled(true);
    }
}
```

**Usage in GLTF loading**:

```rust
// Acquire thread safety guard before using Godot APIs
let _thread_guard = GodotSingleThreadSafety::acquire_owned(&ctx).await?;

// Now safe to use Godot APIs (GltfDocument, PackedScene, etc.)
let mut new_gltf = GltfDocument::new_gd();
// ... process GLTF ...
save_node_as_scene(root_node, &scene_path)?;

// Guard drops here, re-enabling thread safety
```

### Promise Resolution

Promises resolve via `call_deferred` to ensure the callback runs on the main thread:

```rust
fn resolve_promise(get_promise: impl Fn() -> Option<Gd<Promise>>, value: Option<Variant>) {
    if let Some(mut promise) = get_promise() {
        // call_deferred queues this for main thread
        promise.call_deferred("resolve_with_data", &[value]);
    }
}
```

---

## Collision Shape Creation

Scene GLTFs have collision shapes created during processing:

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                          COLLIDER CREATION FLOW                             │
└─────────────────────────────────────────────────────────────────────────────┘

During GLTF Processing (Rust, background thread):
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
1. For each MeshInstance3D in the scene:
   ├── Call mesh_instance.create_trimesh_collision()
   │   └── This creates a child StaticBody3D with CollisionShape3D
   ├── Set collision_layer = 0, collision_mask = 0 (disabled)
   ├── Set process_mode = DISABLED
   ├── Set metadata: dcl_col, invisible_mesh
   └── Enable backface collision for concave shapes

After Instantiation (GDScript, main thread):
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
2. gltf_container.gd calls set_mask_colliders():
   ├── Sets proper collision_layer based on Decentraland SDK
   └── StaticBody3D remains STATIC by default

On Entity Movement (Rust detects transform change):
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
3. If entity with colliders moves:
   ├── Rust emits switch_to_kinematic signal
   └── GDScript changes body mode via PhysicsServer3D
       (KINEMATIC mode allows programmatic movement)
```

**Why this approach?**
- StaticBody3D is more efficient for non-moving objects
- Only switch to KINEMATIC when movement is detected
- Masks start at 0 to prevent collisions during scene setup

---

## Avatar Wearable Loading

Avatar loading uses `WearableLoader` for efficient batch loading.

### Loading Flow

```gdscript
# In avatar.gd
func async_load_wearables():
    # 1. Load all wearables in parallel via WearableLoader
    await wearable_loader.async_load_wearables(wearable_keys, body_shape_id)

    # 2. Get each wearable node from cache
    for category in wearables_by_category:
        var file_hash = Wearables.get_item_main_file_hash(wearable, body_shape)
        var obj = await wearable_loader.async_get_wearable_node(file_hash)

        # 3. Reparent mesh children to avatar skeleton
        for skeleton_3d in obj.find_children("Skeleton3D"):
            for child in skeleton_3d.get_children():
                skeleton_3d.remove_child(child)
                child.set_owner(null)  # Clear owner for reparenting
                body_shape_skeleton_3d.add_child(child)

        # 4. Free the now-empty container
        obj.queue_free()
```

**Key points**:
- Each `async_get_wearable_node()` returns a **fresh instance** from `PackedScene.instantiate()`
- Mesh children are **reparented** to the avatar's skeleton (not duplicated)
- Original container is freed after reparenting

### WearableLoader

`godot/src/decentraland_components/avatar/wearables/wearable_loader.gd`

Batches wearable loading with promise-based API:

```gdscript
func async_load_wearables(wearable_keys: Array, body_shape_id: String) -> Dictionary:
    # Start all GLTF loads in parallel
    for wearable_key in wearable_keys:
        var promise = Global.content_provider.load_wearable_gltf(main_file, content_mapping)
        gltf_promises.push_back(promise)

    # Wait for all to complete
    await PromiseUtils.async_all(gltf_promises)

    # Return mapping: file_hash → scene_path
    return _completed_loads

func async_get_wearable_node(file_hash: String) -> Node3D:
    var scene_path = _completed_loads.get(file_hash)

    # Use threaded loading for non-blocking disk read
    ResourceLoader.load_threaded_request(scene_path)
    # ... poll until loaded ...
    var packed_scene = ResourceLoader.load_threaded_get(scene_path)

    return packed_scene.instantiate()
```

---

## Error Handling

| Error | Location | Recovery |
|-------|----------|----------|
| File not in content mapping | ContentProvider | Returns `null` Promise |
| Download failed | ResourceProvider | Promise rejects with error message |
| GLTF parse error | gltf.rs | Promise rejects with error details |
| Save failed | scene_saver.rs | Promise rejects with error |
| Load timeout | gltf_container.gd | Timer fires, finishes with error state |

---

## Performance Optimizations

1. **Parallel Downloads**: Up to 32 concurrent downloads (configurable in ResourceProvider)
2. **Throttled Instantiation**: `MAX_CONCURRENT_LOADS = 10` in gltf_container.gd to prevent frame drops
3. **Priority Queue**: Current scene's assets prioritized via `pending_load_queue`
4. **Texture Quality**: Textures resized based on `DclConfig.texture_quality` setting
5. **iOS Compression**: Creates compressed textures on iOS for memory efficiency
6. **Threaded ResourceLoader**: All `.scn` files loaded via `ResourceLoader.load_threaded_*` for non-blocking I/O

---

## Files Reference

| File | Purpose |
|------|---------|
| `lib/src/content/content_provider.rs` | Central content loading node, exposes GDScript API |
| `lib/src/content/resource_provider.rs` | Disk cache management, downloads, LRU eviction |
| `lib/src/content/gltf.rs` | GLTF loading and processing |
| `lib/src/content/scene_saver.rs` | PackedScene saving utilities |
| `lib/src/content/thread_safety.rs` | Godot API thread safety |
| `lib/src/content/texture.rs` | Texture loading and processing |
| `lib/src/content/audio.rs` | Audio loading |
| `godot/src/decentraland_components/gltf_container.gd` | Scene GLTF component |
| `godot/src/decentraland_components/avatar/wearables/wearable_loader.gd` | Wearable batch loading |

---

## Debugging

Enable detailed logging:

```bash
RUST_LOG="warn,dclgodot::content=debug" cargo run -- run
```

Key log messages:
- `Scene GLTF cache HIT: {hash}` - Using cached scene
- `Scene GLTF cache MISS: {hash}` - Processing new scene
- `Wearable GLTF cache HIT/MISS` - Wearable loading status
- `Emote GLTF cache HIT/MISS` - Emote loading status
- `Scene GLTF cache EVICTED: {hash}` - Re-downloading after cache eviction
- `Failed to...` - Error conditions
