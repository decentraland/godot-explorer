# HudButton — normalized backgrounds

The 3 shared background textures, swapped by state in `hud_button.gd`:

- `bg_normal.png`   — default
- `bg_pressed.png`  — finger held
- `bg_selected.png` — toggle latched on

These are the **normalized** copies (VRAM Compressed, high quality — same import as the
joypad orbs). Don't hand-edit the `.import` files after normalization; drop new raw art in
`../_raw/` and re-run the normalize step instead.
