I have what I need. No write tool is available to me (read-only toolset), so the complete artifact is below for the runtime to persist at `/tmp/plan-3206/05-review.md`. Repository untouched.

```markdown
# 05 — Hostile review of `04-campaign-plan.md`

Reviewer position: this plan has to survive an engineering team that will re-derive every number.
Citations are to `/tmp/plan-3206/04-campaign-plan.md` by line, and to repo files by path:line.
Nothing here is a measurement. I re-computed what was re-computable from the tree and say where the
plan's number is wrong.

---

## 0. What is actually correct (so the rest reads as targeted, not blanket)

- **MDR table (§3.2, l.194-201) is arithmetically right.** `15.7·(CV/Δ)²` with `2(1.96+0.84)²=15.68`;
  `√(15.7/3)=2.29`, `√(15.7/4)=1.98`, `√(15.7/6)=1.617`. Every cell of the three CV columns checks
  (22.9/34.3/45.8, 19.8/29.7/39.6, 16.2/24.3/32.4).
- **Mann–Whitney floors are right**: `2/C(6,3)=2/20=0.100`, `2/C(8,4)=2/70=0.0286`,
  `2/C(12,6)=2/924=0.00216`.
- **The 435-minute sum is internally consistent**: 169+78+54+82+32+20 = 435 exactly. (The *components*
  are wrong; see §2.)
- **The ABBA position balance is right**: 1.x at positions 1-3,10-12 → mean 6.5; 2.x at 4-9 → mean 6.5.
- **Cut-order arithmetic is right**: −54, −26 (4×13=52 vs 78), −56 (4×14.1=56.4).
- **The run order does survive the day split** (day 1 = A1,B1; day 2 = B2,A2 is still ABBA, and 2.x is
  already installed at the start of day 2).
- **R12 is correct**: `screen.png` is captured after the trace is pulled
  (`scripts/bench/2x-vs-1x/bench_android.py:252-255`), so the luma shot does not perturb the window.
- **The additive-change analysis of `frames.csv` consumers is correct**: only `bench_android.py` writes
  it, and no other script reads it (`grep frames.csv scripts/bench/2x-vs-1x/*.py` → one hit).

---

## 1. Arithmetic and statistics

### P0-1. `pct_window_in_stalls` is a rare-event proportion; at n=6 the campaign cannot detect anything
The plan makes it **primary** (§5 l.320, §1b B1 l.43). Derive its per-run noise from the plan's own
reference run: `gp-hi-fixed-sky/2x-sky-r1` has `frames=458`, `span_s=29.86`, `p50=66.69` (modal 8
periods), `p99=83.13`, `max=83.69`. A stall is `periods ≥ modal+2` = ≥83.33 ms, i.e. the top ≈1 % of
intervals → **≈4.6 events per run**. A count of ~5 events has relative SE `1/√5 ≈ 45 %`; for
`1x-sky-r1` (`frames=528`, `p50=58.29`=7 periods, `p99=75.08`=9 periods) the count is also ~5.
So per-run CV on the primary metric is ≈45 %, not the 3.7-5.8 % borrowed from `fps_avg`, and
`MDR(6) = 45 % × 1.62 ≈ 73 %`. **The campaign's own primary metric can only detect a 73 % difference.**
This is the same rarity argument the plan uses to refuse p99.9 at 900 frames (§1c.4) — applied to the
metric it elevates to primary — and it invalidates the whole n=6 rationale.
Fix: make `on_modal_frac` (a fraction over ~500 samples, CV ≈2-4 %) or `interval_p99_ms` primary, or
measure the real CV of `pct_window_in_stalls` on the 15 usable run dirs before fixing n.

### P0-2. `§8.10 pid_before = 0` invalidates every run, including the plan's own F2 reference run
`docs/bench/2x-vs-1x-3206/android-a54/gp-hi-fixed-sky/1x-sky-r1/meta.json:9` → `"pid_before": 27411`;
`2x-sky-r1/meta.json:9` → `"pid_before": 20251`. `run_paired.sh` launches the app, settles, and only
then calls `bench_android.py window` (`run_paired.sh:233-279`), so `pid_before` is *always* the live pid —
which is the same pid the plan's profile check uses. Under rule 10 as written, 0 of the 15 useful runs
and 0 of the 24 planned runs can be `valid`, and `manifest.py check` would reject F2's reference file.
Fix: replace with "the `[RenderReady]`/`Profile` lines used as evidence carry this run's `pidof` pid".

### P1-3. n=6 justification contradicts the source it cites (l.201)
"n = 6 is also the smallest n that makes an exact two-sided p < 0.05 arithmetically possible with a
comfortable margin (0.0022)." The plan's own table (l.197) prints `4 → 2/70 = 0.029`, and
`03-tooling.md` §6.1 states "With 4 vs 4 it is 2/70 = 0.029". Smallest n for p<0.05 is **4**; for p<0.01
it is **5** (2/252=0.0079). Only "smallest n with a two-sided floor below 0.005" is true of 6. Rewrite or drop.

### P1-4. The effect size used to size n is the confound the campaign exists to remove (l.196-198)
"`gp-hi-fixed-sky` gives 1.x 17.48/17.93/18.82 … vs 2.x 15.18/14.42/13.47 …, ratio 0.81 (−19 %)".
Those two arms are **not light-matched**: `luma_mean` 104.6/108.8/107.4 (1.x) vs 72.6/75.5/77.6 (2.x),
a 27-36 point gap — 5-7× the plan's own ±5 tolerance. `00-constraints.md:2` says "the sun moves the
result more than the build does". So the −19 % may be 0 % after matching, and n=6 is sized on a number
the plan's §2.6 would forbid publishing. Correct statement: the only usable effect-size prior is the
within-build spread (CV 3.7-6.0 %), for which MDR(4)=CV·1.98 → 12 %, i.e. **n=4 is sufficient**, and the
−19 % cross-build figure must be labelled "unmatched, not evidence of an effect".

### P1-5. Time budget: the sum is 435, the components are not
Recomputed per-run cost, applying the plan's *own* universal floor "≥ 480 s since the previous window's
end" (§4.2 cond. 4, which the plan says applies to **a run**, l.262) plus the plan's own overhead line
(0.7 launch + 0.4 settle + window + 0.8 capture + 0.2 manifest):

| arm | plan | corrected | arithmetic |
|---|---|---|---|
| P1 (12) | 14.1 → 169 | 14.2 → 170 | gate 12 + 2.2 overhead (this one is right) |
| S1 (6) | 13 → **78** | 14.35 → **86** | S1 has a *longer* window (45 s) than P1 yet is budgeted 1.1 min cheaper |
| S2 (6) | 9 → **54** | 10.4 → **63** | 9 min < the 8-min floor + 2.7 min overhead, i.e. arithmetically impossible under §4.2 |
| preflight | 82 | 82 | ok for F0-F6, but **not** for the F4b fallback (+42 min, §2.7) |
| installs | 8 → 32 | **10 → 40** | §11's F3(2x) → F4(both) → F5 F6(both) order forces 4 preflight installs, not 2 (l.223) |
| soak | 20 | 20 | undefined in §4.1; in the budget anyway |
| A3 (10-min leak probe × 2 builds) | — | **+22** | claimed in §1a l.34, no budget line, no metric in §5, no table in §7.3 |
| A5 (20 navigations, 2.x) | — | **+20** | claimed in §1a l.36, no budget line |
| **total** | **435** | **≈ 483 (8.0 h)** | before any re-run |

Add the observed invalidation rate for contingency: 3 of 6 in `gp-hi-pair`, 1 of 3 in
`sky-fixed-midday` (01-data-inventory §4). At ~25 % re-runs of 24 windows the real cost is **≈ 590 min
(9.8 h)**, not 7.3 h. The budget has **zero** contingency and does not mention re-runs at all.

### P1-6. The cut order silently breaks the design
"Cut P1 to n = 4 (−56 min)" (l.236) is not a count change: 4 per build cannot fill two 3-run blocks, so
the ABBA sequence, the install boundaries and the mean-position-6.5 balance all have to be re-derived.
Also, the floor "never below 4 + 4" (l.238) is consistent with 1c.8 but makes the *cut* sequence unable
to reach its own floor without a redesign. State the re-derived order for each cut, or replace cut 3
with "cut preflight F4b and ship bands".

### P2. Minor numeric corrections
- l.198: "MDR(6) is 6-9 %" → CV 5.97 % (sample SD over 15.18/14.42/13.47) gives **9.7 %**; the range is
  6.1-9.7 %, i.e. "6-10 %".
- l.196: 2.x CV "5.8 %" → recompute gives **5.97 %** (sample) / 5.3 % (population); state which.
- §2.6 l.144: "the condition changes we must exclude are ≥ 20 points" while the cited gap is
  27.0-36.2 points. Either is fine, but the sentence and its evidence disagree.

---

## 2. Level of effort versus value

### P0-7. Two of the three arms are re-measurements of data already on disk — offline
The device-time bill is 301 of the 435 minutes (P1 169 + S1 78 + S2 54). Check against the tree:

- **P1's 2.x arm already exists.** `android-a54/gp-hi/2x-gp-hi-r1..r5` are the same condition
  (dynamic OFF, profile HIGH, Genesis Plaza) with `trace.perfetto-trace`, `screen.png`, `cpu.csv`,
  `mem.csv`, `thermal.csv` (verified: `ls gp-hi/2x-gp-hi-r1` → all present). What they lack is
  `state.json` (i.e. `luma_mean`) and a run-dir `launch.log`. Both are recoverable **without the
  device**: `luma_crop` is a pure function of the existing `screen.png`, and `window_ms` is a re-query
  of the existing trace (`af.dur/1e6`). §1c.2 refuses `gp-hi` for "no state.json, no per-run profile
  evidence" — the second half is wrong (01-data-inventory §1 records profile 3 for all 5 from the
  scenario-level logs), and the first half is a 30-line offline script. **This removes 6 of 12 P1 runs
  and 84 min of device time**, and it is also the only way to know the primary metric's CV before
  committing to n.
- **S2 is redundant with `gp-sun-drift`.** 12 sequential 2.x runs at GP over a drifting sun, each with
  `meta.json` + `screen.png` + trace (`ls gp-sun-drift/2x-drift/s1`). S2 plans 6 new runs (54 min) for
  n=3/build of the same curve. Offline luma on 12 existing runs dominates it at 0 min.
- **S1 is redundant with `gp`.** `android-a54/gp` = 10 runs (1x 5, 2x 5) at profile 0 (the shipped
  A54 config per `docs/bench/2x-vs-1x-3206-protocol.md:85`) with fps. S1 buys 78 min to re-measure a
  descriptive number that exists at n=5/5, and the only fields it adds (luma, pid-filtered profile)
  are exactly the ones the tree lacks — so **keep S1 at n=2/build (26 min) or drop it** and state
  "shipped config is descriptive, light unmatched" over the existing 10 runs.

**Smallest campaign that still supports the primary claim (B1):**
- P1 at n=4/build, ABBA (A1 3+B1 1 / B2 1+A2 3 — re-derive and print the order), 30 s windows,
  parcel 100,100 phase-matched if §3 holds: 8 runs × 10.4 = **83 min**
- or P1 at n=6/build = 12 × 10.4 = **125 min**
- 2 installs 8 min, F0/F1/F2/F5 40 min, offline reconstruction of the `gp-hi` 2.x arm 0 min device
- **n=4: ≈ 131 min (2.2 h); n=6: ≈ 173 min (2.9 h)**. Add 25 % re-runs → **2.7 h / 3.6 h**.
  Against the plan's 7.3 h (really 9.8 h) that is the honest price of claim B1 alone.

### P1-8. n=6 buys a claim the plan then refuses (the question as asked)
Yes, partially. §3.2 sizes n on the −19 % unmatched effect, but §2.6 (l.148-153) aborts the arm if 2 of
6 runs leave a ±5 luma band, and §7.2 (l.486-492) refuses the delta outright when the bands are
disjoint. Since the campaign's own probe says the light already moved ~4 points between sessions
(98 vs 102/103) — 80 % of the band half-width — the plausible outcome after 169 min is claim 1c.1.
That is a legitimate outcome, but it should be the *pre-registered decision with its cost stated*, not
a footnote: "if F4 fails we stop at 82 min of preflight, not at 251 min".

### P2-9. The "first run after install" effect is not balanced
R8 (l.575) claims "positions are already balanced, so the effect cancels across the ABBA halves". With
installs before positions 1, 4 and 10, the first-run-after-install slots are 1.x (pos 1), 2.x (pos 4)
and 1.x (pos 10): **2 of 3 fall on 1.x**. Fix: add one discarded warm-up run after each install
(+3 × 14 min) or accept and state the asymmetry.

---

## 3. Condition integrity

### P0-10. The §4.2 gate is unreachable by the plan's own evidence
§6.3 (l.393-394) attacks the old gate: "40 °C is not reachable on this device today (idle AP 49.1 °C)".
Then §4.2 cond. 2 (l.267) sets `mName=AP ≤ 40.0 °C`, and §8.7 (l.548) makes `start_ap_c ≤ 40.0` a
validity rule. Every historical run the plan calls usable started at **43.4-45.9 °C**
(`gp-hi-fixed-sky/*/state.json` `thermal_start_c` = 43.8, 44.2, 44.2, 44.0, 43.8, 43.4; `gp-hi-v3` =
45.0, 45.3, 45.9). So the gate rejects 100 % of the prior good data and, on today's device, times out at
1500 s (§4.2) and aborts the block — twice (50 min) before R1's fallback even fires. Fix: make AP
relative like SKIN — `--ap-max <F0 baseline + 1.0>` — and delete the absolute 40.0 from l.267 and l.548.

### P1-11. The ±5 luma band is 1.25× the observed noise, not 5×
§2.6 l.140-142: "±5 is ≈5× the observed relaunch noise". The numbers in the same bullet are: relaunch
within a build ~1 point (102.3 vs 103.1) **and** "~4 points against the in-session value (98 → 102/103)".
The relevant noise for a cross-session, cross-install A/B is the 4-point one → ±5 = **1.25×**. The
claim is false as written; the band cannot distinguish "same light" from the session-to-session shift
the plan itself measured. Fix: widen to ±8 with a stated false-accept probability, or (better) stop
using luma as the match variable — see P1-12.

### P1-12. Mean luma cannot be the matching variable: it is not monotone in fps, in this scene, per the plan's own citation
§1b B1 (l.43) says fps "spans 26.87 at luma 101.6 → 15.83 at luma 117.6 → 14.06 at luma 86.0 … i.e. a
1.9× swing from light alone". Verified in the tree (`gp-hi-v3/1x-v3-r{1,2,3}`: `fps_avg`
26.87/15.83/14.06 against `luma_mean` 101.6/117.6/86.0). The three points are **non-monotone**: the
dimmest run (86.0) is as slow as the brightest, and the fastest run is in the middle. That is not "a
swing from light alone"; it is an unattributed 1.9× within-build swing. Consequently (a) the causal
claim in B1 is unsupported, and (b) "fps per luma point" — the whole purpose of S2 (§2.3) and of the
§7.2 note — cannot be fitted. `03-tooling.md` §4.1.1/§4.1.2 predicts exactly this (tonemapping
compression, skybox dominance: "Luma then tracks camera pitch, not scene complexity").

### P1-13. Luma on a screenshot is confounded across builds, and `03-tooling.md` §4.3 forbids exactly this use
`03-tooling.md` §4.1 item 3 names it: "HUD overlays and UI chrome … a systematic, version-correlated
bias. **This is the most dangerous failure mode for this campaign**", and §4.3.1 says "**Stop using mean
luma as a covariate across versions** — with different HUD/UI chrome it is confounded with the
treatment. Keep it only as a same-build, same-version manipulation check". The plan's §2.6 cross-build
rule (`|luma_crop(2x) − luma_crop(1x)| ≤ 5`) is precisely the forbidden use, and §7.2 step 3 gates the
primary delta on it. The crop `x∈[0.15W,0.85W], y∈[0.10H,0.80H]` is asserted to exclude "the bottom
20 % where both shells draw their controls" with **no layout evidence in the plan or in the tools
doc**; `run_paired.sh`'s own tap coordinates show the shells differ in kind (1.x engine HUD chevron at
2283,240; 2.x RN dialog nodes), and the two shells do not share a HUD geometry.
**Replacement (in order of preference):**
1. Primary light witness = the scene's own value, not pixels. The GP scene's `SkyboxTime` component is
   the authority (`lib/crates/dcl_engine/src/sky/skybox_time.rs:75-92`: `sdk_active` branch eases to
   `sdk_skybox_fixed_time`), so record `SkyboxTime SDK control changed: active=true` **on both arms**
   plus the component's own fixed time read from the scene JSON for the pinned `scene_id`. That is a
   deterministic, cross-build-comparable witness; luma then becomes a *same-build* manipulation check.
   The plan admits the log line exists only on 2.x (§2.4), so today the 1.x arm has **no** light
   witness at all — that asymmetry must be closed, not papered over with a pixel mean.
2. Secondary = `03-tooling.md` §4.3.3's actual recommendation: pixel-fraction bands (`% > 200`, `% < 16`)
   on a viewport crop *measured once per build* from a "Hide UI" screenshot (the RN gallery registers
   one: `rn/src/hud/gallery.tsx:208`, `hud__hideui`), not a scalar mean.
3. If a scalar survives, `03-tooling.md` §4.1.5 also asks for Rec.709 weights; §2.6 specifies Rec.601
   (`Image.convert("L")` is 601). Say which and why; do not let a reviewer find two conventions.

### P1-14. §2.6, §4.3 and §7.2 define the primary analysis three different ways
- §7.2 l.487-488: out-of-band runs "excluded from the **primary comparison**"; pairs must satisfy the
  cross-build rule, "unmatched pairs go to the stratified table".
- §4.3 l.299-301: "Pairs are a **secondary** analysis (n ≤ 6); **the primary analysis is the unpaired
  arm comparison**".
Those cannot both hold: if the primary is unpaired, the cross-build ≤5 rule is never applied to it.
Concretely, with F4 passing at `|L*(2x) − L*(1x)| = 3`, one arm can sit at `L*+4.9` and the other at
`L*−4.9` (both in band) → the published arm-level delta is drawn at **Δ ≈ 12.8 luma points**, while
1c.1 promises no delta above 5. Fix: pick one — either the primary test is the matched-pair median
difference (then say so, and put the pairing in §7.1 where the test is declared), or the unpaired
comparison additionally requires `|median(luma_crop)2x − median(luma_crop)1x| ≤ 5` on the arm medians.

### P2-15. L* has two incompatible definitions and `campaign.json` is written before its bands exist
§2.6 l.135 defines `L*` as "the median `luma_crop` of the **two** pilot runs of that build"; §4.1 F4
l.250 runs "**1 run each**". Also `manifest.py init` (§6.5 l.436) "records … the luma bands" at step 0
of §11, before F3/F4 have produced them. Add a `manifest.py design --set-bands` step and fix the count.

### P2-16. P1's sky policy is inert **only if** the scene override arrives — and that is the arm's single point of failure
§2.1 is honest that "In Genesis Plaza this switch is inert: the scene's SkyboxTime has higher
precedence … so the light is the scene's own time on both builds". Verified in the source
(`skybox_time.rs:75-106`). But the consequence is that **the primary condition's light is an
uncontrolled external input** (a live scene's component), that no code path can pin
(`00-constraints.md:3`), and that its 4-point session drift is exactly the variation §2.6 cannot
absorb. Given the plan already has a design where the light *is* a pure function of UTC and matchable
to ±24 in-game seconds (§2.7, verified: `86400/12 = 7200` s), the phase-matched parcel-100,100 design
should be **primary**, and Genesis Plaza the second, descriptive condition. That inversion also deletes
the need for the luma band as a gate (§3, replacement 1 above), which is where most of the fragility
lives.

---

## 4. Harness specifications

### P0-17. The per-run sequence in §6.6/§11 cannot execute: `verify` runs before the trace exists
§6.1 l.357: `display_mode.sh verify <trace-dir>   # reads <trace-dir>/frames.csv`. §6.6 l.452 and §11
l.663 call it **before** `run_paired.sh` produces that `frames.csv`, and §11 order is
`verify "$RUN"` → `thermal_gate.sh` → `run_paired.sh`. Also the check needs the new `window_ms` column,
which only exists after the run. Fix: run `display_mode.sh verify` **after** the window (it is already
duplicated by §6.2 item 2 inside `cmd_window`, so make `cmd_window` the only writer of `display.json`
and delete the pre-run call).

### P0-18. `run_paired.sh` deletes the run directory after the gate, so `gate.json` never survives
`run_paired.sh:181` is `rm -rf "$OUT"`, executed after `ensure_build`/settings and before
`bench_android.py window`. §6.3 has the gate write `<run-dir>/gate.json` and §6.6 runs the gate
*before* `run_paired.sh` → the file is deleted, `manifest.py` finds no gate, and §8.7 fails **every**
run. Fix: gate writes to `<campaign>/gates/<label>.json` (or `<run-dir>.gate.json`) and `manifest.py`
reads that path; keep the bytes out of `$OUT`.

### P1-19. The gate is invoked twice → +96 min, and it cannot implement condition 4
§6.6 l.452 has the orchestrator call `thermal_gate.sh`, and §6.7 l.460 makes `run_paired.sh` call it too
(with `[ -x ]` guards). Two 480 s floors per run = `+8 min × 12 = +96 min` on P1 alone. Add
`GATE_SKIP=1`/`GATE_JSON=<path>` and one owner.
Second defect: §4.2 cond. 4 is "≥ 480 s since the previous window's end", but the interface (§6.3 l.385)
has no way to receive that timestamp, and it writes `wait_s` (time spent *waiting*). §8.7 then checks
`gate_wait_s ≥ 480` (l.548) — which fails every run whose gate is satisfied on the first poll, i.e. the
good case. Fix: add `--since <unix>` and record `since_prev_end_s` separately from `wait_s`; check the
former.

### P1-20. `cadence.py` cannot run on its own F2 reference file, and its published formula is wrong
- `P = modal(round3(window_ms))` (§6.4 l.406) requires `window_ms`, which **no existing `frames.csv`
  has** (`grep` header of `gp-hi-fixed-sky/2x-sky-r1/frames.csv` = 6 columns, no `dur`). So F2
  ("`cadence.py --check` on that run", §4.1 l.247, §11 l.645) fails on its own fixture, and claim A4's
  "measured p50 land on 4, 6, 7, 8, 9 periods" cannot be re-derived from durable data. Fix: derive `P`
  from `meta.json` + the modal cluster of `interval_ms` (e.g. `round(p50 / round(p50/8.333))`) and only
  *prefer* `window_ms` when present; then F2 can pass on historical runs.
- `stall_time_frac = sum(stall) * P / sum(periods) * P` (l.412) is dimensionally wrong: the correct
  share is `sum(stall) / sum(periods)` (the `P` cancels). As written it multiplies by `P² = 69.4`, so a
  0.041 share is reported as ≈2.85.
- The example `cadence.json` (l.417-419) is internally inconsistent three ways: `frames 872` vs
  `stall_periods_hist` summing to 654; 872 frames over `span_s 29.9` = 29.2 fps while `modal_periods 7`
  implies 17.1 fps; and `pct_window_in_stalls 0.041` implies ≈16 stalled frames while `stalls_per_min
  2.4` implies ≈1.2. A reviewer cannot tell which convention is intended — rewrite the example from a
  real run.

### P1-21. `§5` requires `jank_type_counts` to "agree with our re-derivation" — it cannot, by construction
`1x-sky-r1/meta.json:23-24`: `pct_missed_deadline 92.61`, `jank_any 528` (= 100 % of 528 frames), while
the same run sits on a clean 7-period cadence (`p50 58.29`). The platform judges against the app's
16.67 ms deadline; the plan judges against the modal cadence. So "disagreement is a tooling bug" (l.324)
would flag every valid run. Fix: report `jank_type_counts` as a descriptive cross-check and state the
two definitions differ; only a *change in the mapping between cadence classes and `jank_type` across
runs* is a bug.

### P1-22. `display_mode.sh verify` accepts a bimodal trace
"anything else → `mode_unknown`" (l.362) tests only the *modal* `window_ms`. A run that spends 60 % of
its presents at 8.333 ms and 40 % at 16.667 ms passes the mode test while `modal_periods`,
`on_modal_frac` and every stall number are meaningless. Add `pct_presents_at_modal_window ≥ 0.98`
(report the top two modes and their shares).

### P1-23. `manifest.py` / `campaign.sh` gaps
- `manifest.py` cannot prove `apk_sha256` from a run: a run dir records `version_name`/`version_code`
  only (`bench_android.py:283-292`). Record the hash at install time (`campaign.sh` after
  `ensure_build`) into `campaign.json` and have `manifest.py check` compare against *that*, and say so —
  otherwise §8.2 is unenforceable.
- `manifest.py run` is listed with three subcommands across two sections (`init`, `run`, `check`) but
  `check` re-applies §8 which needs the *window length* and the *campaign design*, neither of which is
  in the run dir. Pass `--campaign` explicitly.
- `campaign.sh` has no adb health check, no per-run timeout for the `run_paired.sh` call, no
  "re-run once" policy (only "stop after two invalid for the same reason"), and `resume` is named but
  unspecified (what is the state source — `campaign.csv`? `campaign.json`? both are written by different
  steps). Specify: state = `campaign.csv` rows keyed by `position`, and `resume` reads the highest
  contiguous `verdict == valid` position.
- §11 l.643 calls `thermal_gate.sh --probe-only` without the `--ap/--status/--skin-max/--floor-s/
  --timeout-s` arguments that §6.3's usage line presents as required; as written argparse exits non-zero.
- §11 l.641 calls `campaign.sh preflight --soak 20`, but §6.6's subcommand list is `preflight F0..F6` and
  has no `--soak`; §4.1 defines no soak step. Either define it (it *is* needed to establish the F0 AP/SKIN
  baseline the gate is relative to) or drop the 20 min.

### P1-24. One edit that breaks existing scripts (beyond the intended behaviour change)
`run_paired.sh:237` writes `frames.csv` with `fieldnames=list(per_frame[0].keys())`, i.e. whatever the
inline SQL returns; adding `window_ms` there is safe for consumers, but the fallback header is
`["dur_ms"]` when `per_frame` is empty. `cadence.py` must therefore treat a `dur_ms` header as "no data"
(see `soak/2x-soak-60m/frames.csv`) rather than crash — the plan notes the outlier (l.370) but does not
specify the guard. Also, `summarize.py`'s new "`verdict != valid` rows excluded" (§6.7 l.463) changes the
meaning of every existing summary table it produces for historical scenarios that have no
`run-manifest.json`; the fallback ("read `cadence.json` when present") does not cover "no verdict", so
old rows must default to `verdict = unknown` and be *kept*, not dropped.

---

## 5. Failure modes the plan does not cover (with where they bite)

1. **Battery cannot power a 7.3 h unplugged campaign.** §4.1 F0 requires "battery ≥ 80 % and
   discharging", §4.2 cond. 5 requires level ≥ 20 % and not charging. At HIGH profile, 120 Hz, ~100 %
   brightness, the A54 draws roughly 3-5 W → 15-25 %/h, so 7.3-8 h needs 110-200 % of one charge. The
   plan has no charging policy, and charging between runs reheats the device — colliding with the very
   gate it just defined. Bites at **day 1, block B1 (position 4)**, where the first 2.x install happens
   on a device already at ~55 %.
2. **`adb` disconnect / partial trace pull.** ~40 MB per run over USB, 24 runs. `frames ≥ 100` catches an
   empty pull but not a truncated one (span shortening is caught only if `span_s < 0.9 × window`).
   `campaign.sh` has no device-health gate and no resume for a half-written run dir. Bites at
   **P1 position 5 (first 2.x run of a fresh install, when the device is hot and the cable is being
   handled)**.
3. **Genesis Plaza is redeployed mid-campaign.** `scene_id` is pinned (§2.1) but nothing re-checks it
   between blocks other than the post-hoc `manifest.py check`. A deploy between day 1 and day 2 voids
   everything after position 6 with no data to show for 170 min. Bites at **position 7 (day 2 opens)**.
4. **An avatar enters Genesis Plaza during one block.** No hide mechanism exists in either build
   (`context.md` §6). The ±2 rule is evaluated on a screenshot taken *after* the window, so a visitor
   present only during the window is invisible to it. Bites at **P1 position 6 or 9** (mid-block, and
   it silently rides on the arm that happens to be running).
5. **`telephony`/`SystemUI`/notification surfaces present into the display-level timeline.**
   `frames_query` does not filter by layer: it joins `actual_frame_timeline_slice` to
   `process p where p.name='/system/bin/surfaceflinger'` (`bench_android.py:130-132,171-181`), i.e. it
   counts *display presents*. A heads-up notification, a low-battery dialog (see #1) or the RN dialog
   surface in 2.x injects frames that 1.x does not have. The plan never states the unit of `frames.csv`
   (§5 calls it "one row per presented frame") nor bounds it. Bites at **any 2.x position after the
   battery crosses 20 %**.
6. **The 2.x build loses profile 3 mid-run** — it did exactly that before
   (`gp-hi-pair/2x-hi-p1`: 4 `[RenderReady]` lines, last with `reason:"settings"` → profile 2). The
   plan detects it post hoc (rule 3) but has no in-run guard and no per-run "abort early" — a block of
   3 already costs 42 min when the first two are invalid. Bites at **B1, position 4**.
7. **Day-2 ambient / A/C drift.** R9 exists but only checks `|Δ median start_ap_c| ≤ 1.0 °C` at the end;
   with the gate relative and unreachable-at-absolute-40 (P0-10), the gate cannot hold ambient constant
   to 1 °C across two days. Bites on **all of day 2**.
8. **`screencap`/`uiautomator` hang.** `run_paired.sh`'s entry-flow taps can hang on the 2.x Discover
   carousel (the script's own comment says so). The plan's per-run timeout policy ("stops when invalid
   twice for the same reason") means two 14-min runs are lost before it acts. Bites at **the first 2.x
   run after an install** (position 4).

---

## 6. Verdict, section by section

| § | verdict | what |
|---|---|---|
| 1 Objective & claims | **fix** | Keep 1a A1/A2/A4/A6 and the whole of 1c (it is the best part). Fix B1's justification (l.43, non-monotone data). Fix A3/A5: they are claimed but unbudgeted, unmetricised and untabled. |
| 2 The condition | **fix + invert** | Make the phase-matched parcel-100,100 condition primary (light is a function of UTC, matchable to ±24 in-game s); GP becomes descriptive. Delete the absolute AP ≤40 gate (P0-10). Replace cross-build luma gating with the scene `SkyboxTime` witness + same-build manipulation check (§4.3.1). Fix `L*` definition and band width (P1-11, P2-15). |
| 3 Arms & n | **fix** | n=6 → n=4 unless the primary metric's CV is measured first (P0-1, P1-4). Rewrite the "smallest n for p<0.05" sentence (P1-3). Rebuild the budget: 435 → 483 + 25 % contingency (P1-5) and define each cut's re-derived run order (P1-6). |
| 4 Run order & preflight | **fix** | Keep ABBA and the 2-install argument. Add one warm-up run after each install (P2-9); make the gate relative and single-owner (P1-19); add F4b's 42 min to the preflight line or drop the fallback. |
| 5 Metrics per run | **fix** | Swap the primary metric (P0-1). Delete the "jank_type must agree" requirement (P1-21). State the unit of `frames.csv` (display present, not app frame). |
| 6 Harness changes | **fix** | Order bug: `verify` after the window (P0-17); gate output outside `$OUT` (P0-18); single gate owner (P1-19); `cadence.py` P-derivation fallback + formula fix + honest example (P1-20); add modal-share and adb-health checks (P1-22/23); add `manifest.py design` (P2-15). |
| 7 Analysis plan | **fix** | One primary test, declared once (P1-14). Keep the bootstrap-MDR/refuse-delta machinery; drop the "fps per luma point" note until a monotone relation exists. |
| 8 Invalidation rules | **fix** | Rule 10 is wrong and voids everything (P0-2); rule 7 must drop the absolute AP (P0-10); rule 6 must add the modal-share check (P1-22); §8.1's file list omits `launch.log`/`launch.json` which `manifest.py` reads. |
| 9 Risks | **keep** | R1-R14 are the right list; add the eight in §5 above and correct R8's balance claim (P2-9). |
| 10 iPhone 16e | **keep** | The labelling rules are correct and the drop-first ordering is right. State that it is outside the 7.3 h. |
| 11 Checklist | **fix** | Fix the per-run ordering (P0-17), the `--probe-only` argument set, `--soak` (P1-23); reorder preflight so it needs 2 installs (P1-5), and move `manifest.py init` after F3/F4. |

**Bottom line: reject as-is; approve with the changes listed.** The plan's *shape* is right — condition
ledger, run-as-unit, refuse-the-delta discipline, and an honest refused-claims list — but three defects
make it unpublishable as written (P0-1 primary metric unmeasurable at n=6, P0-2 rule 10 voids every run,
P0-10 + P0-17 + P0-18 mean the per-run pipeline cannot even produce a valid run dir), and one makes it
wasteful (P0-7: 138 of the 301 measured-arm minutes re-measure data already on disk). With §1-§4 of this
review applied, the campaign is ~3-4 h of device time, not 7.3-9.8 h, and it can actually produce B1.
```

## 15-line summary of the most damaging findings

1. **P0 primary metric is unmeasurable at n=6**: `pct_window_in_stalls` has ~5 events/run (p99 83.13 vs 10-period 83.33 in `2x-sky-r1`) → CV ≈45 % → MDR(6) ≈73 %. Nothing smaller is detectable.
2. **P0 §8.10 `pid_before = 0` voids every run**: `1x-sky-r1` 27411, `2x-sky-r1` 20251 — `run_paired.sh:233-279` always launches first. 0 of 24 runs can be valid.
3. **P0 gate contradiction**: §6.3 says 40 °C is unreachable (idle AP 49.1); §4.2/§8.7 then gate on AP ≤ 40.0. All 15 prior good runs started 43.4–45.9 °C.
4. **P0 ordering**: §6.6/§11 run `display_mode.sh verify` before `frames.csv` exists, and `run_paired.sh:181 rm -rf "$OUT"` deletes the `gate.json` the gate just wrote → `manifest.py` fails every run.
5. **P1 n=6 rationale self-contradicts**: "smallest n with p<0.05" — table row says n=4 → 0.029; tooling §6.1 says the same. Smallest for p<0.01 is 5.
6. **P1 effect size = the confound**: the −19 % is from arms 27–36 luma points apart (104.6–108.8 vs 72.6–77.6), which §2.6 forbids publishing as a delta.
7. **P1 budget**: 435 sums, components don't — S1 13 min < its own 480 s floor + overhead (14.35), S2 9 < 10.4, preflight installs 2 not 4, A3/A5 unbudgeted. Corrected ≈483 + 25 % re-runs ≈ 590 min.
8. **P1 138 device-minutes re-measure data on disk**: `gp-hi/2x-gp-hi-r1..r5` have traces + `screen.png` (luma and `window_ms` are offline recomputes); `gp-sun-drift` has 12 drift runs; `gp` has 10 shipped-config runs.
9. **P1 luma is the wrong variable**: `gp-hi-v3` is non-monotone (26.87 fps @101.6, 15.83 @117.6, 14.06 @86.0), so "fps per luma point" (S2, §7.2) is unfittable and B1's "1.9× from light alone" is unsupported.
10. **P1 band width wrong**: "±5 ≈ 5× relaunch noise" — it is 1.25× the 4-point session shift in the same bullet. And `03-tooling.md` §4.3.1 explicitly forbids cross-version mean luma; replacement = scene `SkyboxTime` fixed-time witness (verified branch order `skybox_time.rs:75-106`) + pixel-fraction bands.
11. **P1 three definitions of "primary analysis"**: §7.2 (pair-gated) vs §4.3 (unpaired, pairs secondary) allows an arm-level delta at Δ ≈12.8 luma while 1c.1 promises ≤5.
12. **P1 `cadence.py` spec broken**: `P = modal(window_ms)` requires a column no existing `frames.csv` has (F2 unbacktestable); `stall_time_frac = sum(stall)*P/sum(periods)*P` should be `sum(stall)/sum(periods)`; example json contradicts itself 3 ways (872 frames vs hist 654 vs 29.2 fps vs 7 periods).
13. **P1 gate double-invocation** (+96 min) and `gate_wait_s ≥ 480` checks wait-time, not time-since-previous-window — fails the fast path, passes a slow one.
14. **P1 §5 "jank_type must agree" is impossible**: `1x-sky-r1` has 92.61 % missed deadline / 100 % jank on a clean 7-period cadence (different definitions, not a bug).
15. **P1 eight uncovered failure modes** — chief among them the battery (15–25 %/h × 7.3 h > one charge, with charging forbidden at the gate), adb partial pulls, a GP redeploy at day-2 open, avatars invisible to the post-window screenshot, and SurfaceFlinger-level frames that count system surfaces 1.x does not have.