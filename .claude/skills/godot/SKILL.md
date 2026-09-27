---
name: godot
description: Use when editing Godot files in this repo — GDScript (.gd), scenes (.tscn) or resources (.tres) under `godot/`. Covers the .tscn/.tres serialization rules that differ from GDScript, instance property overrides, GDScript pitfalls this codebase has hit, and the repo's validation commands. Trigger on hand-editing a .tscn/.tres, "failed to load" resource errors, or GDScript parse errors.
---

# Godot files in this repo

The project runs a custom Godot 4.6 fork through xtask — `cargo run -- run` runs the client,
`cargo run -- run -e` opens the editor. There is no `godot` binary on `PATH`. UI placement lives in
the `godot-ui-components` skill, user-facing text in the `i18n` skill.

## .tscn / .tres are serialization, not GDScript

```
[ext_resource type="Script" path="res://item.gd" id="1"]

[resource]
script = ExtResource("1")   # not preload()
item_name = "Sword"         # not var item_name = "Sword"
```

- External files: declare `[ext_resource ...]` and reference `ExtResource("id")`; inline resources:
  `[sub_resource ...]` + `SubResource("id")`. Never `preload()`; every id is declared before use.
- No `var` / `const` / `func` — properties are bare `name = value` lines.
- Arrays serialize by the property's declared type: a typed `Array[Resource]` property is written
  `Array[Resource]([SubResource("a")])`, an untyped `Array` is plain `[...]`. Match the script.
- Keep structural .tscn changes (reparenting, editing instanced children) to the editor — hand edits
  there break when the editor re-saves. Simple property edits by hand are fine.

## Instance property overrides

An instanced scene uses its defaults (often `null`) for child properties unless overridden with the
`index` syntax — a pickup with a null `item_resource` fails silently:

```
[node name="KeyPickup" parent="." instance=ExtResource("6_pickup")]

[node name="PickupInteraction" parent="KeyPickup" index="0"]
item_resource = ExtResource("7_key")
```

## CPUParticles3D `color` / `color_ramp`

Particle colors only show when the mesh's material has `vertex_color_use_as_albedo = true`:

```
[sub_resource type="StandardMaterial3D" id="StandardMaterial3D_1"]
vertex_color_use_as_albedo = true

[sub_resource type="SphereMesh" id="SphereMesh_1"]
material = SubResource("StandardMaterial3D_1")
```

## GDScript

- `:=` can't infer a type from a `Variant` — properties on results of `get_edited_scene_root()`,
  `get_node()`, `Dictionary.get()`, `Array.front()` and the like. Annotate explicitly:
  `var scene_path: String = root.scene_file_path`.
- Use callables, not method-name strings: `_apply_current.call_deferred()`, not
  `call_deferred("_apply_current")` — the string form skips parse-time checks and breaks on rename.

## Validate

```bash
cargo run -- check-gdscript      # every script must parse
gdformat godot/ && gdlint godot/ # dcl-regenesislabs fork of gdtoolkit
cargo run -- import-assets       # after adding assets or moving scripts
```
