# Profile profundo Android — 2026-05-06

Captura: `bench-results/profiles/android-profile-deep-baseline-20260506T223819Z/`.
Bench: `--gp-benchmark` con preview server local del commit pinned de
Genesis-Plaza-2025; tag `profile-deep-baseline`. Build: Rust dev-release +
Godot DEBUG export template (libgodot_android.so unstripped, build-id
`0f54f0f5e6cbfcd1`). Símbolos resueltos vía `llvm-symbolizer` directo sobre
los offsets más calientes (binary_cache_builder.py de simpleperf no
matcheó por falta de build_ids embedidos en perf.data — limitación
conocida).

## Métricas del bench (10s sample window, 296 frames)

| Métrica | Valor |
|---|---|
| FPS mean | 21.6 |
| frame_proc_ms mean | 60.0 |
| draws mean | 869 |
| build_mode | debug (Godot debug template, ~10-15% slower than release) |

CRDT throughput (`crdt_metrics`):

| Counter | Valor | Por frame |
|---|---|---|
| send_bytes (V8 → Rust) | 437 KB | **1.5 KB** |
| send_ops | 297 | 1 |
| recv_bytes (Rust → V8) | **7.58 MB** | **25.6 KB** |
| recv_ops | 296 | 1 |
| dirty_lww_entries | **153,507** | **519** |
| dirty_gos_entries | 888 | 3 |

Per-state CPU breakdown (top 5, suma a ~7 ms/frame ≈ 12% del frame_proc):

| State | Total (μs / 10s) | Per frame (μs) |
|---|---|---|
| MeshRenderer | 412,888 | 1,395 |
| TransformAndParent | 354,030 | 1,196 |
| Billboard | 152,500 | 515 |
| Tween | 146,736 | 496 |
| Material | 120,164 | 406 |

## Reparto de CPU por thread

| Thread | % de eventos | Cycles |
|---|---|---|
| **VkThread** | **52.9 %** | 17.91 G |
| **Thread-16** (scene_runner main) | **33.7 %** | 11.41 G |
| V8 DefaultWorker | 6.4 % | 2.16 G |
| mali-cmar-backe (GPU driver) | 3.0 % | 1.03 G |
| Resto | 3.5 % | <1.2 G |

VkThread + Thread-16 = **86.6 %** del trabajo CPU. Esos son los dos a atacar.

## Top funciones por self-time (symbolicated, % medidos)

### VkThread (52.9 % del total) — main rendering thread

| % thread | % total | Función | Archivo |
|---|---|---|---|
| **6.55** | 3.46 | `Node::_notification(int)` | scene/main/node.cpp |
| **4.53** | 2.40 | `longest_match` (3 lines, zlib hot loop) | thirdparty/zlib/deflate.c:1449 |
| 3.44 | 1.82 | `abs[abi:nn190000](double)` | libc++ stdlib.h (math, posiblemente inlined hot) |
| 2.54 | 1.34 | `abs(double)` (otro callsite) | libc++ |
| 2.22 | 1.17 | `RenderingDeviceGraph::_run_draw_list_command` | servers/rendering/rendering_device_graph.cpp:894 |
| 1.84 | 0.97 | `longest_match` (línea 1450) | zlib/deflate.c |
| 1.56 | 0.83 | `GDExtensionMethodBind::call` | core/extension/gdextension.cpp:108 |
| 1.55 | 0.82 | `__aarch64_cas1_acq` (atomic) | aarch64 builtin |
| 1.44 | 0.76 | `Node3D::_propagate_transform_changed` (línea 115) | scene/3d/node_3d.cpp |
| 1.36 | 0.72 | `RenderingDeviceDriverVulkan::command_queue_execute_and_present` | drivers/vulkan |
| 1.28 | 0.68 | `Node3D::_propagate_transform_changed` (línea 115:16) | (otro callsite) |
| 1.16 | 0.61 | `JavaClass::_call_method` | platform/android/java_class_wrapper.cpp |

### Thread-16 (33.7 % del total) — scene_runner / V8 host

| % thread | % total | Función | Archivo |
|---|---|---|---|
| **4.25** | 1.43 | `operator new(unsigned long)` | libc++ src/new.cpp:47 |
| **4.18** | 1.41 | `v8::internal::Heap::AllocateExternalBackingStore` | v8/heap/heap.cc:3217 |
| 2.10 | 0.71 | `v8::internal::Malloced::operator new` | v8/utils/allocation.cc:96 |
| 1.56 | 0.53 | `Builtins_FindOrderedHashMapEntry` | V8 (Map.has/get) |
| 1.16 | 0.39 | `__aarch64_ldadd4_acq_rel` | atomic |
| 1.08 | 0.36 | `Builtins_LoadIC_Megamorphic` | V8 (property access poly cache) |
| 0.92 | 0.31 | `EphemeronRememberedSet::RecordEphemeronKeyWrite` | v8/heap (WeakMap GC barrier) |
| 0.88 | 0.30 | `__aarch64_cas4_acq` | atomic |
| 0.84 | 0.28 | `Builtins_LoadIC_Noninlined` | V8 |
| 0.79 | 0.27 | `__do_rehash` (V8 hash) | libc++ |
| 0.77 | 0.26 | `Builtins_WeakCollectionSet` | V8 (WeakMap.set) |
| 0.70 | 0.24 | `__hash_table::__emplace_unique_key_args` | libc++ |
| 0.68 | 0.23 | `Builtins_MapPrototypeSet` | V8 (Map.set) |

### Agregado por categoría (% total CPU del proceso)

| Categoría | % total | Notas |
|---|---|---|
| **Node tree propagation/notification** | **~5.0 %** | `Node::_notification` + `Node3D::_propagate_transform_changed` (2 callsites) — escala con node count |
| **zlib compression `longest_match`** | **~4.0 %** | Anómalo en VkThread; probable texture/swapchain compression. Investigar |
| **abs(double) math** | **~3.2 %** | Bizarro al top — probablemente inlined desde algún hot loop matemático |
| **V8 heap allocations** | **~3.6 %** | `operator new` + `Heap::AllocateExternalBackingStore` + `Malloced::operator new` |
| **Vulkan submit + draw cmd** | **~2.5 %** | command_queue + draw_list — hardware path |
| **V8 Map/WeakMap/property ops** | **~2.5 %** | `Builtins_*` family |
| **Atomic ops (lock contention)** | **~2.0 %** | `__aarch64_cas*` + `__aarch64_ldadd*` |
| **JNI bridge** | **~1.2 %** | `JavaClass::_call_method` (Android API polling?) |
| **V8 GC + write barriers** | **~1.5 %** | Ephemeron + Scavenger |
| **GDExtension dispatch** | **~0.8 %** | C++↔Rust bridge cost |

## Hallazgos

### 1. VkThread NO es solo Vulkan submission

El top-1 `longest_match` está en zlib's deflate. **zlib en el thread de
render es anómalo** — probablemente es texture compression/decompression al
upload, o snapshot de buffers Vulkan, o screenshot encode. Vale investigar
qué pipeline lo dispara: si es snapshot del bench (que solo corre 1 vez
al final), no debería estar en el sample window. Si es texture transcode
in-flight, hay buckets para optimizar.

### 2. Node3D transform propagation domina el "pure Godot" subset

`Node3D::_propagate_transform_changed` (#3) y `Node3D::get_global_transform`
(#6) en VkThread top. Ambos escalan con node count. Con 19,604 nodos
totales (post Timer-coalescer), esto es proporcional. **B1 del plan
(drop entity Node3D) está data-supported** — bajar de ~20k a ~15k nodos
debería bajar VkThread perceptiblemente.

### 3. Thread-16 ES V8 + heap allocations, NO scene_runner Rust

Top 10 de Thread-16 = 100% V8 internals. Cero funciones de
`scene_runner/update_scene.rs` o `dcl/crdt/`. Eso refuerza el hallazgo
del per-state instrumentation: el state loop es ~12% del frame_proc, el
resto es V8 ejecutando JS.

Lo que V8 está haciendo, según los Builtins:
- **`Map`/`WeakMap` operations** dominan — SDK7 lleva tracking de entidades
  en Maps. 519 dirty_lww_entries/frame implica 519+ Map operations/frame
  (set / get / has).
- **Heap allocations** constantes (`operator new`, `AllocateExternalBackingStore`,
  `Malloced::operator new`). Cada CRDT message recibido (25.6 KB/frame) crea
  ArrayBuffer / Uint8Array en V8 → trigger de GC + write barriers.
- **GC overhead**: `ScavengerCollector::ClearOldEphemerons` aparece arriba —
  el GC corre seguido por presión.

### 4. CRDT round-trip: el Rust → V8 es 17x más grande que V8 → Rust

- V8 → Rust: 1.5 KB/frame.
- Rust → V8: 25.6 KB/frame.

V8 está RECIBIENDO mucho más data del que MANDA. Eso son 7.5 MB en 10s pasados
desde Rust hasta V8 vía `op_crdt_recv_from_renderer`. Cada uno de esos bytes
llega como ArrayBuffer en V8, allocando + GCing.

**B5 del plan (CRDT optimization) está data-supported con números:** reducir
recv_bytes pondría menos presión sobre V8 heap.

### 5. JavaClass::_call_method (JNI bridge) en VkThread top

Java↔C++ calls per frame en el thread de render. Puede ser Android API
polling (sensor, accelerometer, audio). Inesperado en VkThread; vale
investigar qué Godot Android wrapper lo dispara.

## Hallazgo de hog accionable: impostor PNG cache (zlib en VkThread)

El callgraph inclusive resolvió el misterio de **`longest_match` (zlib) y `abs(double)` en VkThread**:

```
SceneTree::_process(true)
  → Object::notification → _gdvirtual__process_call
    → GDScriptInstance::callp (GDScript dispatch)
      → impostor_capturer.gd::_async_process_next
        → Global.avatars.set_impostor_texture (Rust GDExtension)
          → save_cached_texture (lib/src/avatars/avatar_scene.rs:1051)
            → Image::save_png(path)  ← bloquea el thread con zlib + math
              → ResourceSaverPNG::save_image
                → PNGDriverCommon::image_to_png
```

`avatar_scene.rs:1054` llama `image.clone().save_png(&path)` síncrono, encadenado a zlib. **~7 % del CPU total** se va ahí.

Por qué pasa con comms held (no remote avatars): el avatar LOCAL también dispara `ImpostorCapturer.request_capture(self)` desde `avatar.gd:535/957/1172`. Una vez la captura está pendiente, `_async_process_next` vacía la cola y dispara el save_png en una sola llamada que bloquea el frame.

**Fix candidatos (ordenados por effort):**

1. **Mover `save_png` a worker thread (tokio task).** ~30 lines en `avatar_scene.rs`. Image clone ya está en CPU memory; solo el zlib encode + write se hace async. Spawn dentro de `save_cached_texture`. Cero cambio de behaviour observable.
2. **Feature flag `disable_impostor_disk_cache`** para benchmarks. Si la session no quiere persistencia entre runs, skip entero. Trivial.
3. **Cambiar formato a JPEG / WEBP.** PNG (con zlib) es lossless pero CPU-pesado. JPEG con calidad alta sería ~5x más rápido. Más invasivo (texture format compatibility).

Mi voto: **fix #1 (async)**. Mantiene el behaviour, ataca el hog, y es validable con bench A/B.

## Recomendación de Phase B (decision)

Tres paths data-supported, NO uno solo. Por ROI esperado:

### Path 1 (highest ROI, doable): reducir CRDT recv_bytes
**Por qué primero:** ataca el origen de la presión sobre Thread-16 (V8 heap +
GC). Cada KB que ahorramos del Rust → V8 reduce ArrayBuffer allocations,
reduce GC frecuencia, reduce write barriers (EphemeronRememberedSet),
reduce el `LoadIC_Megamorphic` (porque V8 ve menos shapes nuevos).

Concretos:
- Investigar QUÉ componentes generan los 153,507 dirty_lww_entries en 296
  frames (~519/frame). Probable culpable: Transform updates de avatares
  remotos / GltfNodeModifiers / Tween. Top sospechoso: GP tiene **comms held**
  (no avatares remotos), pero las locales (player + camera) sí actualizan
  transform cada frame.
- Si es Transform: ver si se puede dedupear. Mismo player_transform 60 veces
  por segundo con cambios subpíxel = waste.
- Pool de buffers en `DclWriter` (`lib/src/dcl/serialization/writer.rs`) para
  que no allocate Vec<u8> por op call.

Effort: 1-3 días investigación + 2-5 días implementación.
Expected: **−15-30% Thread-16 CPU** ≈ **+3-6 FPS Android**.

### Path 2 (data-supported, riskier): drop entity Node3D (Step 6 del plan original)

Node3D::_propagate_transform_changed + Node3D::get_global_transform en VkThread
top. Drop ~3-5k Node3D = menos propagation work.

Effort: 5-10 días (hay que tocar pointer_events_system para que collider
lleve metas dcl_scene_id/dcl_entity_id directamente, no via parent walk).
Expected: **−10-20% VkThread CPU** ≈ **+2-4 FPS Android**.

### Path 3 (investigación, antes de comprometer): zlib en VkThread

Algo está deflateando en el render thread. Vale 1 día de investigación
antes de cualquier opt — si es un bug de Godot upstream o un setting de
texture compression que se puede toggle off, puede ser **un win gratis**.

## Hog #2 cuantificación expandida (post call-graph slicing)

Vía `python3 /tmp/callp_callees.py stacks.folded` derivamos quién llama a quién
por offset. Resolución de los 25 offsets unsymbolicated más calientes:

| Offset | Función | Self samples |
|---|---|---|
| `+0x12849b8` | `OS_Android::main_loop_iterate(bool*)` | 8713 |
| `+0x43b2978` | `Object::_notification_forward(int)` | 6170 |
| `+0x236b8f4` | `SceneTree::_process(bool)` | 5928 |
| `+0x165cc5c` | `GDScriptInstance::callp(...)` | **5864** |
| `+0x12e4380` | `Main::iteration()` | 4433 |
| `+0x40570c0` | `Variant::callp(...)` | 4024 |
| `+0x43b217c` | `Object::callp(...)` | 3899 |
| `+0x167f9ac` | `GDScriptFunction::call(...)` | 3456 |
| `+0x23267dc` | `Node::_gdvirtual__process_call(double)` | 1854 |
| `+0x2d9776c` | `AnimationNode::process(...)` | 1667 |
| `+0x2d96468` | `AnimationNode::_pre_process(...)` | 1365 |
| `+0x2d95360` | `AnimationNode::blend_input(...)` | 1081 |
| `+0x29a31f0` `+0x2dcc798` `+0x2a30964` `+0x24f9ef8` | `Node::_notification_forwardv(int)` (×4 callsites) | 278+214+41+19 |
| `+0x2d210d0` | `AnimationNodeBlendTree::_process(...)` | 406 |
| `+0x2d1dae8` | `AnimationNodeOutput::_process(...)` | 383 |
| `+0x2d14474` | `AnimationNodeAdd2::_process(...)` | 194 |
| `+0x2d157a8` | `AnimationNodeBlend2::_process(...)` | 187 |
| `+0x12be33c` | `JNISingleton::callp(...)` | 142 |
| `+0x4022250` | `Callable::callp(...)` | 43 |

### Call-graph slicing del dispatch chain

```
NOTIFICATION_PROCESS broadcast (SceneTree)
  → Object::_notification_forward (6170)
    → Node::_notification_forwardv (4 callsites) (~552)
      → Node::_gdvirtual__process_call (1854, 100% en este path)
        → GDScriptInstance::callp (5864 total)
          → GDScriptFunction::call (3456 + 1168 = 4624)
            → Variant::callp (4024)
              → Object::callp (3886/3899 = 99.7%)
                → GDScriptInstance::callp (RECURSIÓN: GDScript llama a otro método/property)

AnimationTree advance (141 instances ticking)
  → AnimationNode::_pre_process (1365)
    → AnimationNode::process (1667)
      → AnimationNode::blend_input (1081)
        → {AnimationNodeBlendTree,Output,Add2,Blend2}::_process (406+383+194+187 = 1170)
```

**Atribución del costo de GDScript dispatch (5864 samples):**
- 1846 (31%) → directo desde `Node::_gdvirtual__process_call` = `_process(_delta)` callbacks
- 4018 (69%) → desde `Object::callp` = property/method calls DESDE GDScript (incluye animation_tree.advance, .global_transform = ..., y similares)

**Atribución del costo de AnimationNode (4113 samples):**
- 100% se origina en AnimationTree advance, que se dispara via NOTIFICATION_PROCESS
  de 141 AnimationPlayers (`OBJECT_NODE_COUNT.AnimationPlayer = 141` en bench).

### Hogs entrelazados, no independientes

El "15% de GDScript dispatch" y el "3.7% de AnimationNode" se solapan: cuando
un AnimationTree advance recorre el blend tree, dispara Variant::callp por
cada blend node interno que evalúa parámetros. **Reducir el número de
AnimationTree activos también reduce el costo de Variant/Object::callp.**

Suma combinada accionable:
- **AnimationNode chain self-time:** 4113 samples
- **GDScript dispatch atribuible a AnimationTree advance (estimado conservador 30% del 4018 "callp from GDScript"):** ~1200 samples
- **Total atacable por culling-de-AnimationPlayer:** ~5300 samples ≈ **~5 % CPU del proceso, no 1.7 %**.

## Hog #2: AnimationPlayer/AnimationTree procesado siempre

Re-symbolicación de los offsets más calientes de libgodot_android.so vía
`llvm-symbolizer` (NDK 28.1) reveló callsites que el report inicial no había
imputado:

```
+0x2d9776c  AnimationNode::process(AnimationMixer::PlaybackInfo, bool)   1667 samples
+0x2d96468  AnimationNode::_pre_process(...)                             1365 samples
+0x43b2978  Object::_notification_forward(int)                           6170 samples
+0x236b8f4  SceneTree::_process(bool)                                    5928 samples
+0x165cc5c  GDScriptInstance::callp(...)                                 5864 samples
+0x12e4380  Main::iteration()                                            4433 samples (3 callsites suman ~10k)
+0x40570c0  Variant::callp(...)                                          4024 samples
+0x236d2d0  Object::notification(int, bool)                              3696 samples
+0x2327408  Node::_notification(int)                                     1825 samples
```

**AnimationNode::{process,_pre_process} suma ~3032 raw samples ≈ 1.7 % del total
del proceso, sólo en VkThread.** Además `Node::_gdvirtual__process_call` (1672)
es el dispatch que termina en GDScript `_process` por cada Node con
`_process` definido — encadena con los 5864 de `GDScriptInstance::callp`.

**Hipótesis de raíz: SDK7 Animator components y AnimationPlayers de avatares
remotos tickean cada frame sin culling por visibilidad/distancia.**

El sistema avatar tiene `_anim_throttle_active` en `avatar.gd:1252` (advance
manual via `animation_tree.advance(delta)` cada N frames cuando offscreen),
pero los **GltfContainer SDK7 Animator** no tienen equivalent — `apply_anims`
en `lib/src/godot_classes/animator_controller.rs:594` configura el AnimationTree
una vez y deja a Godot tickeándolo every frame.

GP tiene props con animaciones (NPCs ambient, decoración animada). Si hay
N AnimationPlayer/AnimationTree activos y M están off-screen / lejos, **N×M es
puro waste**. node_type_breakdown del bench reporta **141 AnimationPlayer**
(con comms held — esos no son remote avatars sino props de GP).

### Fix candidatos (ordenados por effort)

1. **Auto-throttle AnimationTree de SDK7 Animator basado en visibilidad de su
   GltfContainer.** Cuando `gltf_container.visible == false` (ya hay culling
   por distancia y frustum), set `process_mode = PROCESS_MODE_DISABLED` en el
   AnimationPlayer/AnimationTree. ~20 lines en
   `lib/src/godot_classes/animator_controller.rs` + un hook en gltf_container.gd
   `_process` que sincronice cuando `visible` cambia. Cero risk si la viz
   binding es correcta.
2. **Throttle by distance** (avanzar 1/N frames más allá de X metros). Más fino
   pero más complejo: hace falta un check del player position por frame.
3. **Pause cuando todas las animaciones tienen `playing=false`** (caso degenerate
   de SDK7 Animator state queue vacía pero AnimationTree aún tickea).

Mi voto: **fix #1 (visibility-driven disable)**. Mismo patrón que ya hacemos
para colliders en `gltf_container.gd:495,534`. Expected: −1.5 a −3 % CPU
total Android, posiblemente similar en iOS donde los AnimationTree también
corren.

## Próximos pasos

1. Profile iOS (paralelo) — pendiente, requiere correr con script ya armado.
2. Investigar zlib origin (git grep `deflate` en Godot source o trace via
   simpleperf call-graph). Hipótesis activa: zlib viene del PNG encode de
   `Image::save_png` del impostor — ya cubierto en hog #1, validar con bench
   post-cache-fix.
3. ✅ Identificar QUÉ scene/component genera los 519 dirty_lww_entries/frame —
   instrumentación agregada en `engine.rs::drain_crdt_component_breakdown` +
   expuesta vía `SceneManager::drain_crdt_component_breakdown` y emitida al
   JSON del bench como `crdt_component_breakdown`. Próximo bench mostrará
   top components por id+name.
4. Validar hog #2 (AnimationPlayer no-cull) con bench A/B: `disable_animator`
   toggle (similar a `disable_tweens`) que setee `process_mode = DISABLED` en
   todos los AnimationPlayers de scenes, mide delta FPS.
5. Pickear Path 1 (CRDT recv reduction) o el visibility-driven AnimationPlayer
   fix como primer commit con perf gain shipeable.

## Update 2026-05-06b: caja negra del CRDT recv abierta

Bench `crdt-component-breakdown` (10s sample window, 112 samples, 14.5 FPS
mean — debug build ~7 FPS por debajo del dev-release baseline pero las
proporciones son representativas):

```
lww:TweenState(1103)=15162    (~135/frame, 51%)
lww:Transform(1)=14563         (~130/frame, 49%)
lww:EngineInfo(1048)=57        (~0.5/frame, <1%)
gos:TriggerAreaResult(1061)=336 (~3/frame, <1%)
```

**El 100% del costo del CRDT recv está concentrado en DOS componentes:
TweenState y Transform.** Cada otro component (Material, GltfContainer,
AnimationStates, etc.) es ruido (<1%).

### Path 1 ahora tiene un target específico

**TweenState (135/frame):** Cada tween activo en GP reporta su
`currentTime` cada frame. Con 70 scenes y suponiendo ~2 tweens activos
promedio por scene = 140 entries/frame, exactamente lo medido.

Tres opciones de fix, ordenadas por effort:

1. **Skip per-frame TweenState reporting**: solo emitir dirty en
   `currentTime == 0` (start) o `currentTime >= duration` (completion).
   El JS scene-userland generalmente no LEE TweenState mid-flight; usa
   el evento `completed`. Si validamos eso, **−51% del recv pressure**
   en una sola línea de código en `lib/src/scene_runner/components/tween.rs`.
2. **Move tween processing to Rust nativo**: V8 nunca ve TweenState,
   solo el `completed` event vía LocalApi. Más invasivo (~200 lines).
3. **Dedup por delta**: solo emitir si `currentTime` cambió ≥ 16ms.

**Transform (130/frame):** Probablemente local player + entidades
animadas/tweened. Sin remote avatars (comms held), 130/frame es alto.
Hipótesis: cada Tween que está moviendo una entidad genera
TweenState + Transform updates en paralelo. Si fixeamos el #1, también
veríamos caer Transform al rate de "solo updates significativos".

### Recomendación final actualizada

**Primer commit con perf gain shipeable: fix #1 de TweenState reporting.**
- Effort: ~30 lines + 1 itest.
- Riesgo: bajo si validamos que JS no lee TweenState mid-flight.
- Expected: **−51% CRDT recv bytes**, **−~15% Thread-16 CPU** por menos
  ArrayBuffer allocation en V8.
- Si Transform también baja proporcionalmente: **−~25% Thread-16 CPU
  total**.

---

## Update 2026-05-07 — post-async-png-deep profile

Captura: `bench-results/profiles/android-post-async-png-deep-20260507T133215Z/`.
Build: Rust dev-release con async-PNG fix + tween-fix + anim-cull
aplicados; Godot debug template (15-20 % slower que release).

### FPS A/B (release-template builds, no debug overhead)

| Run | FPS mean | frame_proc_ms | Δ vs baseline |
|---|---|---|---|
| baseline (revert) | 14.48 | 74.7 | — |
| tween-fix solo | 14.19 | 68.3 | -2 % FPS / -8 % frame_proc |
| anim-cull + tween-fix | 14.32 | 68.6 | -1 % / -8 % |
| **async-png + anim-cull + tween-fix** | **17.09** | **63.9** | **+18 % FPS / -14 % frame_proc** |

El cap real de Android era el `Image::save_png` síncrono. Confirmado:
mover el encode a tokio worker rompió el techo de 14 FPS.

### VkThread post-fix (debug template, 18372 samples, 50 % del CPU total)

Top **incluyendo** la PNG-screenshot del bench (artifact de instrumentación,
ocurre 1 vez en `_dump_results`):

| Self % VkThread | Function | File |
|---|---|---|
| **7.9 + 2.2 + 1.9 + 0.9 = 12.9 %** | `longest_match` (zlib) | `thirdparty/zlib/deflate.c:1449` |
| 1.9 + 1.5 = 3.4 % | `abs(double)` | `c++/v1/stdlib.h:124` |
| 1.9 + 1.0 = 2.9 % | `Node3D::_propagate_transform_changed` | `scene/3d/node_3d.cpp:115` |

⚠️ Los 12.9 % de zlib/deflate son **el screenshot del bench runner**
(`gp_benchmark_runner.gd:292`). El _save_screenshot corre justo después del
sampling window; con 1080×2400 RGBA tarda ~200-500 ms = ~4-7 % del 10 s
window. **No es un hog del gameplay.**

### VkThread post-fix excluyendo PNG screenshot

Total weight 11.6 G (vs 15.2 G with PNG). Top leaves:

| Self % | Function | Path |
|---|---|---|
| 2.5 + 2.0 = **4.5 %** | `abs[abi:nn190000](double)` | inlined en math/transform helpers |
| 2.2 % | `read` syscall | I/O (FileAccess) |
| 1.9 + 1.4 = **3.3 %** | `Node3D::_propagate_transform_changed` | scene/3d/node_3d.cpp:115 |
| 1.7 + 1.0 + 0.5 = **3.2 %** | aarch64 atomics (cas/ldadd) | locks + ref counters |
| 1.1 % | `Node3D::get_global_transform` | node_3d.cpp:642 |
| 1.0 % | `__memcpy_aarch64_simd` | memory copy |
| 1.0 % | `hal::halp::draw_template_internal::draw_build_command` | Mali driver Vulkan submission |
| 0.8 % | `RenderingDeviceDriverVulkan::command_render_bind_vertex_buffers` | rendering_device_driver_vulkan.cpp:6280 |
| 0.7 % | scudo allocator | heap alloc/free |
| 0.5 % | `AnimationMixer::_blend_calc_total_weight` | animation_mixer.cpp:1166 |
| 0.5 % | `RenderingDeviceGraph::ResourceTracker::reset_if_outdated` | render graph |

### Otros threads (post-fix)

| TID | Identidad | % CPU total | Top symbols |
|---|---|---|---|
| 7561 | **VkThread** (main render) | **50 %** | ver tabla arriba |
| 7739 | **V8 main isolate** (SDK runtime) | **30 %** | `Builtins_LoadIC_Megamorphic`, `FindOrderedHashMapEntry`, `WeakCollectionSet`, scudo, atomic CAS |
| 7745 | V8 GC/Scavenger | 1 % | `BackingStore::Unregister`, `Scavenger::ScavengeObject`, `ArrayBufferSweeper::SweepYoung`, `Sweeper::RawSweep` |
| 7577 | mali-cmar-backe (GPU driver) | 3 % | `__ioctl`, `cmarp_release_atom_id`, `cmarp_backend_thread` |
| 7869 | Choreographer/swappy | 1 % | `__ioctl`, `recvfrom`, `Choreographer::registerStartTime` |

### Hallazgos accionables

1. **No hay un único hog gordo restante en VkThread.** Lo que queda es
   distribuido: transform propagation, abs/math, atomics, Vulkan submission.
   Para mover los próximos 10 % de FPS hace falta atacar varios frentes
   chicos en paralelo.

2. **Transform propagation 4.4 % de VkThread.** El usuario fija la cámara
   durante el sample, así que las propagaciones vienen de:
   - Animaciones (los 144 AnimationPlayer animan bones, cada bone
     `_propagate_transform_changed` por su esqueleto)
   - Tweens activos (no eliminados — sólo dedup'd state)
   - Avatar idle anims (locales y remotos)

   Posible próximo paso: extender el animation culling de
   `gltf_animation_coordinator.gd` también para `Skeleton3D` (bone-driven
   transforms) cuando el avatar/prop está fuera del frustum. Es lo mismo
   que hicimos con AnimationPlayer pero llegando a más fuentes de
   transform churn.

3. **Atomics 3.2 % en VkThread.** Los CAS/ldadd locked instructions vienen
   de `SafeNumeric<unsigned int>::conditional_increment` (refcount de
   recursos Godot). Cada draw_call y cada texture/material binding hace
   bumps. Mitigación natural: bajar draw_calls (1542 → menos via mejor
   batching) o bajar resource churn por frame. Difícil de atacar
   directamente sin cambios arquitectónicos.

4. **V8 isolate 30 % del CPU.** Los símbolos top
   (`Builtins_LoadIC_Megamorphic`, `FindOrderedHashMapEntry`) son
   inline-cache miss en el SDK7 dispatch. **No es un fix del cliente** — el
   SDK7 puede optimizarse (más Rust nativo, menos JS hot loops) pero
   está fuera del scope del cliente Godot. Worth flagging upstream.

5. **`_save_screenshot` del bench contamina el profile.** El zlib spike se
   debe enteramente a la screenshot final. Para profiles más limpios:
   mover el screenshot fuera del PROFILE_WINDOW o desactivarlo via flag.
   *No fix de gameplay.*

### Recomendación de Phase C (post async-png win)

Los siguientes candidatos en orden de ROI/riesgo:

1. **Skeleton3D animation culling (FPS gain medio, bajo riesgo).** Extender
   el coordinator de animaciones para pausar `AnimationMixer.advance` de
   skeletons fuera del frustum. Esperado: -1 % a -2 % VkThread, +0.5 a
   +1 FPS Android. ~50 lines.

2. **Impostor RAM (no FPS, pero -65 MB GPU).** RGBA8 → RGBA4444 + 64
   layers en `avatar_scene.rs:42, 222`. 1 archivo, low risk.

3. **Bench screenshot off-thread (limpia profiles).** Mover
   `_save_screenshot` a un signal post-bench que corre en `await` con
   `tween/Engine.idle_frame` para no overlapear con el sample. Cero
   cambio de gameplay.

4. **Reducir draw calls** (high gain, high risk). 1542 draw_calls implica
   batching insuficiente. El path Step 4 ya tiene MultiMesh batching para
   GLB estático con `PromotionTracker`. Verificar que está activo en GP
   y subir threshold para promover más assets. Esperado: -10 % VkThread
   si baja a ~1000 draws.

5. **Node count reduction** (highest gain, highest risk). 19425 nodes,
   3599 CollisionShape3D + 3572 StaticBody3D. Cada Node3D propaga sus
   notifications. Path original del plan (Step 5/6) — colapsar pares
   StaticBody+CollisionShape en el padre MeshInstance3D vía pointer_events
   metadata. Effort grande, gain potencial -20-30 % NodeProcess.
