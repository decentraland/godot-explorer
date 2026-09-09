# HudButton — raw texture drop (un-normalized)

Drop the raw exports here. This folder has a `.gdignore`, so **Godot never imports
anything in it** — no `.import` clutter, no VRAM warnings. It's the staging area; the
normalized copies live one level up in `../textures/` and `../icons/`.

## What to drop

**3 shared backgrounds** (used by every HudButton, swapped by state) — square PNGs,
high-res (≈600×600 like the joypad orbs is ideal so they stay crisp at every size):

- `bg_normal.png`   — default
- `bg_pressed.png`  — finger held down
- `bg_selected.png` — toggle latched on (only shows on toggle buttons)

**Per-button icons** (one glyph each; the icon does NOT change with state):

- `chat.png`
- `emote_wheel.png`
- `chat_flip.png`
- `discover.png`

SVG icons are fine too — say so and I'll import them as `DPITexture` instead of VRAM.

## What "normalize" does (I run this once the files are here)

1. Copy each file to `../textures/` (backgrounds) or `../icons/` (icons).
2. Set the `.import` to **VRAM Compressed, high quality** — matches the orb textures
   (`compress/mode=2`, `compress/high_quality=true`, `mipmaps/generate=true`), which is
   what avoids gradient banding and what the CI check (`tests/check_asset_imports.py`)
   requires. SVGs go to `DPITexture` instead.
3. `cargo run -- import-assets` to reimport.
4. Wire the normalized textures into `../hud_button.tscn` (or into each button's instance).

Nothing here is meant to ship — the raw files can stay uncommitted; only the normalized
copies in `../textures/` and `../icons/` are used by the game.
