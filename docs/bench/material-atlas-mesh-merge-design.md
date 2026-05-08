# Material atlas + mesh merging — design

Branch: `feat/rendering-server-animations-1948`
Status: design / not implemented
Driver: Genesis Plaza on Samsung A54 is **GPU-bound at ~50 ms/frame**, with ~1500 draw calls. A/B vs `scaling_3d_scale=0.75` showed only -5 % GPU time → bottleneck is **draw-call submission / vertex processing**, not fill-rate / fragment shader.

## Goal

Bring draw calls from ~1500 to ~200–500, targeting **render_gpu_ms ≤ 30 ms** (~30 FPS) on the A54 device.

## Why plain mesh merging is not enough

Plain mesh merging only combines `MeshInstance3D`s that share a material. In Genesis Plaza, the merge profile (see `_count_node_types` in `gp_benchmark_runner.gd`) shows the material count grows roughly with mesh count — most meshes have unique materials. Plain mesh merging would yield a small reduction.

## Technique: material atlas + vertex-encoded material id

Combine **N materials into a single draw call** using:

1. A **`Texture2DArray` atlas** per merge bucket: each layer holds one source material's texture set (albedo / normal / ORM / emissive packed by channel or stored as separate arrays).
2. A **per-vertex material index** stored in `COLOR.r * 255` or a custom vertex attribute (`CUSTOM0`).
3. A **custom shader** that, in the fragment stage, samples the array using `texture(atlas_array, vec3(uv, vertex_material_id))`.

Result: one draw call renders fragments from up to N different materials. Vertex stages run once per merged mesh; fragment stages branch on the material id.

## Bucketing — what merges with what

A bucket key combines pipeline-state features that **must** match between merge candidates:

- `transparency` (opaque / alpha-blend / alpha-cutoff with same threshold)
- `cull_mode` (front-only vs double-sided)
- `vertex_format_flags` (UV0 only vs UV0+UV1, has tangent vs not, etc.)
- `texture_filter_mode` (linear vs nearest)

**Inside** a bucket, materials differ only by texture content + scalar params (baseColorFactor, emissiveFactor, etc.). Those go in the array / per-instance constant buffer.

Buckets that **don't merge with anything**: ShaderMaterial (custom shader, would need atlas-aware variant), shader-time-dependent materials, anything we can't classify.

## Mergeable / non-mergeable classifier

A `MeshInstance3D` is **non-mergeable** if any of:

| Skip rule | Why |
|---|---|
| Has `Skeleton3D` (skinned) | Vertices deform per-frame; can't bake into shared buffer |
| Has `AnimationPlayer` ancestor that animates this transform | Same |
| Mesh has blend shapes (morph targets) | Same |
| Material is `ShaderMaterial` | Custom shader; would need atlas-aware variant |
| Ancestor `DclAvatar` | Always animated |
| Entity has `Tween` SDK7 component | Transform mutates per-frame |
| Entity has `GltfNodeModifiers` | Mesh visibility / material toggled by SDK |
| Entity has `Visibility` toggling | Same |
| Entity has `PointerEvents` with hover state changing material | Same |

## Dynamic mutation — the JS-side problem

SDK7 scenes can attach `Tween`, `GltfNodeModifiers`, `Visibility`, etc. **to any entity at any time** via CRDT. A mesh that classifies as "static and mergeable" at one moment can become animated the next.

**Solution: deferred promotion + observed demotion** — extend the existing `PromotionTracker` from `lib/src/scene_runner/promotion_tracker.rs` (already used for MultiMesh batching).

### Deferred promotion

A mesh enters the merged atlas only after **N seconds of stability** (no mutating-component CRDT updates on its entity). Default `N = 5 s`. Concept:

```
on entity created          → mark UNSTABLE, last_mutation_at = now
on mutating component      → last_mutation_at = now
                              if currently MERGED → demote
each frame (capped @ K)    → for each UNSTABLE entity:
                                if (now - last_mutation_at) > N
                                  AND classifier passes
                                  AND atlas slot free
                                → promote into atlas, mark MERGED
```

### Observed demotion

When a mutating CRDT update lands on a merged entity, **demote immediately** — pull its vertices out of the merged buffer, instantiate a standalone `MeshInstance3D`. The CRDT pipeline (`lib/src/dcl/crdt/`) already routes component updates per-entity; we add an observer that flags atlas-tracked entities for demotion.

### Hysteresis (anti-thrashing)

Once an entity has been demoted, it stays demoted for **M seconds** (default `M = 60 s`) before becoming eligible for promotion again. Prevents promote/demote thrashing on scenes that toggle Tweens repeatedly.

## Cost analysis

**Per-frame tracking overhead:**

| Operation | Cost |
|---|---|
| CRDT observer (lookup + flag) on mutating-component update | ~1 µs per update |
| Promotion eligibility scan (capped K=20/frame) | ~10 µs |
| `last_mutation_at` HashMap (3500 entries × 16 B) | 56 KB RAM |

Total per-frame: **<0.1 ms**. Negligible vs the 50 ms GPU wait we're trying to fix.

**Spike costs (amortized over scene lifetime):**

| Operation | When | Cost |
|---|---|---|
| Bucket promotion (vertex bake into shared buffer + GPU upload) | Once per entity, after stability window | O(vertices) — for typical 100-vert prop, sub-ms |
| Atlas slot fill (texture upload to array layer) | Once per unique material | One image upload, sub-ms |
| Atlas reallocation (when capacity exceeded) | Rare; cap by pre-allocating buckets sized to 95th-percentile scene | ~ms-scale stall |
| Entity demotion (extract vertices, spawn standalone) | On Tween/Modifier add | sub-ms |

**Throttling:** cap promotions per frame to 20, demotions to whatever lands (rare). The throttle keeps the spike cost bounded.

## Estimated win

If 1500 → 200 draws (factor 7.5×):

- Driver per-draw cost (`vkCmdDrawIndexed`, RID rebinds, vertex buffer rebinds) drops ~85 %.
- Vertex stage cost mostly amortized (already issuing vertices, fewer state changes).
- Fragment stage cost unchanged (same pixels, same shader complexity).
- **Expected GPU time: 50 ms → 15–20 ms** = ~50–65 FPS uncapped, capable of sustaining 30 FPS cap with thermal headroom.

## Phasing

The work is large enough to phase into shippable increments:

### Phase 1 — Profile + classifier (DONE)

`_classify_mesh_mergeable` walks the SceneTree at end of bench → buckets meshes by pipeline-state key, counts unique materials, counts skipped (animated/skinned/shader/etc.). Output in bench JSON: `node_type_breakdown._merge_buckets` + `_unique_materials` + `_merge_skipped`.

**Genesis Plaza measured profile (run `mp-v6` on Samsung A54, Profile=Medium auto):**

```
MeshInstance3D total:   3550
  Mergeable:            2947  (83 %)
  Skipped (skinned):     598  (avatars)
  Skipped (animated):      1
  Skipped (shadermat):     4
Unique meshes:          1196
Unique materials:        992
```

Top 7 buckets (by count) cover 2354 meshes ≈ 80 % of mergeable:

| Count | Bucket signature |
|---|---|
| **1016** | opaque + culled + **no textures** (pure-color material) |
| 298 | opaque + culled + albedo + normal |
| 269 | opaque + double-sided + albedo + normal |
| 249 | opaque + culled + albedo + normal + emissive |
| 237 | opaque + double-sided + albedo + emissive |
| 185 | alpha-cutoff + culled + albedo |
| 109 | alpha-cutoff + double-sided + albedo |

Other 19 buckets cover the remaining ~600 meshes.

### Phase 2.0 — Textureless bucket — GDScript prototype lessons

We tried a GDScript prototype (`_apply_textureless_merge` in `gp_benchmark_runner.gd`) running at warmup phase. It successfully classified and combined 1093 textureless meshes. Three iterations, all regressed FPS:

| Variant | Strategy for source meshes | FPS | GPU ms | draws |
|---|---|---|---|---|
| baseline (tm-off) | n/a — no merge | 17.84 | 50.82 | 1645 |
| tm-on (no cells) | `queue_free` originals | crashed (Scudo: race on chunk header → SIGABRT) | — | — |
| tm-c4 (32 m cells, `visible=false`) | `mi.visible = false` | 14.09 | 63.29 | 1575 |
| tm-c5 (32 m cells, `layers=0`) | `mi.layers = 0` | 14.14 | 64.00 | 1699 |
| tm-mn2 (32 m cells, `mesh=null`) | `mi.mesh = null` + visible + layers | 16.71 | 52.99 | **1373** |

**Three lessons:**

1. **Without spatial partition, frustum culling collapses.** One mega-mesh covering all of Genesis Plaza has an AABB the culler cannot early-out on, so every frame processes vertices for the whole map (even the 80 % off-screen). FPS goes down because we *added* GPU work even though draw call count went down.

2. **`queue_free` races with the scene_runner Rust thread.** The Rust scene-tick code holds references to source MeshInstance3Ds via the entity→node map. Freeing one from GDScript while Rust is in the middle of a tick = scudo allocator catches a chunk header race → SIGABRT. We can't safely remove nodes from outside the scene_runner.

3. **`visible = false` and `layers = 0` are both overwritten by DCL each tick.** The scene_runner's MeshRenderer / GltfContainer state re-applies visibility / cull-mask on every tick based on SDK7 component state. A GDScript-side flip is a one-frame write that gets stomped immediately. **The merge cannot live outside the Rust scene_runner.**

**Implication:** Phase 2.0 must move into Rust, integrated with `mesh_renderer.rs` / `gltf_container.rs`. The merge cannot be retrofitted from GDScript; it must own the entity→node mapping it manipulates.

**Update — `mesh = null` works.** The `tm-mn2` follow-up dropped draws from 1645 → 1373 (-272). So setting the resource pointer to null *does* prevent rendering and *does* stick across frames. The previous variants failed because the Godot draw-call counter doesn't fully honor `visible=false` / `layers=0`, not because DCL was re-applying them. Two new findings from this run:

- Of the 1093 classifier-matched textureless meshes, only ~272 were producing visible draw calls. The other ~820 were collision-only / decorative-with-collider meshes which never rendered to begin with. Hiding them visually exposed underlying collider visualizers (the "veo meshes de colisión" report from the user).
- Even with 272 fewer source draws, FPS was -1.1 vs baseline because the merged meshes were built without index reuse — the prototype emits one vertex per *index* (`st.add_vertex(v)` per `idx[i]`), so shared vertices in source meshes get duplicated 6× on average. The 41 merged meshes ended up doing more vertex work than the 272 they replaced.

**Concrete API/algorithm requirements for the Rust port (validated):**

- Suppress originals via `mesh = null`, not `visible` or `layers`.
- Classifier must exclude collision-only meshes (e.g., entities whose visual mesh is hidden / whose material has `transparency=DISABLED` and exists only for the GltfContainer's invisible-collision mask).
- Merged ArrayMesh must preserve index buffer (append unique vertices + remap indices), or vertex bloat eats the draw-call savings.

### Phase 2.0 (real) — Textureless bucket in Rust scene_runner

The 1016-mesh textureless bucket is the highest-leverage / lowest-effort target. The prototype proved the *technique* works (1093 → 41 merged meshes with 32 m cells, classifier solid) but the *integration* must own the entity→node mapping.

**Implementation in Rust:**

- New module `lib/src/scene_runner/textureless_merger.rs`.
- `TexturelessMergerState` lives on `SceneState`, carries: spatial-cell map, atlas slot allocator (no atlas needed for color-only), bucket → merged `Gd<MeshInstance3D>` map.
- Hook in `mesh_renderer.rs` *update* path: when a `MeshInstance3D` is about to be (re)inserted into the scene tree, the merger asks "is this mesh textureless + classifier-passes + entity stable?" If yes, route the geometry into the merged buffer for that (cell, bucket) instead of spawning a fresh `MeshInstance3D` for the entity.
- The entity's `GltfContainer` keeps a placeholder Node3D for transform, but no per-entity `MeshInstance3D` is created.
- On entity mutation (Tween, modifier add, mesh change), the merger pulls the entity's vertices out of the merged buffer and lets `mesh_renderer` create a standalone `MeshInstance3D` (Strategy A: cold-store CPU-side blueprint, GPU only on demote).
- Spatial cell size: 32 m × 32 m horizontal grid (proven viable in prototype). Cells with < N source meshes (N ≈ 4) skip merging — overhead not worth it.

Expected delta on GP: **1016 draws → ~40 draws** (per-cell × per-bucket), with intact frustum culling (each merged mesh has cell-sized AABB).

### Phase 2.1 — Textured PBR atlas

Materials with at least one texture. Per-bucket `Texture2DArray` + custom shader. Vertex `CUSTOM0` carries material-id + albedoColorFactor packed.

### Phase 3 — Multi-bucket + full PBR

Add normal map / ORM / emissive arrays per bucket. Multiple buckets active concurrently (one shader variant per bucket). Atlas slot allocator with eviction policy.

### Phase 2 — Single-bucket prototype

- Pick the **largest** static-opaque bucket (probably `transp=0 cull=0 alb=1 nrm=0 em=0 orm=0` — typical PBR opaque).
- Implement `MaterialAtlas` Rust struct: `Texture2DArray` with N slots, packed albedo only.
- Implement `MeshCombiner`: takes M `MeshInstance3D`s + atlas → emits one combined `ArrayMesh` with per-vertex `CUSTOM0 = material_index_u8`.
- Custom `.gdshader` for the bucket, samples atlas array.
- Integrate via flag `material_atlas_enabled` (default OFF).
- Bench: GP delta on draws + GPU time.

### Phase 3 — Multi-bucket + full PBR

- Add normal map / ORM / emissive arrays per bucket.
- Multiple buckets active concurrently (one shader variant per bucket).
- Atlas slot allocator with eviction policy.

### Phase 4 — Runtime promote/demote (PromotionTracker integration)


- Wire `PromotionTracker` to mark entities stable after `N` seconds.
- CRDT-side observer in `lib/src/dcl/crdt/` for mutating components → demotion signal.
- Hysteresis cooldown.
- Per-frame promote/demote throttle.

### Phase 5 — Wearables / avatar exclusion polishing

- Avatars never merge (animated). Their draw calls are separate from this work.

## RAM strategy (the 2× problem)

Naive promote/demote keeps **both** representations: original `ArrayMesh` (for demote) and merged buffer (for rendering). That doubles vertex + texture memory. Three strategies to avoid:

### Strategy A — Cold-store original (CPU only)

After promotion, free the original's GPU resources but keep its CPU-side `ArrayMesh` data as a blueprint. Demotion = re-upload to GPU.

- Cost: 1.2× CPU RAM, 1× GPU RAM
- Benefit: GPU pressure unchanged (the metric that matters for VRAM-tight devices)
- Drawback: demote takes a few ms (vertex re-upload)

### Strategy B — Re-extract from merged buffer

No original kept. Demote pulls vertices out of the merged buffer using stored per-instance offset/length annotations + clears that atlas slot.

- Cost: 1× total RAM
- Benefit: best memory profile
- Drawback: **demote stall is 5–20 ms** (vertex copy + new GPU buffer alloc + atlas slot reclaim) — and it lands at the exact moment the user does something dynamic (Tween triggers, modifier toggles), which is the worst perceptual timing. A burst of 5 demotes in one frame can blow the entire frame budget.

### Strategy C — Don't promote things that might demote

Strict classifier: only promote entities with strong static signal (e.g., 30+ seconds since last CRDT mutation, no Tween/Animator/Modifier in their *historical* component record, scene-asset GLB origin). Originals are freed permanently. If something escapes the heuristic and later mutates, accept it stays merged — entity won't update visually.

- Cost: 1× RAM
- Benefit: simplest implementation
- Drawback: heuristic misses break entity behavior

### Recommended combo

**C as default + A as fallback** for borderline entities (~10 % of scene). For Genesis Plaza:

- 3500 meshes × ~1 KB avg vertex data = 3.5 MB CPU; 1.2× ≈ +0.7 MB
- 100 materials × 256×256 RGBA atlas = ~26 MB packed; cold-store of originals ~26 MB → +26 MB peak temporary
- **Total worst case: ~+30 MB** (4 % of the 745 MB GPU mem currently used)

Acceptable. The atlas itself replaces the original textures in the rendering hot path, so VRAM pressure on draws is actually reduced.

## Risks

- **Texture size mismatch**: source materials may have textures of varying dimensions. Either resize to atlas slot size (quality loss) or per-bucket fixed-size requirement (some materials excluded).
- **Z-sort regressions for transparency**: alpha-blend bucket needs stable per-instance depth ordering. Likely keep transparency unchanged for now (Phase 2 = opaque only).
- **Material parameter drift**: `baseColorFactor` etc. need a per-instance constant buffer or another vertex attribute. Adds vertex bandwidth.
- **Atlas rebuild stalls**: if atlas exceeds capacity at runtime, the rebuild is a multi-ms stall. Mitigate via pre-sized buckets sourced from a profile pass.
- **Shader complexity**: custom shader has to handle the per-bucket combination of features (with/without normal, with/without emissive). Multiple variants or `#define` permutations.

## Files (when implemented)

| File | Role |
|---|---|
| `lib/src/godot_classes/material_atlas.rs` | Atlas struct: Texture2DArrays + slot allocator |
| `lib/src/godot_classes/mesh_combiner.rs` | Bake N meshes + atlas → one ArrayMesh |
| `lib/src/scene_runner/promotion_tracker.rs` | Existing; extend with atlas eligibility |
| `lib/src/dcl/crdt/atlas_observer.rs` | New; signals demotion on mutating-component CRDT |
| `godot/assets/shaders/atlas_pbr.gdshader` | Custom shader sampling array |
| `lib/src/godot_classes/dcl_gltf_render_manager.rs` | Existing; route mergeable meshes to atlas instead of MultiMesh batcher |

## Open questions

- Do GLB importers in DCL preserve enough material metadata for atlas keying? Need to verify against scene assets.
- How to handle alpha-cutoff with arbitrary cutoff thresholds? Quantize to N levels?
- Decentraland avatar wearables share materials per body type — could a wearable-specific atlas work? Out of scope for Phase 2.
