# 06 — Decision-consistency review of the campaign plan

Reviewed: `04-campaign-plan.md` against `00-constraints.md`, `01-data-inventory.md`, `03-tooling.md`, plus the
raw runs it cites. The plan's statistics, metric definitions and invalidation rules are sound. The
disagreement is in one place and it is the load-bearing one: **the primary condition P1 cannot produce a
matched-light pair, and the plan's own data already says so.**

The existing shipped-config comparison (5+5, 17.9 vs 18.1 fps, p99 +11.9 %, p1 −10.6 %, CPU −10.3 %,
PSS −3.2 %) is not in dispute and does not need re-measuring. What is missing is one sentence that survives a
hostile reader: *"at one verified condition, 2.x paces worse than 1.x by X."* Everything below is about buying
that sentence for the least phone time.

---

## 1. Is P1 the best available comparison?

**No. P1's premise is contradicted by the runs it would replace.**

P1 states that the scene's `SkyboxTime` outranks the player setting *on both builds*, so `dynamic_skybox =
false` is inert and both arms see the scene's light. The measurements say otherwise:

| scenario | build | luma_mean | what that light actually is |
|---|---|---|---|
| `gp-hi-fixed-sky` (dynamic OFF) | 1.x | 104.6 / 107.4 / 108.8 | the **player setting** (`Morning` measures ≈ 105–112 separately) |
| `gp-hi-fixed-sky` (dynamic OFF) | 2.x | 75.5 / 77.6 / 72.6 | the **scene's own time** — 30 points away |
| `gp-hi-pair` (dynamic ON) | 1.x | 129.4 | the **world clock** (UTC × 12) |
| GP, dynamic ON, measured today | 2.x | 98.0 → 98.6 in 90 s; 102.3 after relaunch | the **scene's own time**, near-frozen |

Two stable arms, 30 points apart. With `dynamic_skybox = false`, 2.x obeys the scene and 1.x obeys the
setting; with it `true`, 2.x still obeys the scene while 1.x jumps to the world clock. **In Genesis Plaza the
two builds are never on the same clock.** No per-run luma gate repairs a systematic offset: P1 §2.6 would
mark every pair `pair_luma_mismatch` and abort the arm after two runs — the fallback path, with extra steps.

The alternatives, judged on "which produces the most credible sentence in a PR comment":

| option | matched light? | cost | verdict |
|---|---|---|---|
| **(a) Scene-free parcel (`100,100`), `dynamic_skybox = true`, windows started at a fixed UTC phase** | **Yes, structurally.** No scene to override, so both builds take `settings_target()` → `DclGlobalTime::get_world_time()` = `(utc_unix_s × 12) mod 86400`: a deterministic function of the wall clock with a 7200 s period. Start both arms in the same 2 s of that period and the sun agrees to ~24 in-game seconds. No proxy, no luma, no client change. | ~10 min/run | **Best available.** The sentence becomes "at identical in-game time, verified from the clock, 2.x spends X % of the window in ≥2-period stalls vs Y %". The independent variable is not arguable. |
| (b) GP with dynamic graphics ON, both builds | No (see table) | ~14 min/run | Descriptive only: "what a player sees", never the delta. |
| (c) Same build twice (2x vs 2x) | n/a | ~10 min/run | Not a comparison — a **noise floor**. Worth paying for (§2); not worth calling a result. |

The price of (a): `100,100` is empty, so it measures engine + shell baseline (34–40 fps, observed today), not
plaza content. That belongs in the first line of the result, not a footnote. But a matched baseline delta plus
an unmatched plaza description beats two unmatched plaza arms, because only the first is falsifiable.

**Recommendation:** make (a) the primary arm, demote GP to a descriptive tier, and use `dynamic_skybox = true`
everywhere. `false` is the one setting that can silently send 2.x to midnight (`try_to::<f64>()` on an `Int`
variant → `unwrap_or(0.0)`); never risk it in a measured arm when `true` has no such failure mode.

Concretely, replace §2.1/§2.7: primary `P1'` = parcel `100,100`, `dynamic_graphics_enabled = false`,
`graphic_profile = 3`, `dynamic_skybox = true`, window started when `int(time.time()) % 7200 <= 2`, invalid if
`(start_unix % 7200) > 5`. Gate on UTC phase, not on cross-build luma.

---

## 2. The single control with the best credibility per hour

**A null arm: the same build twice under the identical protocol — `2x` vs `2x`, n = 3 + 3, same UTC phase,
same gate, same window, the second block after the same interruption a real arm gets.**

Why this and not the alternatives:

- It bounds the quantity every reader actually doubts: how much of the delta could be harness, ordering and
  thermal drift rather than build. P1's ABBA order and `start_ap_c ≤ 1.0 °C` rule are *arguments* that the bias
  is small; the null arm *measures* it.
- It validates the two assumptions the campaign rests on — that the condition reproduces block to block at the
  same UTC phase, and that the app restart/install does not itself move the number.
- It fails usefully and early: if the null spread exceeds the 1x↔2x difference, the campaign knows at hour two
  that it must publish bands instead of a delta, instead of at hour seven.

Cost: 6 runs × ~9 min ≈ **54 min**, and if the second block reuses the same install (an app restart is what a
real run does anyway) there is no install cost at all. If 30 more minutes exist, run the same null for 1.x at
n = 2: that turns "the arms differ" into "the arms differ by more than each arm differs from itself".

---

## 3. If luma is confounded across builds, what replaces it?

`03-tooling.md` §4.1.3 is right: full-frame luma carries build-correlated UI bias, because the two shells draw
different chrome. The plan's `luma_crop` mitigates but does not remove it — today's data shows a **stable 30
point cross-build offset under nominally identical settings**, which is the signature of a systematic bias, not
of a light mismatch. Luma therefore cannot be the cross-build gate.

What is available today, no client change, ranked:

1. **The independent variable itself: UTC phase — structural, not measured.** Where no scene overrides the
   clock and `dynamic_skybox = true`, in-game time is `(utc_unix_s × 12) mod 86400` on both builds
   (`dcl_global_time.rs:10-27`). Record `start_unix` per run; require both members of a pair inside the same 5 s
   of the 7200 s period. This is a proof, and it costs one integer per run.
2. **The engine's own statement of the light (2.x only).** `rust-log=dclgodot::sky=debug` gives
   `SkyboxTime SDK control changed: active=true` plus the scene's `fixed_time`. Pilot only — 1.x has no
   equivalent — and never as a per-run gate.
3. **Within-build luma stability, as a gate only.** Each build must stay inside its *own* pilot band
   (observed: ±2 within a session, ±4 across a relaunch). That catches a modal, a wrong time, or a settings
   failure without pretending to compare chrome.
4. **Cross-build luma: report, do not gate.** Publish the offset as a finding ("2.x renders the same condition
   ~30 luma darker in the visible viewport even with HUD cropped").
5. **Modal presence and avatar count from the screenshot** — a stronger invalidator than luma, already in the plan.

**Consequence for the primary claim:** with (1) the claim is *stronger* than luma matching, because "same
in-game time" is exact. With only (3)+(4) — i.e. inside Genesis Plaza — the campaign cannot publish a
cross-build delta at all; it publishes two arm bands and says why. That asymmetry is the entire argument for
moving the primary arm off the plaza.

---

## 4. Value per minute, and the 3-hour version

| rank | arm | minutes | why it earns the slot |
|---|---|---|---|
| 1 | **P1' primary**, n = 3/build | 60 | only matched-light comparison available; produces the sentence |
| 2 | **N null** (2x vs 2x, n = 3 + 3) | 54 | bounds the noise; licenses or forbids every other delta |
| 3 | **P1' second block** (n = 6/build) | 60 | halves the CI; n = 6 is where p < 0.05 becomes reachable |
| 4 | **F0/F1/F2/F6 preflight** | 25 | prevents the three failure modes that voided the last pass |
| 5 | S1 shipped, n = 3/build | 78 | descriptive; the 5+5 shipped comparison already exists |
| 6 | S2 sun-sensitivity | 54 | the effect is already documented (26.87 → 14.06 fps over luma 86–117.6) |
| 7 | GP descriptive arm, n = 3/build | 84 | ecological validity, no delta |

**3 hours of device time → one claim plus one control (~170 min of runs, 30 min of slack):**

1. `F0/F1/F2/F6` preflight + pilot — 25 min.
2. `P1'` block A: 1.x, n = 3, parcel `100,100`, dynamic ON, UTC-phase windows — 30 min.
3. Install 2.x (4 min) → `P1'` block B: 2.x, n = 3, same phase discipline — 30 min.
4. `N` null: 2.x, n = 3, immediately after block B (no install) — 27 min.
5. Analysis and go/no-go (no phone needed) — 15 min.

Published: *"At identical in-game time on a scene-free parcel with profile High and dynamic graphics off, 2.x
spends X % of a 30 s window in ≥2-period stalls versus Y % for 1.x (n = 3 + 3, null-arm spread Z)."*
Genesis Plaza stays descriptive, cited from the existing runs.

If the null spread is wider than the 1x↔2x difference, the campaign stops and publishes bands plus the null —
still honest, still useful, and it cost two hours instead of nine.

---

## 5. The three most likely failure modes, with the early signal

1. **The light is not matched after all, discovered at analysis time.** Signal: the pilot's `start_unix`
   bookkeeping shows the arms slipping out of the 5 s UTC window, or the first pair sits 20+ luma apart.
   **Check after the 1x block, before the 2x install**: if `|Δluma_crop| > 15` at matched UTC phase, the parcel
   is not following the world clock as the code says (a parcel-level scene is running) → move to a parcel
   verified empty by `scene_id`, or abandon the delta for bands. Cost of finding out late: the 2x block + null.
2. **The settings do not survive the install and one arm silently runs another profile.** Signal: the plan's F6
   rule (two consecutive `"profile":3`) or any `Profile (downgrade|upgrade)` line. This voided 3 of 6 runs last
   time. **Check at every launch and stop the block on the first miss** — never "re-run that one later": later
   means a different thermal state and a different sun.
3. **Thermal drift makes the second arm slower for reasons unrelated to the build.** Signal: `start_ap_c`
   diverging past 1.0 °C, or a gate that never reaches 40 °C inside 1500 s (idle AP today was 49 °C at status
   LIGHT, so the plan's own gate is already at its limit). **Check the gate ledger after block B**: if 2x's
   median `start_ap_c` is > 1 °C above 1x's, re-run a block or publish the delta with the thermal offset quoted
   next to it and declared unresolvable.

Cheapest one to avoid rather than fix: **battery** — 13 % today, discharging, ~18 %/h drain measured
(32 % → 14 % in 60 min), so the 7.3 h plan needs ~130 % of a charge.

---

## 6. Operational rules for the operator

Battery and heat are the real budget; the plan's 7.3 h assumes a charged, cool phone, and this one is neither.

**Charging.**
- Nothing starts below **95 %**, charged screen-off and idle (~75–90 min from 13 %). Never start a block below
  60 %: a run that dies mid-window is worse than a run not taken.
- Charge only **between blocks**, app force-stopped, screen off, and **stop charging ≥ 10 min before** the first
  run of a block so battery heat has bled off. Charging during measurement adds 3–6 °C at the AP and voids the
  gate.
- Budget one ~40 min top-up in a 7 h campaign; a 3 h campaign fits in one charge.
- Record `level` at the start and end of every run; a run spanning a low-battery notification is void.

**Block length.**
- Blocks of **3 runs maximum** (~45 min), then **≥ 15 min** with the app closed. Do not chain blocks even if the
  gate passes: the gate measures the AP, not the chassis.
- **≤ 8 runs and ≤ 4 h wall clock per session.** Split over two days at block boundaries only, as the plan says.
- After every launch: dismiss the modal **once**, screenshot, confirm no modal before the window. One
  un-dismissed tutorial cost a run 30 luma last pass.

**Hot phone at the start of a block.**
- Do not wait inside the gate loop past 1500 s. Stop the app, screen off, **20–25 min** passive cooldown,
  re-baseline SKIN, then restart the block at run 1 — never resume mid-block, because the first runs of a block
  are the coolest and resuming at run 3 silently biases the arm.
- `Thermal Status >= 2` at any gate → stop for the day. Today it was 1 at idle: this campaign runs on a phone
  one notch throttled, and that sentence belongs in the report. Prefer early morning after an overnight charge.

**Rule the plan is missing:** one `campaign.json`, written before the first measured run and **appended to by the
harness, never by the operator** — design version, gate ledger, per-run flags. Last pass produced 59 run
directories, three schemas and two levels of `INVALID.md` because writes were informal.

---

## Recommended shape

**(a) 7 h available — two tiers, one delta.**
1. Charge to ≥ 95 % first (+90 min wall clock).
2. Preflight F0/F1/F2/F6 + pilot — 25 min.
3. `P1'` primary (parcel `100,100`, dynamic ON, UTC-aligned, profile High, dynamic graphics off): 1x n = 3,
   install, 2x n = 3, then 2x n = 3 and 1x n = 3 in a second ABBA block → **n = 6/build**, ~2 h.
4. `N` null: 2x vs 2x at matched phase, n = 3 + 3, ~55 min, folded into the ABBA as an extra 2x block.
5. GP descriptive arm, dynamic ON, n = 3/build, ~85 min, no delta.
6. Cut S1 and S2: the shipped comparison exists and the sun effect is already documented.

**(b) 3 h available — n = 3/build plus the null.** Preflight 25 min, 1x ×3 30 min, install, 2x ×3 30 min, null
2x ×3 27 min, analysis 15 min. One claim (matched-phase stall fraction, n = 3 + 3 with a measured null spread),
one control (the null), Genesis Plaza cited from existing data.
