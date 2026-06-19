# Genesis Plaza GLTF audit

Numbers from parsing the 637 `.glb` files in the shipped scene cache directly. Targeting mobile minspec (Mali-G68 / Samsung A54).

Decentraland scenes are UGC — most of the optimizations below are GLTF-in / GLTF-out transformations that ideally run **once at upload to the catalyst**, shared across all clients, rather than per-client at runtime.

## Totals

| Metric                  | Value     |
|---|---:|
| `.glb` files            | 637       |
| Total disk              | 603 MB    |
| Total triangles (LOD0)  | 2.61 M    |
| Total vertices          | 2.48 M    |
| Meshes                  | 3,490     |
| Primitives              | 4,393     |
| Materials               | 1,606     |
| Textures                | 3,013     |
| Animations              | 247       |

## Critical issues

### 1. 14 GLBs over 50 k triangles (top: 268 k)

| GLB                                | Triangles | File size |
|---|---:|---:|
| `background-buildings/BB_buildings.glb` | 268,106 | 14 MB |
| `central-plaza/clockTower.glb`          | 158,800 |  5 MB |
| `blockout/clock-tower.glb`              |  95,922 | 12 MB |
| `closed-game-arena.glb`                 |  82,664 |  8 MB |
| `theatre.glb`                           |  80,515 |  3 MB |

A single 268 k-tri mesh is ~5× a reasonable building budget for mobile.

### 2. 17 GLBs over 10 MB on disk (45 % of total scene weight)

`cp_roads.glb` (69 MB), `theatre.glb` (45 MB), `store.glb` (33 MB), `environment.glb` (31 MB), `news.glb` (29 MB), + 12 more.

### 3. No LOD chain shipped

Every mesh is LOD0 only. The renderer has to decimate at import or render the full count at all distances.

### 4. 64 % of GLBs ship normal maps (410 of 637)

Most small / distant props don't need them — they double texture memory and force the per-fragment lighting path.

### 5. 36 % of GLBs ship emissive textures (231 of 637)

Most are not actually glowing at runtime — likely bakeable into albedo.

### 6. 1,606 materials for 3,490 meshes (only ~ 2× sharing)

Near-duplicate materials (`stone_01`, `stone_01_dark`, …) inflate state changes.

### 7. 3,013 textures with no atlasing

422 are 1024 × 1024, 204 are 512 × 512. Heavy candidates for atlas consolidation (target ~ 400 atlas-bound textures total).

### 8. 47 GLBs ship JOINTS/WEIGHTS without a `skin`

Skinning attributes (16 bytes/vertex) shipped on meshes that aren't actually rigged.

### 9. 149 GLBs ship vertex `COLOR_X` or `TEXCOORD_1+` streams

UV2/UV3 and COLOR_1+ are unused at runtime; 8–16 bytes/vertex wasted.

### 10. 149 GLBs (23 %) use transparency or alpha-test

Each is several times more expensive per visible triangle than opaque equivalents.

## Distributions

### Triangles per GLB

| Bucket          | GLBs |
|---|---:|
| < 100           | 121  |
| 100 – 1 k       | 296  |
| 1 k – 5 k       | 147  |
| 5 k – 20 k      | 42   |
| 20 k – 50 k     | 17   |
| **> 50 k**      | **14** |

### File size per GLB

| Bucket           | GLBs |
|---|---:|
| < 100 KB         | 428  |
| 100 – 500 KB     | 108  |
| 500 KB – 1 MB    | 26   |
| 1 – 5 MB         | 51   |
| 5 – 10 MB        | 7    |
| **> 10 MB**      | **17** |

### Embedded texture resolution

| Resolution  | Count |
|---|---:|
| 256 × 256   | 20   |
| 512 × 512   | 204  |
| 1024 × 1024 | 422  |
| 2048 × 2048 | 17   |
| external / not detected | 2,088 |

### Vertex streams in use

| Stream                          | GLBs |
|---|---:|
| POSITION + NORMAL + TEXCOORD_0  | 602  |
| TEXCOORD_1 (UV2)                | 50   |
| TEXCOORD_2                      | 47   |
| COLOR_0                         | 76   |
| COLOR_1                         | 69   |
| COLOR_2 / COLOR_3               | 4    |
| JOINTS_0 + WEIGHTS_0            | 47   |
