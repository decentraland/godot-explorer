# Movement & Collision Parity Audit — Godot vs Unity

**Issue:** [#905](https://github.com/decentraland/godot-explorer/issues/905) (research only — no behavior change ships from this audit)
**Date:** 2026-08-25 (v2 — validated twice by independent reviewer, corrections applied)
**Sources:** godot-explorer `main` (7f7302805c) · unity-explorer **`dev`** (6d8b5e966)
**Test scene:** `decentraland://open?position=-98,103&realm=jezter.dcl.eth`

All values verified against source on both repos (file:line per row), including a full adversarial re-validation pass. The 2026-08-24 baseline in #905 holds: 13/13 Godot claims exact.

> ⚠️ **Source branch note:** Unity references are against `dev`, not `main` — `main` lags significantly (pre-Unity-6, missing double-jump/glide). `dev` is the live reference for movement parity.

---

## 1. Gap table

### 1.1 Avatar capsule (tracked in its own issue — cross-ref only)

| Parameter | Godot | Unity | Delta |
|---|---|---|---|
| Height / radius | 1.5 / 0.25 — `godot/src/logic/player/player.tscn:16,15` | 1.6 / 0.3 — `CharacterObject.prefab:101-102` | Godot shorter + thinner |
| Center Y | 1.2 — `player.tscn:29` | 0.8 — `CharacterObject.prefab:107` | Godot floats 0.45 m above feet |
| Margin / skin width | 0.05 — `player.tscn:14` | 0.08 — `CharacterObject.prefab:105` | — |
| Extra shape | `SeparationRayShape3D` length 1.0779 at y=1.1 — `player.tscn:11,23-25` | none | Godot-only crutch bridging the 0.45 m gap |

### 1.2 Step / ground

| Parameter | Godot | Unity | Delta |
|---|---|---|---|
| Step offset | **none** — `CharacterBody3D` has no built-in step offset; no custom logic | 0.35, force-written every physics tick — `ApplySlopeModifier.cs:24` (prefab serializes 0.3, overridden at runtime) | Missing system. Godot's de-facto step clearance ≈0.45 is a side effect of the floating capsule, not step logic |
| Floor snap / downslope stick | `floor_snap_length = 0.2` — `player.gd:218` | Downslope raycast 0.45 jog / 0.55 run — `ApplySlopeModifier.cs:40,42`, settings `:83-84` | Different mechanism, shorter reach in Godot |
| Slope limit | unset → engine default 45° | 46° — `CharacterObject.prefab:103` | 1° — noise |
| Safe margin / contact offset | unset → engine default 0.001 | 0.08 skin width | Different model (Godot margin vs PhysX skin) |
| Physics engine · tick | Godot Physics · 60 Hz — `godot/src/config/graphic_settings.gd:235,243` (`physics_ticks_per_second = min(fps, 60)`) | PhysX · **manual/variable timestep** — `AdaptivePhysicsSettings.asset` Mode 2, `Physics.Simulate(Time.deltaTime)` per frame (`UpdatePhysicsSimulationSystem.cs:33`); `TimeManager.asset` no longer serializes a fixed 0.02 step (Unity 6 format) | Different solver **and** Unity no longer runs fixed 50 Hz — tick-math assumptions (e.g. "coyote = 8 ticks") are nominal, not guaranteed |

### 1.3 Jump / gravity

| Parameter | Godot | Unity | Delta |
|---|---|---|---|
| Coyote time | **Unreachable.** 0.2 s grace condition at `player.gd:391-395` only drives `avatar.land` animation (`:396`); ground-jump branch `player.gd:455-470` sits in an `elif` after `elif not on_floor:` (`:390`), so it never fires off-ground. Symptom note: off-ledge jump input is now often swallowed by the glide-open gate (`:425-433`) | 0.15 s ≈ nominal 8 ticks — `ApplyJump.cs:54` (`RoundToInt(0.15/0.02)`) | Parity bug |
| Jump input buffer | 0.15 s — `player.gd:9` | 0.15 s window — `JumpTrigger.cs:8-9`, `UpdateInputJumpSystem.cs:76` | ✅ parity |
| Gravity | single 10 m/s² — `player.gd:58` | asymmetric: descent 10, ascent ×4 = 40 — `ApplyGravity.cs:41-42` (`Gravity −10 × JumpGravityFactor 4`) | Different arc: Unity jumps snap up, fall soft; Godot is symmetric |
| Long jump (hold) | none | 0.5 s window ×0.5 gravity → ascent 20 — `ApplyGravity.cs:37-38`, settings `:74-75` | Missing system |
| Jump height | 1.8 jog / 1.8 run, fixed — `player.gd:59-60` (`sqrt(2·h·g)` at `:62,465`) | 1.0 jog / 1.5 run, lerped by horizontal speed — settings `:19-20` | Godot jumps higher; not speed-scaled |
| Air jump (double jump) | 1 · 2.0 m · 0.2 s delay · 8.0 fixed impulse — consts `player.gd:8,11-13`, impl `:376-386` | 1 · 2.0 m · 0.2 s delay · impulse **max(8, current horizontal speed)** — `ApplyJump.cs:76,99,127,148-186`, settings `:77-82` (`AirJumpCount 1`, `AirJumpHeight 2`, `AirJumpDelay 0.2`, `AirJumpDirectionChangeImpulse 8`). Gated by feature flag `FeatureId.DoubleJump` — `UpdateInputJumpSystem.cs:53,68` | ✅ values parity; impulse differs at high speed (Unity preserves momentum, Godot clamps to 8) |
| Jump cooldown | 0.3 s — `player.gd:10`, enforced `:394,414,459` | 0.3 s — `CooldownBetweenJumps` settings `:81`, enforced `ApplyJump.cs:80-83` | ✅ parity |

### 1.4 Glide (was omitted in v1 — both engines have it)

| Parameter | Godot | Unity | Delta |
|---|---|---|---|
| System | FSM in `player.gd` — consts `:14-20`, FSM `:34-38`, open/close `:421-454`, horizontal cap `:523-530`, landing force-close `:483-485` | `ApplyGliding.cs` (fall clamp `:55`), `GlideState`, `GliderPropControllerSystem`, wired `CharacterMotionPlugin.cs:127`; feature flag `FeatureId.Gliding` | both present |
| Max fall speed | 1.0 — `player.gd:14` | 1 — settings `:85`, `ApplyGliding.cs:55` | ✅ parity |
| Horizontal speed | cap 6.0 — `player.gd:15,:523-530` | `GlideSpeed 6` as speedLimit — settings `:83`, `CalculateSpeedLimitSystem.cs:36` | ✅ value, different mechanism |
| Min ground distance to open | **1.0** — `player.gd:16` | **0.2** — settings `:84`, `ApplyGliding.cs:27` | ⚠️ 5× mismatch — Godot refuses glide near ground |
| Jump→glide interval | 0.5 — `player.gd:17` | 0.5 — settings `:87`, `ApplyGliding.cs:39` | ✅ parity |
| Glide cooldown | **0.6** — `player.gd:18` | **0.2** — settings `:88`, `ApplyGliding.cs:40` | ⚠️ 3× mismatch |
| Trigger model | toggle-press; walking off a ledge can open glide without double jump (`player.gd:421-433`, deliberate divergence comment) | hold-to-glide, requires `JumpCount > MaxAirJumpCount` (`ApplyGliding.cs:25-31`) | different input model (partially deliberate) |
| Wind response | none | `GlideWindResponse 1.5` — settings `:86`, `ApplyExternalForce.cs:29` | Missing in Godot |
| Stun interaction | landing force-closes glider | fall-height tracker resets while gliding — `StunCharacterSystem.cs:53-56` | different |

### 1.5 Speed / acceleration / air

| Parameter | Godot | Unity | Delta |
|---|---|---|---|
| Walk / jog / run | 1.5 / 8 / **11** — `player.gd:55-57`, mirrored in `lib/src/godot_classes/dcl_locomotion_settings.rs:7-9` | 1.5 / 8 / **10** — settings `:15-17` | Run speed delta |
| Acceleration | none — velocity assigned directly (`player.gd:513-514`); only direction smoothing at 8/s | ground lerp 20→25, air lerp 15→20 over 0.5 s curve — `GetAcceleration` in `ApplyCharacterMovementVelocity.cs:72-78`, settings `:68-72` | Missing system |
| Deceleration | `move_toward(v, 0, walk_speed)` per physics tick, **step not scaled by dt** — `player.gd:520-521` ⇒ 90 m/s² whenever physics runs at 60 Hz, slower at lower tick rates | `SmoothDamp` with `StopTimeSec` — `ApplyCharacterMovementVelocity.cs:42,54`. Note: asset sets `StopTimeSec = 0` (degenerate — instant stop) | Different; Godot's is tick-rate-dependent (latent bug), Unity's is effectively instant |
| Air drag | none on locomotion velocity (only `external_velocity` scene impulses get viscous drag — const `player.gd:27`, block `:607-611`, `EXT_ENV_DRAG 1.5`) | quadratic horizontal drag, effective coeff `AirDrag 0.05 × JumpVelocityDrag 4 = 0.2` — `ApplyHorizontalAirDrag.cs`, called at `CalculateCharacterVelocitySystem.cs:137` (note: `ApplyAirDrag.cs` still exists but is dead code) | Missing system |
| Air control | full, identical to grounded | reduced — separate air accel pair, `MoveTowards` instead of snap | Missing system |

### 1.6 Slope / edge / wall / landing / platforms

| System | Godot | Unity | Delta |
|---|---|---|---|
| Slope velocity modifier | none | 1.35× downhill → 1.0 flat → 0.65× uphill curve over ±55° — `ApplyCharacterMovementVelocity.cs:21`, settings `:104-136` | Missing system |
| Steep-slope slide | none (engine default `move_and_slide` only) | gravity tilted along slope when `IsOnASteepSlope` — `ApplyGravity.cs:20-29`. Steep flag triggers at `slopeLimit` = 46° (`ApplyEdgeSlip.cs:52-53`) | Missing system |
| Edge slip | none | spherecast down from center (radius = capsule radius, dist = height×0.6) — `ApplyEdgeSlip.cs:28-31`; slip works via gravity-direction tilt (`:71`); `NoSlipDistance 0.1` / `EdgeSlipSafeDistance 0.4` — settings `:94,:96` | Missing system. Note: `EdgeSlipSpeed 2` is **dead config** (no consumers) — do not replicate |
| Wall slide | none | capsulecast forward 0.5 m, move multiplier lerped to 0 by wall dot — `ApplyWallSlide.cs:29-42`, settings `:137-138` | Missing system |
| Hard-landing stun | **present** — `hard_landing_cooldown` from scene locomotion settings; timer freezes input and decelerates — `player.gd:304-308,475-476` | stun on fall > 8 m, lasts 0.75 s, velocity zeroed — `StunCharacterSystem.cs:44-80` (trigger `:66`), `ApplyVelocityStun.cs:12`, settings `:92-93` | Gap is the trigger: scene-driven cooldown vs fall-height physics rule |
| Moving platforms | **partial** — scene GLTF colliders switch to `KINEMATIC` when their entity moves (`gltf_container.gd:366-384`; Rust signal `transform_and_parent.rs:93`, registry `lib/src/scene_runner/scene.rs:278`), but `CharacterBody3D` platform-follow properties are engine defaults; no platform-delta logic | spherecast down 0.3 + radius, 2-frame ungrounded buffer, applies platform delta + rotation — `CharacterPlatformSystem.cs:19-73`, `PlatformRaycast.cs:13-41`, delta applied in `PlatformSaveLocalPosition.cs:28-34` | Missing the follow half |

### 1.7 Collider generation

| Parameter | Godot | Unity | Delta |
|---|---|---|---|
| GLTF scene colliders | trimesh (`ConcavePolygonShape3D`) per `MeshInstance3D` — `lib/src/content/gltf/scene.rs` (`create_trimesh_collision()` at `:169`) | `MeshCollider` (trimesh, convex=false, `Physics.BakeMesh`) — `ConfigureGltfContainerColliders.cs:93-100` | ✅ same approach |
| Backface collision | **only for non-planar meshes** — `scene.rs:202`; planar = AABB thinner than 0.01 on any axis (`PLANAR_THICKNESS_THRESHOLD` `:219`, checks `:227-229`) | always double-sided (PhysX MeshCollider) | **Parity bug.** Direct cause candidate for #1529 and #1203: thin geometry (floors, ramps) is one-way in Godot |
| Primitive colliders | Box→`BoxShape3D`, Sphere→`SphereShape3D`, Cylinder→`ArrayMesh` trimesh (fallback `CylinderShape3D`), Plane→`BoxShape3D` size.y=0 — `lib/src/scene_runner/components/mesh_collider.rs:230-310` | Box→`BoxCollider`, Sphere→`SphereCollider`, Plane→`BoxCollider`, Cylinder→`MeshCollider` — `InstantiatePrimitiveColliderSystem.cs:27-32` | ✅ parity |
| Default SDK collider layer | `CL_POINTER \| CL_PHYSICS` (3) — `mesh_collider.rs:230` | both layers → `Default` (0) — `PhysicsLayers.cs:103-105` | equivalent routing |
| Player collision mask | **layer 4 (`CL_PLAYER`), mask 2 (`CL_PHYSICS`) only** — `player.tscn:19-20` | controller layer 16 collides with `Default, Floor, InvisibleColliders, CharacterController, CharacterOnly, SDKAvatarTriggerArea` — `DynamicsManager.asset:20` | **Parity bug.** Unity player collides against 6 layers incl. invisible colliders; Godot player walks through anything not on `CL_PHYSICS` |
| Layer model | 19 named physics layers (`project.godot:217-235`): 6 semantic (CL_POINTER, CL_PHYSICS, CL_PLAYER, CL_CLICKABLE_AVATAR, CL_AVATAR_MODIFIER_AREA, CL_CAMERA_MODE_AREA) + CL_RESERVED2-6 + CL_CUSTOM1-8 | SDK mask routing: ClPhysics-only→`CharacterOnly`(17), ClPointer-only→`OnPointerEvent`(9), both→`Default`(0), custom→`SDKCustomLayer`(26), **avatar-only→`SDKAvatarHit`(20)** — `PhysicsLayers.cs:101-137` (`IsAvatarOnlyMask` `:56-59`) | Godot lacks an avatar-only collider route |

---

## 2. Classification

### Parity bugs (Godot behavior wrong vs reference)

| # | Gap | Evidence | Symptom link |
|---|---|---|---|
| B1 | Coyote time unreachable — elif-chain routes off-ground jumps away from ground-jump branch (input often swallowed by glide-open gate instead) | `player.gd:390` vs `:455-470` | jump feels unresponsive at ledges |
| B2 | One-way thin colliders (AABB < 0.01) — Unity is always double-sided | `scene.rs:219,227-229` | #1529, #1203 — falling through thin floors/ramps |
| B3 | Player mask = `CL_PHYSICS` only; Unity collides with 6 layers incl. invisible colliders | `player.tscn:20` vs `DynamicsManager.asset:20` | walking through scene blockers |
| B4 | Deceleration `move_toward` step not dt-scaled — 90 m/s² at 60 Hz tick, weaker at lower tick rates | `player.gd:520-521` | inconsistent stop feel |
| B5 | Glide min-ground-distance 1.0 vs 0.2 — Godot refuses to open glider near ground where Unity allows it | `player.gd:16` vs settings `:84` | glide fails close to rooftops/ledges |
| B6 | Glide cooldown 0.6 vs 0.2 | `player.gd:18` vs settings `:88` | glider feels sluggish to re-open |

### Missing systems (exist in Unity, nothing to fix in Godot — build)

| # | System | Unity source |
|---|---|---|
| M1 | Step offset (0.35, per-tick write) | `ApplySlopeModifier.cs:24` |
| M2 | Slope velocity modifier (1.35×↓ / 0.65×↑) | `ApplyCharacterMovementVelocity.cs:21` |
| M3 | Steep-slope slide (gravity tilt > 46°) | `ApplyGravity.cs:20-29` |
| M4 | Edge-slip spherecast (gravity-tilt slip; `EdgeSlipSpeed` config is dead — skip it) | `ApplyEdgeSlip.cs` |
| M5 | Wall slide | `ApplyWallSlide.cs` |
| M6 | Moving-platform follow (delta + rotation) | `CharacterPlatformSystem.cs`, `PlatformSaveLocalPosition.cs` |
| M7 | Acceleration curves (ground 20→25 / air 15→20) + reduced air control | `ApplyCharacterMovementVelocity.cs:72-78` |
| M8 | Quadratic air drag (coeff 0.2) | `ApplyHorizontalAirDrag.cs` |
| M9 | Long jump (hold: 0.5 s ×0.5 gravity) | `ApplyGravity.cs:37-38` |
| M10 | Asymmetric gravity (ascent ×4) | `ApplyGravity.cs:41-42` |
| M11 | Fall-height landing stun (8 m → 0.75 s) — Godot has scene-driven stun only | `StunCharacterSystem.cs` |
| M12 | Glide wind response (1.5) | `ApplyExternalForce.cs:29` |
| M13 | Avatar-only SDK collider layer route (`SDKAvatarHit`) | `PhysicsLayers.cs:123-127` |

### Unconfirmed deviations (differ; no evidence of deliberate design decision found — needs product context)

| # | Parameter | Godot | Unity |
|---|---|---|---|
| D1 | Run speed | 11 | 10 |
| D2 | Jump height | 1.8 fixed | 1.0 jog / 1.5 run, speed-lerped |
| D3 | Downslope stick | snap 0.2 | raycast 0.45 / 0.55 |
| D4 | Slope limit | 45° (engine default) | 46° |
| D5 | Hard-landing stun trigger | scene locomotion settings | fall-height physics rule |
| D6 | Air-jump impulse | fixed 8.0 | max(8, current horizontal speed) |
| D7 | Glide trigger model | toggle-press, ledge-walk-off opens (has deliberate-divergence comment) | hold-to-glide after air jumps exhausted |

### Parity confirmed

Jump buffer 0.15 s · air jump values (1 · 2.0 m · 0.2 s · 8 base impulse) · jump cooldown 0.3 s · walk/jog 1.5/8 · glide fall-speed 1.0 / speed 6 / jump→glide interval 0.5 · GLTF trimesh approach · primitive collider shapes.

### ⚠️ Unity dead settings — do NOT copy values blindly

Replicate **behavior**, not the asset: `MinSlopeAngle 50` unused (real steep threshold is slopeLimit 46°) · `EdgeSlipSpeed 2` unused (slip is gravity-tilt) · `StopTimeSec = 0` makes SmoothDamp degenerate (Unity effectively stops instantly) · `GroundDrag 0.5`, `MinImpulse 1`, `CharacterControllerRadius 0.4` declared but never consumed · `ApplyAirDrag.cs` is a dead file (live one is `ApplyHorizontalAirDrag.cs`). Prefab serializes `StepOffset 0.3` but code overwrites with 0.35 every tick. `MaxSlopeAngle 80` **is** consumed (wall slide, slide animation blend, conditional rotation) — just not for the steep-slope trigger.

---

## 3. Repro checklist (test scene `-98,103`, jezter.dcl.eth)

Derived from code — not yet executed live. Run Godot and Unity side by side; each step lists expected Unity behavior vs current Godot behavior.

| Gap | Steps | Unity (expected) | Godot (current) |
|---|---|---|---|
| B1 coyote | Walk off a low ledge (< 1 m drop), press jump within ~0.15 s | jump fires | nothing — input swallowed (off high ledges the glider opens instead) |
| B2 one-way | Find thin floor/ramp geometry (elevated platforms); approach from below | blocked from both sides | pass through from below, land on top |
| B3 mask | Walk into scene-placed invisible blocker walls | blocked | walk through |
| B4 decel | Release move keys at full run; compare stop at stable 60 fps vs under lag (drops physics tick rate) | near-instant stop, rate-independent | stop distance varies with tick rate |
| B5/B6 glide | Double-jump near a rooftop and try to open glider below 1 m altitude; then close and re-open quickly | opens above 0.2 m; re-opens after 0.2 s | refuses below 1.0 m; re-opens only after 0.6 s |
| M1 step | Walk into 0.35–0.5 m ledges (stairs, curbs) | steps up smoothly | blocked or requires jump |
| M2 slope mod | Run up vs down the longest ramp | 0.65× uphill / 1.35× downhill | constant speed both ways |
| M3 steep slide | Stand on > 46° slope | slide down | stand still |
| M4 edge slip | Stand with > 50% of capsule past a ledge | slips off (gravity-tilt driven) | stand mid-air |
| M5 wall slide | Run diagonally into a wall | movement killed along wall normal | slide along wall at full speed |
| M9 long jump | Hold jump through a run-jump arc | higher/longer arc | same arc regardless of hold |
| M10 asym gravity | Tap jump: compare rise vs fall feel | fast rise, soft fall | symmetric |
| D6 air jump | Air-jump while sprinting at full run | impulse preserves momentum (≥ current speed) | impulse clamped to 8.0 |
| D1/D2 speed+jump | Run-jump onto geometry reachable in Unity at run speed | clears 1.5 m @ 10 m/s | 1.8 m @ 11 m/s — different reachable set |

---

## 4. Salvage recommendations

### PR #943 (Rust locomotion rewrite + Jolt) — recommend: **drop code, keep the body**

- Closed unmerged 2026-04-19. Rust rewrite of locomotion is the wrong shape for incremental parity — locomotion lives in `player.gd` and is iterated there.
- Its body is an ADR-grade locomotion spec: **reuse it as the implementation checklist** for M1–M13 follow-ups.
- Never implemented step offset, air drag, slope modifier, edge slip, stun, platforms; left capsule center at 1.2.

### PR #1417 (capsule + snap + slope + coyote + Jolt cherry-pick) — recommend: **re-scope and reopen (without Jolt)**

- The minimal parity slice: capsule fix (belongs to the capsule issue), coyote fix (B1), snap/slope values (D3/D4).
- Re-scope = split Jolt out (see §5), land the GDScript-side fixes only. Small diff, high Playtime impact, directly addresses B1.

---

## 5. Jolt evaluation

**Context:** both #943 and #1417 bundled Jolt physics. Neither landed.

| | Godot Physics (current) | Jolt |
|---|---|---|
| Status | built-in, zero deps, 60 Hz configured | extension/ fork dependency, used by many shipped Godot titles |
| Trimesh robustness | known thin-geometry tunneling issues (B2-adjacent) | stronger CCD + contact handling; would likely reduce thin-collider symptoms but **does not fix B2** (one-way flag is our code, `scene.rs:219`, not the engine) |
| `CharacterBody3D` behavior | snap/slope quirks are scene-script territory | same API; behavior differences subtle |
| Cost | — | engine/extension bump across all platforms incl. mobile + VR; revalidation of every physics assumption in this audit |
| Parity argument | PhysX-like behavior is the reference; Jolt is closer to PhysX in contact generation | moderate win, not decisive |

**Recommendation:** do **not** bundle Jolt with parity fixes. Land B1–B6 + M-systems on Godot Physics first (they are solver-independent). Evaluate Jolt separately, only if thin-geometry tunneling persists **after** B2 is fixed. Bundling engine swap + behavior changes makes regressions unattributable — the failure mode that killed both prior PRs.

---

## 6. Follow-up candidates (not filed — pending team review)

Priority = Playtime impact × effort. Order suggested:

| Pri | Candidate | Addresses | Effort |
|---|---|---|---|
| P0 | Fix coyote time elif-chain | B1 | XS — reorder branches in `player.gd:455-470` |
| P0 | Double-sided colliders for thin geometry (drop or gate one-way flag) | B2, #1529, #1203 | S — `scene.rs:219,227-229`; validate perf of double-sided trimesh |
| P0 | Widen player collision mask to match Unity layer set | B3 | XS — `player.tscn:20` + layer audit |
| P1 | dt-scale deceleration | B4 | XS |
| P1 | Glide param alignment (min distance 0.2, cooldown 0.2) | B5, B6 | XS — two consts in `player.gd:16,18` |
| P1 | Asymmetric gravity + long jump + speed-lerped jump height | M9, M10, D2 | S — all in jump/gravity block |
| P1 | Step offset (custom; Godot has no built-in) | M1 | M — needs step-up raycast logic or Jolt-style char controller |
| P2 | Slope velocity modifier + downslope raycast | M2, D3 | S |
| P2 | Steep-slope slide + edge slip + wall slide | M3, M4, M5 | M — three small cast-based systems |
| P2 | Acceleration curves + air control + air drag | M7, M8 | M |
| P3 | Moving-platform follow | M6 | M |
| P3 | Fall-height stun (replace/augment scene-driven stun) | M11, D5 | S |
| P3 | Glide wind response + avatar-only collider layer | M12, M13 | S |
| — | Jolt evaluation (only after B2 lands) | §5 | spike |
| — | Resolve unconfirmed deviations with product | D1–D7 | conversation |

---

## Appendix — verification log

Two independent adversarial validation passes (k3 reviewer subagent), 2026-08-25:

- **Round 1** ran against a stale unity-explorer checkout (dev @ 2bc28559c): produced false positives on air jump and jump cooldown ("missing in Unity" — both exist on current dev, values match Godot exactly). Confirmed Godot 13/13 baseline values.
- **Round 2** (dev @ 6d8b5e966): confirmed air jump / cooldown parity; surfaced the **glide system on both sides** (omitted in v1) with two real param mismatches (B5/B6); corrected `MaxSlopeAngle` (consumed — not dead) and `EdgeSlipSpeed` (dead — v1 quoted it as live); found Unity physics now runs **manual variable timestep**, not fixed 50 Hz; layer-16 matrix drifted ("AllAvatars" gone, `SDKAvatarTriggerArea` in); `ApplyAirDrag.cs` dead (live: `ApplyHorizontalAirDrag.cs`); ~20 stale file:line refs corrected throughout.
- Godot baseline corrections vs #905: hard-landing stun exists (scene-driven) — issue listed it absent; moving platforms partial (KINEMATIC switch without follow) — issue listed it absent.
