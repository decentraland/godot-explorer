# Mobile-friendly scene authoring

Mobile GPUs (Mali, Adreno, PowerVR) are 5–10× weaker than desktop discretes and run inside a thermal envelope that throttles them within minutes of sustained load. A scene that runs at 60 fps on PC will commonly land at 7–15 fps on mid-range Android unless it's authored for mobile from the start.

This guide gives concrete budgets, recommended techniques, and a triage checklist for scene authors targeting mobile (Mali-G68 / Adreno 650 class — roughly Samsung Galaxy A54 and equivalents).

---

## A note on where this should actually be solved

Decentraland scenes are **user-generated content** — creators ship whatever GLTFs they make, and the platform has to render them. Most of the optimizations in this guide are **operations on the GLTF itself**: generating LOD meshes, splitting oversized meshes, atlasing textures within a GLB, baking impostors, stripping unused vertex streams.

These are GLTF-in / GLTF-out transformations. The right place to run them is **at upload time to the catalyst**, once per scene revision, producing an optimized GLTF that every client (Godot, Unity, web) consumes as-is.

We are implementing those transformations at the client level today, but per-client GLTF processing is duplicated work — Godot, Unity and the web client each end up reimplementing the same LOD bake, the same atlasing, the same vertex-stream cleanup. A unified GLTF-optimization service at upload time is the structurally correct place for this.

Until that service exists, the recommendations below are what an author can do today to ship mobile-friendly GLTFs.

---

## Why mobile is different in one paragraph

Desktop GPUs have lots of memory bandwidth, deep pixel pipelines, and a stable power budget. Mobile GPUs have **shared memory** with the CPU, much shallower pipelines (fragment shading is the slowest stage on Mali), and **thermal throttling** that drops performance ~30 % after ~3 minutes of heavy load. Optimizing for mobile means: less data, fewer state changes, simpler shaders.

---

## Recommended budgets

These are starting targets for a mid-range Android phone at the HIGH graphics profile. Going significantly over reduces the device tier you can address.

### Scene-wide budgets

| Metric                                  | Target  | Hard limit |
|---|---:|---:|
| Visible triangles per frame             | ≤ 800 k | 1.2 M     |
| Visible objects per frame               | ≤ 800   | 1,200     |
| Draw calls per frame                    | ≤ 1,200 | 1,500     |
| Unique materials in the scene           | ≤ 80    | 150       |
| Unique source meshes in the scene       | ≤ 200   | 400       |
| Unique 1 K-equivalent textures          | ≤ 100   | 200       |
| Total texture memory                    | ≤ 400 MB| 600 MB    |
| Total GLB-on-disk for the scene         | ≤ 80 MB | 150 MB    |
| Scene-graph node count                  | ≤ 8,000 | 15,000    |

### Per-mesh triangle budgets

Budget is per **source mesh** (the GLB), not per instance. Reusing the same source 50× costs as much as 1× for the source budget.

| Mesh category               | Static (no animation) | Dynamic (animated / skinned) | Hero / landmark |
|---|---:|---:|---:|
| Small prop (< 1 m AABB)     | ≤ 500     | ≤ 1,000  | ≤ 2,000   |
| Medium prop (1–3 m)         | ≤ 2,000   | ≤ 3,000  | ≤ 5,000   |
| Large prop / vehicle (3–8 m)| ≤ 5,000   | ≤ 8,000  | ≤ 15,000  |
| Building (8–20 m)           | ≤ 10,000  | ≤ 15,000 | ≤ 30,000  |
| Landmark (> 20 m)           | ≤ 25,000  | —        | ≤ 60,000  |
| Avatar / character          | —         | ≤ 12,000 | —         |
| Foliage card / billboard    | ≤ 50      | —        | —         |

### Texture budgets

Texture resolution is one of the most over-spent budgets on mobile scenes. The right resolution is the one **the texture is actually sampled at on the player's screen**, not the resolution the asset was authored at.

#### Memory cost by resolution

Every doubling of resolution quadruples GPU memory. ASTC 6×6 (mobile) compression assumed; uncompressed source is ~4× these numbers.

| Resolution | GPU memory (ASTC 6×6, with mipmaps) |
|---:|---:|
| 256 × 256   | ~0.1 MB   |
| 512 × 512   | ~0.4 MB   |
| 1024 × 1024 | ~1.7 MB   |
| 2048 × 2048 | ~6.6 MB   |
| 4096 × 4096 | ~26 MB    |

A single 4K texture costs as much GPU memory as 256 × 256 textures at 256 each. Pick your battles.

#### Recommended resolution per use case

| Use case                              | Recommended | Hard cap |
|---|---:|---:|
| UI icons / pip-style elements         | 128–256   | 512  |
| Avatar wearables (worn item textures) | 512       | 1024 |
| Small props (< 1 m, seen from > 2 m)  | 256–512   | 1024 |
| Medium props (1–3 m)                  | 512–1024  | 1024 |
| Standard buildings (8–20 m)           | 1024      | 2048 |
| Hero / landmark buildings             | 1024–2048 | 2048 |
| Sky / cubemap / panoramic background  | 1024 per face | 2048 per face |
| Impostor / billboard far-LOD textures | 128–256   | 512  |
| Particle / VFX sprites                | 128–512   | 512  |

**Rule of thumb**: divide the asset's max on-screen size (in pixels) by 2. That's the resolution you need. A 32 × 32 m building face viewed from > 20 m on a 1080p phone subtends ~200 pixels — a 4K texture on that face is 99 % wasted.

#### Texture count per material

Aim for **≤ 3 textures per material** on mobile. The typical composition:

| Texture                            | Channels                     | When to ship |
|---|---|---|
| Albedo                             | RGB (+ A if alpha-cutout)    | Always |
| ORM (Occlusion-Roughness-Metallic) | R: AO, G: rough, B: metallic | Pack 3 textures into 1 |
| Normal                             | RG (B reconstructed)         | Only if the surface needs detail bump |
| Emissive                           | RGB                          | Only if genuinely glowing |

Each extra texture sampled per fragment costs ~0.5 ms on Mali-G68 at 1080p. **Skip normal maps on props < 2 m AABB** — at typical viewing distance they don't read.

#### Atlas sizes

For atlases, pick the smallest power of two that fits your tiles plus padding. Typical ranges:

| Atlas usage                              | Recommended size |
|---|---:|
| UI icon atlas (16–64 icons at 64–128)    | 512 or 1024 |
| Small-prop albedo atlas (8–16 props at 256) | 1024 |
| Medium-prop albedo atlas (16 at 512)     | 2048 |
| Foliage atlas (4–8 leaf textures at 512) | 1024 or 2048 |
| Sign / decal atlas                        | 1024 |

Leave **8–16 pixels of padding** between atlas tiles to prevent mip-level bleeding. Use **bleed dilation** on the atlas (the texture content gets extended into the padding area) to handle sub-pixel UV sampling at lower mips.

**Don't atlas across alpha modes** — opaque and alpha-tested textures must live in separate atlases (the renderer batches by alpha mode).

#### Per-scene texture budget

| Limit                              | Target  | Hard cap |
|---|---:|---:|
| Total unique 1K-equivalent textures | ≤ 100   | 200      |
| Total texture memory at peak       | ≤ 400 MB| 600 MB   |
| Unique textures referenced in any one POV | ≤ 60 | 100  |

Genesis Plaza as shipped reaches ~630 MB texture memory (Godot-tracked) and ~1+ GB on the GPU once mipmaps and format padding count — over the hard cap by a large margin.

Skip 4 K unless the asset genuinely fills the screen. Avoid normal maps on small / distant props (they double texture memory **and** force the slower per-fragment lighting path).

---

## Mesh strategy

### Static vs dynamic

These are completely different cost classes on mobile.

| Aspect                       | Static                                | Dynamic (`AnimationPlayer`, skinned)         |
|---|---|---|
| GPU buffer sharing (instancing) | Yes — N copies share one VBO          | No — every instance is its own VBO           |
| Automatic LOD                | Yes                                   | No (bones would mismatch after decimation)   |
| Frustum / occlusion culling  | Cheap, per-AABB                       | Same but uses the looser animated AABB       |
| Shadow pass cost             | Once per mesh                         | Once per instance, every frame                |
| Draw batching                | Across same-material meshes           | Almost never                                  |

**Rule of thumb**: an animated mesh costs **5–10× as much per visible triangle** as the equivalent static one on mobile. Reserve animations for hero content (doors, characters, vehicles in motion). Decorative motion (flags, leaves swaying) is almost always cheaper as a **vertex-shader effect** than as an `AnimationPlayer` keyframe track.

### Mesh sharing across instances

The single highest-impact decision is whether you ship **one GLB referenced N times** or **N byte-different GLBs**. The renderer cannot tell that two byte-different GLBs encode the same geometry; it allocates VRAM for both.

In Genesis Plaza we measured ~5,800 rendered surfaces but only ~200 unique source meshes — the other ~5,600 were duplicates of those 200, each occupying its own slot in GPU memory.

**Always**: re-use one GLB via SDK transforms. **Never**: copy the same file with a different name to apply different transforms.

### LODs

For any mesh > 3 k triangles, ship author-controlled LODs:

| LOD level | Indicative tri count vs LOD0 | When it's used (mobile typical) |
|---|---:|---|
| LOD0      | 100 %                       | < 8 m from camera     |
| LOD1      | 40–50 %                     | 8–20 m                |
| LOD2      | 15–25 %                     | 20–50 m               |
| LOD3      | 5–10 %                      | > 50 m                |

LOD0 is what the player sees up close, so don't over-decimate it. The aggressive cuts come at LOD2/LOD3.

### Mesh AABB sizes

The renderer's frustum culling, LOD selection, and shadow cascade slotting all key off the mesh's bounding box.

- Avoid meshes whose AABB diagonal exceeds **30 m** — the whole mesh ends up in the same shadow cascade and the same LOD slot regardless of which part is on-camera. Split hero buildings into ~ 8 m chunks at authoring time.
- At the other extreme: meshes with AABB < 0.5 m are below the per-draw fixed-cost threshold; merge them into a parent.

### Vertex format

Ship only the vertex streams you actually use. A mesh with `POSITION + NORMAL + TANGENT + UV + UV2 + COLOR + BONES + WEIGHTS` is ~85 bytes per vertex; a sane mobile mesh is ~32 bytes. Multiply by N vertices, multiply by N instances — the savings are large.

Drop:
- `TEXCOORD_1` (UV2) unless you're actually using lightmaps.
- `TANGENT` unless the material has a normal map.
- `COLOR` unless vertex color meaningfully varies (not all-white).
- `JOINTS / WEIGHTS` unless the mesh is skinned.

---

## Materials & textures

### Texture atlasing (highest-ROI authoring decision)

Atlasing means **packing many small textures into one large texture**, then using UV offsets to address each. Three reasons it's a huge mobile win:

1. **Fewer draw calls.** Two props that share an atlas can be drawn in a single batch instead of two. With ~30 typical signs on a parcel, that's a 30× draw-call collapse.
2. **Less GPU memory.** A single 1024 × 1024 atlas holding 16 prop textures uses 4 MB; 16 separate 256 × 256 textures occupy 4 MB on disk but ~5–6 MB once mip levels and format padding are counted.
3. **Better cache behavior.** The GPU's texture cache reads atlas tiles efficiently; flipping between many small textures defeats the cache.

**Practical atlasing**:
- Group props by spatial proximity and visual style. Sign textures → one atlas. Building trim textures → one atlas. Foliage textures → one atlas.
- Reserve atlas size: 1024 (16 props at 256), 2048 (16 props at 512 or 64 props at 256).
- Leave padding (8 px minimum) between atlas tiles to avoid mip bleeding.
- Don't atlas across **different alpha modes** — opaque and alpha-tested textures must live in separate atlases.

### Packed texture channels (ORM)

Mobile materials usually need: albedo, roughness, metallic, optionally normal, optionally occlusion (AO).

The conventional setup (5 separate textures) is wasteful. Pack roughness + metallic + AO into one ORM texture (one read, three channels).

| Texture                  | Channels                       | Comment                               |
|---|---|---|
| Albedo                   | RGB + optional A (alpha-cutout)| Always                                |
| ORM (Occlusion-Roughness-Metallic) | R: AO, G: Roughness, B: Metallic | One texture instead of three |
| Normal                   | RG (reconstructed B in shader) | Optional, skip on small/distant props |
| Emissive                 | RGB                            | Only when genuinely glowing           |

This halves typical material texture count.

### Material sharing

The renderer batches draws by material. Two props with the same material → one draw call. Two props with materials that differ in a single tint → two draw calls.

- Use one material across many props whenever the textures are the same.
- Vary appearance via vertex color or UV offset, not via per-instance material clones.
- Avoid "near-duplicate" materials (`stone_01.material`, `stone_01_slightly_darker.material`) — pick one, use overrides if absolutely needed.

### Transparency strategy

Transparent and alpha-tested materials cost much more than opaque ones on mobile because:
- They disable early-z (every alpha-tested pixel runs the full fragment shader before its alpha is known).
- They cause overdraw when multiple layers overlap.
- They cannot be automatically LOD-decimated safely.

**Use opaque whenever you can**. For unavoidable transparency (foliage, fences, decals):

- Prefer **opaque alpha-cutout cards with tight silhouettes** over loose transparent billboards. The cutout area should be < 50 % of the quad.
- Cap overlapping foliage layers at 3 per parcel.
- For dense vegetation, prefer **one low-poly mesh with baked variation** over many billboard cards.

### Texture compression

Always ship textures in a compressed format. The pipeline handles conversion (ASTC 6×6 on mobile, BC1/BC3 on desktop), but you should:

- Author source textures in **PNG with no alpha when alpha is not used**.
- Ensure dimensions are **powers of two** (256, 512, 1024) — non-power-of-two textures skip mipmaps on some drivers.
- Avoid alpha channel in albedo unless the material is alpha-cutout / alpha-blended.

---

## Lighting and shadows

### Cast shadow curation

Every shadow caster roughly doubles the cost of the directional light pass. The default in most authoring tools is "everything casts shadow" — fine on desktop, expensive on mobile.

Mark `cast_shadow = OFF` for:
- All foliage, billboards, fences, signs.
- Indoor furniture, decorative trim, small props (< 1 m in any dimension).
- Anything whose shadow contribution is invisible (a tree shadow under a building's shadow).

Keep `cast_shadow = ON` for:
- Buildings, landmarks, ground meshes (the cheapest per-vertex case and most visually important).
- Hero props whose shadow defines the silhouette (statues, large signage).

### Baked lighting vs runtime

Runtime PBR lighting is the most contended stage on mobile. For static lighting (sun angle doesn't change, no moving lights), baked lighting saves the most fragment cost of any single technique.

- Bake **AO into the albedo or into the ORM texture's red channel**.
- Bake **specular highlights and gradient lighting into the albedo** for matte materials.
- Reserve true PBR for materials that need it (metallic statue, glass).

### Dynamic lights

Each additional dynamic light (`OmniLight3D`, `SpotLight3D`) adds a render pass for every lit surface. On mobile, **target ≤ 4 dynamic lights visible at once**, ideally 0–2.

If you need many small lights, use **emissive materials** instead — they don't actually illuminate surrounding geometry but read as "this thing is glowing" visually, at fragment-only cost.

---

## Animation and dynamic content

- Use `AnimationPlayer` only where motion is gameplay-critical (doors, vehicles, NPCs).
- Decorative motion (flag flutter, leaf sway, water ripple) → **vertex shader effect**.
- Particles: cap to a few hundred particles per visible system, no overdraw-heavy stacks of additive-blended sprites.
- Skinned characters: ≤ 12 k tris on mobile, no more than ~ 4 characters visible at once.

---

## Authoring quick-wins (effort × payoff)

| Effort | Win | Action |
|---|---|---|
| 5 min   | High      | Toggle `cast_shadow = OFF` on non-essential MIs |
| 15 min  | High      | Drop unused vertex streams (UV2, TANGENT, COLOR) per mesh |
| 30 min  | High      | Downsize textures > 1024 on small / distant props |
| 1 hr    | High      | Pack roughness + metallic + AO into ORM textures |
| 2 hr    | Very high | Build a shared atlas for groups of small props |
| 2 hr    | High      | Ship `_lod1.glb` for any prop > 3 k tris, any building > 5 k tris |
| 4 hr    | Very high | De-duplicate prop variants: replace N near-identical GLBs with one + transform overrides |
| 1 day   | High      | Convert decorative animations (flags, foliage sway) into vertex-shader effects |
| 1 day   | Medium    | Merge small static decorations into shared GLBs |
| 1 day   | Medium    | Bake AO + AO-modulated lighting into albedo |
| 2 days  | Medium    | Split hero buildings into ~ 8 m chunks for cleaner LOD / shadow behavior |

---

## What we measured: raw GLTF data from Genesis Plaza

We audited every `.glb` shipped in the Genesis Plaza scene cache (`Genesis-Plaza-2025-30cdaffd`). Numbers come from parsing the GLBs directly, not from runtime measurement.

### Totals

| Metric                          | Value     | Notes |
|---|---:|---|
| Total `.glb` files              | 637       | one scene |
| Total disk size                 | 603 MB    | downloaded by every player |
| Total triangles                 | 2.61 M    | summed across all meshes (LOD0 only — no LODs shipped) |
| Total vertices                  | 2.48 M    | suggests poor index reuse (verts ≈ tris, so each tri is ~3 unique verts) |
| Total Godot meshes              | 3,490     | distinct mesh objects |
| Total primitives                | 4,393     | mesh × surface |
| Total materials                 | 1,606     | unique BaseMaterial3D |
| Total textures                  | 3,013     | most are 512 or 1024 |
| Animations                      | 247       | (across 82 skinned + others keyframed) |

### Source-mesh size distribution

| Triangle bucket | GLBs |
|---|---:|
| < 100 tris       | 121 |
| 100 – 1k         | 296 |
| 1k – 5k          | 147 |
| 5k – 20k         | 42  |
| 20k – 50k        | 17  |
| **> 50k tris**   | **14** ← over the recommended building budget |

Worst offenders (top 5 by triangle count, single GLB):

| GLB | Triangles | File size |
|---|---:|---:|
| `background-buildings/BB_buildings.glb`    | 268,106 | 14 MB |
| `central-plaza/clockTower.glb`             | 158,800 |  5 MB |
| `blockout/clock-tower.glb`                 |  95,922 | 12 MB |
| `closed-game-arena.glb`                    |  82,664 |  8 MB |
| `theatre.glb`                              |  80,515 |  3 MB |

A single 268k-triangle building is **5× the hero/landmark budget** and singlehandedly fills ~25 % of the per-frame triangle budget.

### File-size distribution

| Size bucket | GLBs |
|---|---:|
| < 100 KB         | 428 |
| 100 KB – 500 KB  | 108 |
| 500 KB – 1 MB    | 26  |
| 1 MB – 5 MB      | 51  |
| 5 MB – 10 MB     | 7   |
| **> 10 MB**      | **17** ← too big to download fast on mobile |

The 17 GLBs over 10 MB account for **45 % of total disk** despite being 2.7 % of the file count. Worst single GLB: `blockout/cp_roads.glb` at **69 MB**.

### Texture inventory

We parsed embedded image headers (PNG/JPEG) and found:

| Resolution | Count |
|---|---:|
| 256 × 256   | 20  |
| 512 × 512   | 204 |
| 1024 × 1024 | 422 |
| 2048 × 2048 | 17  |
| Other / external | 2088 |

422 textures at 1024 × 1024 is the dominant authoring resolution. That's reasonable for medium buildings but oversized for the many small props that are also 1024.

### Material-feature usage

| Feature                     | GLBs (of 637) | % |
|---|---:|---:|
| Uses normal maps            | 410 | **64 %** |
| Uses emissive               | 231 | 36 % |
| Uses alpha BLEND (transparent) | 84 | 13 % |
| Uses alpha MASK (alpha-tested) | 65 | 10 % |
| Skinned (rigged to bones)   | 82  | 13 % |

### Vertex-stream usage

| Stream                | GLBs |
|---|---:|
| POSITION + NORMAL + TEXCOORD_0 | 602 (mandatory) |
| TEXCOORD_1 (UV2)               | 50  |
| TEXCOORD_2                     | 47  |
| COLOR_0                        | 76  |
| COLOR_1                        | 69  |
| COLOR_2                        | 3   |
| COLOR_3                        | 1   |
| JOINTS_0 / WEIGHTS_0 (skinning) | 47 |

---

## Issues found in Genesis Plaza (and what to do about them)

These are concrete things visible in the raw GLTF data above.

### 1. 14 GLBs blow the per-mesh triangle budget by 2–5×

`BB_buildings.glb` (268 k tris), `clockTower.glb` (159 k), `clock-tower.glb` (96 k), `closed-game-arena.glb` (83 k), `theatre.glb` (80 k), and 9 others.

**Action**: decimate at authoring time. Most of these are visually identifiable as "could be 30 % the polygon count without a perceivable difference" — they're authored at desktop-game density.

### 2. 17 GLBs are > 10 MB on disk (45 % of total scene weight)

`cp_roads.glb` at 69 MB, `theatre.glb` at 45 MB, `store.glb` at 33 MB, `environment.glb` at 31 MB, `news.glb` at 29 MB.

**Action**: audit individually. Usually one of (a) oversized embedded textures, (b) too many triangles, or (c) duplicated geometry within the file.

### 3. 410 of 637 GLBs (64 %) ship normal maps

Most small / distant props don't need normal maps and pay 2× the texture-memory cost plus force per-fragment lighting.

**Action**: strip normal maps from any prop < 2 m AABB. Keep them on hero buildings and characters.

### 4. 231 GLBs (36 %) ship emissive textures

Many of these are likely "lit logos" or "screen content" that could be baked into albedo with a high constant brightness.

**Action**: audit which emissives genuinely glow at runtime. Replace the rest with bright albedo + post-process bloom on a single global emissive pass.

### 5. 1,606 materials for 3,490 meshes (~ 1 material per 2.2 meshes)

Material reuse is poor — only ~ 2× sharing across the whole scene. Each unique material is a separate state change in the draw call stream.

**Action**: identify near-duplicate materials (`stone_01`, `stone_01_dark`, `stone_01_wet`) and consolidate. Vary appearance via UV offset or vertex color rather than material clones.

### 6. 3,013 textures with no atlasing

Each texture is a separate sampler binding. With ~ 1,200 average draws per frame and ~ 3,000 textures available, the texture cache thrashes.

**Action**: build atlases for groups of small props (signs, decals, small building trim). Realistic target: 3,013 → ~ 400 textures via atlasing.

### 7. 149 GLBs ship vertex `COLOR_X` streams; 47 ship `TEXCOORD_2`

UV2/UV3 and COLOR_1+ are useless at runtime — we don't lightmap-bake and most materials ignore vertex color.

**Action**: strip these streams at export. They add 8–16 bytes per vertex with zero visual benefit.

### 8. 47 GLBs ship `JOINTS_0` / `WEIGHTS_0` (skinning streams)

That's higher than the 82 GLBs that actually have `skins` set, suggesting **35 GLBs ship skinning attributes for meshes that aren't actually rigged**. Those bytes are pure waste — skinning streams cost 16 bytes per vertex.

**Action**: strip JOINTS/WEIGHTS on any mesh whose top-level GLTF `skin` array is empty.

### 9. 269 GLBs are < 100 KB (likely duplicable / atlasable)

A 100 KB GLB is typically a tiny prop. With 269 of them, many are repeated decorations.

**Action**: identify which ones are near-duplicates (lampposts, benches, signs) and consolidate to a single GLB + SDK transform instancing.

### 10. 84 + 65 = 149 GLBs use transparency or alpha-test (23 %)

Each one is significantly more expensive per visible triangle than opaque equivalents.

**Action**: review which need true transparency. Most signs / posters can be opaque rectangles with transparent areas baked into UV offsets in an atlas.

---

## Quick triage table

If you measure the scene and see one of these patterns, address the corresponding column first.

| Symptom                                  | Likely cause                                  | Easiest fix |
|---|---|---|
| Triangles visible per frame > 1 M       | Source meshes too heavy / no LODs             | Add author LODs, decimate LOD0 |
| Draw calls > 1,500                      | Many small unique meshes / materials          | Atlas textures, merge decorations, share materials |
| GPU frame time high, triangles low      | Fragment-bound (transparency, normal maps, lights) | Drop transparency layers, simplify materials |
| Texture memory > 500 MB                 | Oversized textures                            | Downsize, drop UV2, drop emissives |
| App RAM > 2 GB                          | Mesh / texture duplication                    | De-duplicate, atlas, downsize |
| FPS spikes then drops after ~3 min      | Thermal throttling                            | Cut overall load by ~30 % |
| Loading time > 30 s                     | GLB total size too large                      | Compress textures, audit oversized GLBs |
