---
name: Prefer built-in button states over code overrides
description: Use toggle_mode and set_pressed_no_signal to drive button appearance, not add_theme_stylebox_override
type: feedback
---

Use the button's built-in state system (normal/pressed/hover styles in the scene) to handle color changes. Set `button_pressed` via `set_pressed_no_signal()` to reflect the current state.

**Why:** Dynamically overriding styles in code is hacky and bypasses the existing scene-defined style system.

**How to apply:** When a button needs different colors for two states, define both `normal` and `pressed` StyleBoxFlat in the .tscn, enable `toggle_mode = true`, and use `set_pressed_no_signal()` in code to switch between them.
