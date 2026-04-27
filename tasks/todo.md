# Avatar Impostor System — Plan aprobado (issue #496)

Branch: `feat/avatar-impostors-496` · Worktree: `feat-avatar-impostors-496`
Plan completo (canonical): `~/.claude/plans/compiled-snuggling-naur.md`

## Decisiones cerradas
- Arquitectura: **hibrida** — render via RenderingServer + MultiMesh + Texture2DArray gestionado desde Rust en `AvatarScene`. Captura via scene-tree reusando `AvatarPreview`. FULL/MID en scene tree sin cambios.
- Threshold 30m, billboard Y, tint progresivo, cross-fade dithered 25-30m.
- Mid-range opts (15-30m) **incluidas** en este PR.
- AvatarShapes **incluidos** con captura local obligatoria.
- Body snapshot local del local player **lazy on-demand**.

## Out of scope (fuera de este PR)
- Material dedup (#1), LOD multi-mesh (#2), comms stress (#4), GPU skinning, octahedral impostor.

---

## Checklist de implementacion

### Fase 0 — Pre-flight
- [ ] Confirmar bindings de gdext para `MultiMesh`, `MultiMeshInstance3D`, `Texture2DArray` (`set_instance_transform`, `set_instance_custom_data`, `update_layer`).

### Fase 1 — Constantes y config
- [ ] `godot/src/decentraland_components/avatar/impostor/avatar_impostor_config.gd` con thresholds y tamaños.
- [ ] Toggle "Avatar Impostors" en `godot/src/ui/components/settings/settings.gd`, persistente, default ON.

### Fase 2 — Rust: fetch del body snapshot del catalyst
- [ ] `lib/src/content/content_provider.rs::fetch_avatar_body_texture(user_id)` clonando el patron de `fetch_avatar_texture` (line 1960) pero leyendo `snapshots.body_url`. Cache key independiente.

### Fase 3 — Rust: AvatarScene + MultiMesh impostor render
- [ ] Campos nuevos en `AvatarScene` (`lib/src/avatars/avatar_scene.rs`):
  - `impostor_multimesh: Gd<MultiMeshInstance3D>` con `MultiMesh { mesh: QuadMesh, transform_format: 3D, use_custom_data: true, instance_count: 256 }`.
  - `impostor_texture_array: Gd<Texture2DArray>` 256×512×256 RGBA8.
  - `impostor_slots: HashMap<SceneEntityId, ImpostorSlot>`.
  - `free_layers: Vec<u32>`.
- [ ] Funciones `#[func]` expuestas a GDScript:
  - `request_impostor_layer(entity_id) -> i32`.
  - `set_impostor_texture(entity_id, image)`.
  - `set_impostor_state(entity_id, fade_alpha, tint_strength)`.
  - `clear_impostor(entity_id)`.
  - `invalidate_impostor_texture(entity_id)`.
- [ ] `process(dt)` de `AvatarScene` actualiza transforms (Y-billboard CPU si no se hace en vertex shader) + custom_data por slot activo.
- [ ] Hook en `add_avatar` / `remove_avatar` para inicializar/limpiar slots.

### Fase 4 — Shader del impostor
- [ ] `godot/assets/avatar/impostor.gdshader`:
  - Vertex: Y-billboard rotando el quad para mirar la camara solo en plano XZ. Transform via `INSTANCE_CUSTOM` y `MODEL_MATRIX`.
  - Fragment: `texture(impostor_array, vec3(UV, layer_index))`. Tint multiplicativo. Discard Bayer 4×4 cuando `dither_threshold(SCREEN_UV) > fade_alpha`. Unlit.
  - Uniforms: `impostor_array: sampler2DArray`, `tint_color: vec3 = (0,0,0)`.

### Fase 5 — Toon shaders: cross-fade dither uniform
- [ ] `godot/assets/avatar/dcl_toon.gdshaderinc`: agregar `uniform float dither_alpha = 1.0;` + funcion `dither_threshold(SCREEN_UV)` Bayer 4×4. Discard antes del ALBEDO.
- [ ] Verificar que los 6 variants (`dcl_toon`, `_double`, `_alpha_clip`, `_alpha_clip_double`, `_alpha_blend`, `_alpha_blend_double`) heredan correctamente el uniform.

### Fase 6 — GDScript: ImpostorCapturer (autoload)
- [ ] `godot/src/decentraland_components/avatar/impostor/impostor_capturer.gd`:
  - Cola FIFO de captures pendientes.
  - `_process` toma 1/frame.
  - Si `body_url` disponible → `Global.content_provider.fetch_avatar_body_texture`. Si falla o vacio → `AvatarPreview` off-screen + `async_get_viewport_image(false, Vector2i(256, 512), 2.5)`.
  - Al obtener Image → `Global.avatars.set_impostor_texture(entity_id, image)`.
- [ ] Registrar autoload en `godot/project.godot`.

### Fase 7 — GDScript: avatar.gd LOD logic
- [ ] `enum LODState { FULL, MID, CROSSFADE, FAR }` + estado en `avatar.gd`.
- [ ] `_process` chequea distancia con phase `unique_id % 6`.
- [ ] `_compute_lod_state(dist)` segun algoritmo del plan.
- [ ] `_apply_lod_state`:
  - FULL: meshes visible, anim_tree on, particles on, click on, dither_alpha=1.
  - MID: meshes visible, particles off, click off, anim throttled, nickname viewport refresh reducido.
  - CROSSFADE: meshes con `dither_alpha=1-t`, layer alocado + textura cargada, `set_impostor_state(fade=t, tint=0)`.
  - FAR: meshes hidden, anim_tree off, `set_impostor_state(fade=1, tint=...)`.
- [ ] Excluir local player y avatares emoting recientemente.
- [ ] Hook en `async_update_avatar` y `update_colors` → invalidar + re-capture.
- [ ] Manejar nickname attached al bone (decidir: ocultar mesh-by-mesh o reparentear nickname).

### Fase 8 — Tools: benchmark
- [ ] `godot/src/tools/avatar_impostor_benchmark.gd` clonando patron de `fi_benchmark_runner.gd`. 50 avatares en grilla 5-50m, FPS con/sin impostors.

### Fase 9 — Verificacion
- [ ] `cargo run -- run` lobby manual.
- [ ] Benchmark synthetic, target +30% FPS.
- [ ] Casos: cambio de wearables/colores, AvatarShape, emote en lejano, mobile, first-person, XR.
- [ ] `cargo run -- run --itest` sigue verde.
- [ ] Settings persistente.

### Fase 10 — Polish
- [ ] Update `CLAUDE.md` con la nueva carpeta `decentraland_components/avatar/impostor/`.
- [ ] PR description con before/after, GIF cross-fade, FPS numbers.
- [ ] Capturar lessons aprendidas en `tasks/lessons.md`.

---

## Stop here
Plan aprobado via ExitPlanMode. Esperar luz verde para arrancar la implementacion.
