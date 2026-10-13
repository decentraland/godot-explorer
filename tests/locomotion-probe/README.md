# Locomotion probe

Headless regression probe for the player step-up / slope-limiter logic
(sprint25 #2753). Mirrors `godot/src/logic/player/player.gd`'s `_step_up`
(PhysX relocation test), the arming gates, and the positional slope limiter
against a synthetic matrix: step ladder (0.30/0.40/0.425 climb,
0.435/0.44/0.45 block), 4x0.15 staircase, 60° bevel, seam-lip, diagonal
corner approaches, gapped blocks, and 45/46/50/55/60/65° ramps.

Run it with the repo's Godot binary (from the repo root):

```
.bin/godot/godot4_bin --headless --path tests/locomotion-probe --script probe.gd
```

Expected: 16 case lines, one per obstacle. `max_y` is the peak height above
ground during the approach; `bails` counts step-up rejections by reason.
Climbable cases end past the obstacle with `max_y` at the top; blocking
cases keep `max_y < 0.05`. To check Godot Physics instead of Jolt, change
`physics/3d/physics_engine` in this folder's `project.godot` to `DEFAULT`.

If the player logic changes, port the change here — the probe caught the
diagonal-corner climb, the gapped-ladder Jolt divergence, and the 46°
recovery ratchet before they shipped.
