# Lighting snapshots (day/night cycle regression — issue #2516)

Baseline captures for the day/night lighting regression test. They are
produced by the standalone lighting renderer
(`godot/src/tool/lighting_renderer/`), which renders a synthetic diorama
(ground + white box + shadow-casting pillar + capsule "avatar") under
`sky_high` at fixed times of day:

| file                       | time  | what it pins                                   |
|----------------------------|-------|------------------------------------------------|
| `lighting_night_00h.png`   | 00:00 | moon light on, scene legible (not crushed)     |
| `lighting_morning_08h.png` | 08:00 | visible ground shadow, neutral grade           |
| `lighting_midday_12h.png`  | 12:00 | reference look — must not change               |
| `lighting_afternoon_17h.png` | 17:00 | visible ground shadow, neutral grade         |
| `lighting_golden_18h.png`  | 18:00 | low-sun clamp keeps a readable shadow          |

## Running

```bash
cargo run -- test-tools
```

This regenerates the captures into `comparison/` and pixel-diffs them against
the PNGs in this folder with a **0.95** similarity threshold (stricter than
scenes/avatars at 0.90, because lighting regressions are subtle color-grade
shifts).

## Updating baselines

When a lighting change is intentional:

```bash
cargo run -- test-tools   # generates comparison/*.png
mv tests/snapshots/lighting/comparison/*.png tests/snapshots/lighting/
git add tests/snapshots/lighting/*.png
```

## Manual renderer run

```bash
cargo run -- run -- --lighting-renderer   # PNGs land in godot/output/
```
