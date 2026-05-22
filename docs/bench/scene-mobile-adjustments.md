# Mobile-friendly scene authoring

Mobile GPUs (Mali, Adreno, PowerVR) are 5–10× weaker than desktop discretes and run inside a thermal envelope that throttles them within minutes of sustained load. A scene that runs at 60 fps on PC will commonly land at 7–15 fps on mid-range Android unless it's authored for mobile from the start.

This guide gives concrete budgets, recommended techniques, and a triage checklist for scene authors targeting mobile (Mali-G68 / Adreno 650 class — roughly Samsung Galaxy A54 and equivalents).

---

## A note on where this should actually be solved

Decentraland scenes are **user-generated content** with no upper bound on complexity — a creator can ship a 100 k-triangle prop with 4K textures and the platform has to render it somewhere. The right place to make UGC mobile-friendly is **at upload time to the catalyst**, as a shared optimization service that runs once per scene revision and produces device-tier variants ready to serve.

That service should do automatically what this document asks creators to do manually: LOD generation, mesh splitting / chunking, dynamic texture atlasing, impostor generation for far props, vertex-stream stripping, format compression per target tier. Each client (Godot, Unity, web) would consume the pre-processed output instead of paying the cost at runtime on a thousand devices.

We are implementing pieces of this at the client level today — LODs, mesh splitting, atlasing, impostors all live in our pipeline — but **per-client implementations are duplicated work that drift apart over time**. A unified upload-time service shared across Decentraland products is the structurally correct place for this. Until then, this guide is a stopgap for creators who want their scenes to run on mobile.

The recommendations below are what an author can do **today** to ship mobile-ready scenes without waiting for the platform-side service.

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
