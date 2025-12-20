# Scene Architecture

This document describes the architecture of scene loading, execution, and unloading in the Decentraland Godot Explorer.

## Overview

The scene system is a multi-threaded architecture that combines:
- **GDScript (Godot)**: Scene fetching, content downloading, and UI coordination
- **Rust**: Scene management, CRDT state handling, and thread orchestration
- **JavaScript/V8 (Deno)**: Decentraland SDK scene execution

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                              MAIN THREAD (Godot)                            │
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                             │
│  ┌─────────────────────┐     ┌─────────────────────┐                       │
│  │   SceneFetcher.gd   │────▶│   SceneRunner.rs    │                       │
│  │  (Content Download) │     │  (Scene Manager)    │                       │
│  └─────────────────────┘     └──────────┬──────────┘                       │
│                                         │                                   │
│                              ┌──────────┴──────────┐                       │
│                              │   Scene Processing   │                       │
│                              │  (CRDT, Components)  │                       │
│                              └──────────┬──────────┘                       │
│                                         │                                   │
└─────────────────────────────────────────┼───────────────────────────────────┘
                                          │
                    ┌─────────────────────┼─────────────────────┐
                    │                     │                     │
                    ▼                     ▼                     ▼
┌───────────────────────────┐ ┌───────────────────────────┐ ┌───────────────┐
│   Scene Thread 0 (V8)     │ │   Scene Thread 1 (V8)     │ │   Thread N    │
│  ┌─────────────────────┐  │ │  ┌─────────────────────┐  │ │               │
│  │  Deno Runtime       │  │ │  │  Deno Runtime       │  │ │     ...       │
│  │  ┌───────────────┐  │  │ │  │  ┌───────────────┐  │  │ │               │
│  │  │ SDK7 Scene JS │  │  │ │  │  │ SDK7 Scene JS │  │  │ │               │
│  │  └───────────────┘  │  │ │  │  └───────────────┘  │  │ │               │
│  └─────────────────────┘  │ │  └─────────────────────┘  │ │               │
└───────────────────────────┘ └───────────────────────────┘ └───────────────┘
```

## Key Components

### 1. SceneFetcher (GDScript)

**File**: `godot/src/logic/scene_fetcher.gd`

Responsible for:
- Monitoring player position changes
- Coordinating with `SceneEntityCoordinator` to determine which scenes to load
- Downloading scene assets (JS, CRDT files) from content servers
- Initiating scene spawning via `SceneRunner`
- Managing floating island generation

```gdscript
# Key flow
func _process(_dt):
    scene_entity_coordinator.update()
    if version != last_version_updated:
        await _async_on_desired_scene_changed()

func async_load_scene(scene_entity_id, scene_entity_definition):
    # Download main.js and main.crdt
    # Call Global.scene_runner.start_scene()
```

### 2. SceneRunner / SceneManager (Rust)

**File**: `lib/src/scene_runner/scene_manager.rs`

The central orchestrator that:
- Creates and manages scene instances
- Handles inter-thread communication
- Processes CRDT state updates
- Coordinates scene lifecycle (Alive → ToKill → KillSignal → Dead)

```rust
pub struct SceneManager {
    scenes: HashMap<SceneId, Scene>,
    sorted_scene_ids: Vec<SceneId>,      // Active scenes
    dying_scene_ids: Vec<SceneId>,        // Scenes being killed
    main_receiver_from_thread: Receiver<SceneResponse>,
    // ...
}
```

### 3. Scene (Rust)

**File**: `lib/src/scene_runner/scene.rs`

Represents a single scene instance:

```rust
pub struct Scene {
    pub scene_id: SceneId,
    pub dcl_scene: DclScene,              // Thread handle + channels
    pub state: SceneState,                 // Alive, ToKill, KillSignal, Dead
    pub current_dirty: Dirty,              // Pending CRDT updates
    pub enqueued_dirty: Vec<Dirty>,        // Queued updates
    // ... components, materials, etc.
}

pub enum SceneState {
    Alive,              // Running normally
    ToKill,             // Kill requested, signal pending
    KillSignal(i64),    // Kill signal sent, waiting for thread
    Dead,               // Thread finished, ready for cleanup
}
```

### 4. DclScene (Rust)

**File**: `lib/src/dcl/mod.rs`

Holds the thread handle and communication channels:

```rust
pub struct DclScene {
    pub scene_id: SceneId,
    pub scene_crdt: SharedSceneCrdtState,
    pub main_sender_to_thread: Sender<RendererResponse>,
    pub thread_join_handle: JoinHandle<()>,
}
```

### 5. Scene Thread (Rust + JavaScript)

**File**: `lib/src/dcl/js/mod.rs`

Each scene runs in its own thread with:
- A Tokio runtime for async operations
- A Deno/V8 JavaScript runtime
- The scene's SDK code

```rust
pub fn scene_thread(
    thread_receive_from_main: Receiver<RendererResponse>,
    scene_crdt: Arc<Mutex<SceneCrdtState>>,
    spawn_dcl_scene_data: SpawnDclSceneData,
) {
    // 1. Load main.crdt if present
    // 2. Create Deno runtime
    // 3. Execute scene JS code
    // 4. Run onStart()
    // 5. Main loop: onUpdate() + check SceneDying
    // 6. Cleanup and exit
}
```

## Communication Channels

### Main Thread → Scene Thread

```
RendererResponse::Ok { dirty_crdt_state, incoming_comms_message }
RendererResponse::Kill
```

Channel: `tokio::sync::mpsc::Sender<RendererResponse>` (capacity: 1)

### Scene Thread → Main Thread

```
SceneResponse::Ok { scene_id, dirty_crdt_state, logs, rpc_calls, ... }
SceneResponse::Error(scene_id, message)
SceneResponse::RemoveGodotScene(scene_id, logs)
```

Channel: `std::sync::mpsc::SyncSender<SceneResponse>` (shared by all scenes)

## Scene Lifecycle

### 1. Scene Loading

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                              SCENE LOADING                                  │
└─────────────────────────────────────────────────────────────────────────────┘

1. Player enters new parcel area
        │
        ▼
2. SceneEntityCoordinator detects new scenes needed
        │
        ▼
3. SceneFetcher.async_load_scene()
   ├── Download main.js
   ├── Download main.crdt (optional)
   └── Download optimized assets (optional)
        │
        ▼
4. SceneFetcher._on_try_spawn_scene()
        │
        ▼
5. SceneRunner.start_scene()
   ├── Generate new SceneId
   ├── Create channels (main_sender_to_thread, thread_sender_to_main)
   ├── Spawn scene thread
   └── Create Scene struct (state = Alive)
        │
        ▼
6. Scene Thread starts
   ├── Load main.crdt → process CRDT messages
   ├── Create Deno/V8 runtime
   ├── Execute scene JavaScript
   ├── Call onStart()
   └── Enter main loop
```

### 2. Scene Execution

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                             SCENE EXECUTION                                 │
└─────────────────────────────────────────────────────────────────────────────┘

Each frame:

MAIN THREAD                              SCENE THREAD
     │                                        │
     │  1. receive_from_thread()              │
     │  ◄─────── SceneResponse::Ok ───────────│
     │                                        │
     │  2. _process_scene()                   │
     │     ├── Update components              │
     │     ├── Process CRDT state             │
     │     └── Prepare RendererResponse       │
     │                                        │
     │  3. send_to_thread()                   │
     │     RendererResponse::Ok ─────────────▶│
     │                                        │
     │                                        │  4. op_crdt_send_to_renderer()
     │                                        │     ├── Receive dirty state
     │                                        │     ├── Process events
     │                                        │     └── Return CRDT data
     │                                        │
     │                                        │  5. run_script("onUpdate")
     │                                        │
     │                                        │  6. Send SceneResponse::Ok
     │                                        │
```

### 3. Scene Killing

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                              SCENE KILLING                                  │
└─────────────────────────────────────────────────────────────────────────────┘

1. SceneFetcher calls Global.scene_runner.kill_scene(scene_id)
        │
        ▼
2. SceneRunner.kill_scene()
   ├── Set scene.state = ToKill
   └── Add to dying_scene_ids
        │
        ▼
3. scene_runner_update() - dying loop iteration
        │
        ├── State: ToKill
        │   ├── Send RendererResponse::Kill to thread
        │   └── Transition to KillSignal(timestamp)
        │
        ▼
4. Scene Thread receives Kill (channel close or Kill message)
   ├── Set SceneDying(true)
   ├── Break main loop
   ├── Send RemoveGodotScene
   └── Thread exits
        │
        ▼
5. scene_runner_update() - next iteration
        │
        ├── State: KillSignal
        │   ├── Check thread_join_handle.is_finished()
        │   └── If finished → transition to Dead
        │
        ▼
6. receive_from_thread() receives RemoveGodotScene
   ├── Set scene.state = Dead
   └── Add to dying_scene_ids (if not present)
        │
        ▼
7. scene_runner_update() - next iteration
        │
        ├── State: Dead
        │   └── Add to scene_to_remove set
        │
        ▼
8. Cleanup
   ├── Remove from all lists
   ├── Free Godot nodes
   ├── Join thread
   └── Emit scene_killed signal
```

## Error Handling

### Thread-Level Errors

| Error | Location | Cause | Recovery |
|-------|----------|-------|----------|
| Script load error | js/mod.rs:287-289 | Invalid JS file | Thread exits, sends Error |
| onStart error | js/mod.rs:296-299 | JS execution fails | Thread exits, sends Error |
| Too many onUpdate errors | js/mod.rs:351-359 | Repeated JS errors | Logs warning (see Issue #1) |

### Manager-Level Errors

| Error | Location | Cause | Recovery |
|-------|----------|-------|----------|
| Scene closed without kill signal | scene_manager.rs:496-499 | Thread died unexpectedly | Scene marked for removal |
| Error sending kill signal | scene_manager.rs:525-530 | Channel full/closed | Logs error, retries next frame |
| Timeout killing scene | scene_manager.rs:539-543 | Thread not responding | Logs error (see Issue #2) |

## CRDT State Management

The CRDT (Conflict-free Replicated Data Type) system synchronizes state between the main thread and scene threads:

```
┌────────────────────────────────────────────────────────────────────────────┐
│                           CRDT STATE FLOW                                  │
└────────────────────────────────────────────────────────────────────────────┘

Scene Thread                              Main Thread
     │                                         │
     │  Scene modifies entities/components     │
     │  via SDK API calls                      │
     │         │                               │
     │         ▼                               │
     │  SceneCrdtState accumulates changes     │
     │         │                               │
     │         ▼                               │
     │  take_dirty() → DirtyCrdtState          │
     │         │                               │
     │         ▼                               │
     │  SceneResponse::Ok { dirty_crdt_state } │
     │  ──────────────────────────────────────▶│
     │                                         │  Process components
     │                                         │  Update Godot nodes
     │                                         │  Prepare renderer response
     │                                         │
     │                                         │  DirtyCrdtState (from Godot)
     │  ◀──────────────────────────────────────│
     │  RendererResponse::Ok { dirty_crdt_state }
     │         │
     │         ▼
     │  Merge into SceneCrdtState
     │  (player position, input, etc.)
```

## Fixed Issues

The following issues have been identified and fixed:

### Fix 1: "Too many errors" now properly stops the scene

**Location**: `lib/src/dcl/js/mod.rs:351-361`

**Problem**: The code logged "shutting down" but didn't actually break the loop.

**Solution**: Added `break` after the error log to properly stop the scene when too many errors occur without renderer interaction.

```rust
if reported_error_filter == 10 && !communicated_with_renderer {
    tracing::error!("too many errors without renderer interaction: shutting down");
    break;  // Now properly exits the loop
}
```

### Fix 2: Script errors now send proper cleanup message

**Location**: `lib/src/dcl/js/mod.rs:142-149, 297-301, 308-311`

**Problem**: When script load or onStart failed, thread returned immediately without sending `RemoveGodotScene`, causing "scene closed without kill signal" errors.

**Solution**: Created a helper function `send_remove_godot_scene()` that is now called from all exit points:

```rust
/// Helper to send RemoveGodotScene response to the main thread.
fn send_remove_godot_scene(state: &Rc<RefCell<OpState>>, scene_id: SceneId) {
    let mut op_state = state.borrow_mut();
    let logs = op_state.take::<SceneLogs>();
    let sender = op_state.borrow_mut::<SyncSender<SceneResponse>>();
    let _ = sender.send(SceneResponse::RemoveGodotScene(scene_id, logs.0));
}

// Used in error cases and normal exit:
Err(e) => {
    tracing::error!("[scene thread {scene_id:?}] script load error: {}", e);
    send_remove_godot_scene(&state, scene_id);
    return;
}
```

### Fix 3: Timeout killing now forces V8 termination

**Location**: `lib/src/scene_runner/scene_manager.rs:540-563`

**Problem**: After 10 seconds, the code only logged an error but never forced the thread to stop.

**Solution**: Now uses the V8 isolate handle to force-terminate execution after timeout:

```rust
if elapsed_from_kill_us > 10 * 1e6 as i64 {
    tracing::error!("timeout killing scene {:?}, forcing V8 termination", scene_id);

    // Use the V8 isolate handle to force-terminate execution
    if let Ok(handles) = crate::dcl::js::VM_HANDLES.lock() {
        if let Some(handle) = handles.get(scene_id) {
            handle.terminate_execution();
        }
    }

    scene.state = SceneState::Dead;
}
```

**Additional**: VM_HANDLES entries are now properly cleaned up when scenes are removed.

## Performance Considerations

1. **Channel Capacity**: Main→Thread channel has capacity 1, preventing queue buildup
2. **Scene Sorting**: Scenes are sorted by distance for priority processing
3. **Time Budget**: Each frame has limited time for scene processing (`MIN_TIME_TO_PROCESS_SCENE_US`)
4. **Partial Processing**: Scene updates can be spread across multiple frames

## Thread Safety

- `SceneCrdtState`: Protected by `Arc<Mutex<...>>`
- `VM_HANDLES`: Global map of V8 isolate handles for emergency termination
- Channels: All use thread-safe primitives (mpsc, tokio::sync::mpsc)

## Debugging

Enable detailed logging:
```bash
RUST_LOG="warn,dclgodot::scene_runner=debug,dclgodot::dcl::js=debug" cargo run -- run
```

Key log messages:
- `breaking from the thread SceneId(X)` - Normal scene shutdown
- `exiting from the thread SceneId(X)` - Thread cleanup complete
- `scene closed without kill signal` - Unexpected thread death
- `error sending kill signal to thread` - Channel issue
- `timeout killing scene` - Thread not responding
