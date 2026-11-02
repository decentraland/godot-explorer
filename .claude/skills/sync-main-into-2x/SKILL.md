---
name: sync-main-into-2x
description: Use when updating the `2.x` branch with what landed on `main` (the recurring "sync main into 2.x" job, D-31) — finding the upstream PRs in range (release syncs included), giving each a verdict (Taken / Ported / Adapted / Already covered / Discarded / Needs follow-up), merging with the 2.x resolution rules (deleted `.gd` stay deleted, nothing added under `godot/src`, `lib/src` → crates, engine branch pin, version), landing one port commit per upstream PR, keeping `plan/` true and opening the draft PR into `2.x` that must be merged as a merge commit. Trigger on "update 2.x from main", "sync main into 2.x", "merge main into 2.x", "bring main's changes to 2.x", or a merge of `origin/main` with conflicts on `godot/src/**/*.gd`. For the porting itself, load `port-main-pr-to-2x`.
---

# Sync `main` into `2.x`

`main` ships the 1.x line (GDScript UI, Godot plugins) until 2.x merges into it in December.
`2.x` is the rewrite: every screen in React Native, the business rules in a Rust core
(`dcl_core`), and an engine (`dcl_engine`) that only runs scenes — with **no GDScript in the
product**. Until the merge, `main` keeps getting fixes and features, and 2.x must pick up every one
that still matters, in 2.x's architecture (D-31; 2.0.0 ships nothing less than 1.x users have,
D-21).

**A sync is not "make the merge compile".** It is reading each upstream PR, deciding what it means
for 2.x, and doing that: taking it, re-implementing it in Rust or React Native, or discarding it
with a reason. The how of re-implementing is the `port-main-pr-to-2x` skill (invariants, where each
1.x system lives on 2.x, main → 2.x Rust idioms); load it before resolving Rust conflicts or
porting. The plan is `plan/` (`README.md`, `02-target.md`, `05-system-placement.md`,
`decisions.md`); if this skill and the plan disagree, the plan wins and this skill gets fixed.

```
in-flight check → branch → inventory → read every PR → verdicts → merge (resolution only)
  → one port commit per upstream PR → checks → plan updates → draft PR into 2.x (merge commit)
```

## 1. Before starting

- Is a sync already open? `gh pr list --base 2.x --search "sync main into 2.x" --state open`. If
  one is open and unmerged, extend it (merge `origin/main` again on its branch, add rows) instead of
  opening a second one that resolves the same conflicts differently.
- Was the previous sync merged as a merge commit? `git merge-base origin/2.x origin/main` must be
  the last synced main commit. If it is older, the last sync was squashed (§6): re-merge from that
  sync's branch instead of redoing every resolution.

## 2. Range and inventory

```bash
git fetch origin main 2.x
git switch -c chore/2.x-sync-main-$(date +%F) origin/2.x
python3 .claude/skills/sync-main-into-2x/scripts/inventory.py --gh > /tmp/sync-inventory.md
```

The script prints one row per upstream change since the merge-base. It expands release syncs
(`Merge pull request #N from decentraland/release-x.y.z`, `chore: sync release into main`) into the
PRs they bring — hotfixes cut on `release-*` reach `main` only that way — and marks every touched
file `same path`, `-> moved path`, `?candidate` (moved into `dcl_core` or split) or `NOT on 2.x`.
Its "first guess" is a starting point, never the verdict. Fold a follow-up fix with no PR number
into its PR's row; skip empty RC re-cut commits.

## 3. Read every PR, give each a verdict

For each row: `gh pr view <N>` (the body is the spec: behaviour, flags, analytics, known gaps),
`git show <sha>`, main's final files with `git show origin/main:<path>`, then the 2.x home of each
touched system (`port-main-pr-to-2x` §2).

| Verdict | When | What the sync does |
|---|---|---|
| **Taken** | CI, tooling, docs; Rust/shader/asset changes whose files exist on 2.x; version bumps (**not** the version number: 2.x stays at 2.0.0) | the merge carries it; check it is still *wired* (a Rust function whose only caller was GDScript is dead on 2.x until a port calls it) |
| **Ported** | behaviour written in GDScript that 2.x needs (engine side, logic, flags, notifications, settings, product UI features) | re-implemented in Rust (engine or core) or RN, one commit per upstream PR, same behaviour |
| **Adapted** | the feature matters but its 1.x shape breaks a 2.x rule (a live avatar inside a modal, a plugin, GDScript modal ordering) | re-implemented the 2.x way; the PR row says what differs and why |
| **Already covered** | 2.x already has it (an RN screen built from Figma, a core rule) | nothing, or the small gaps closed; the row names the 2.x file |
| **Discarded** | fixes to GDScript UI that no longer exists (layout, sizing, focus, a `.tscn` glitch); 1.x-only tooling (`check_gdscript.rs`, gdlint); plugin build fixes; social/friends client changes (D-43); GDScript tests whose subject is gone; release-branch chores | nothing; one line saying why |
| **Needs follow-up** | it matters but cannot be done in this sync: an engine build, a design, a device, a product decision | flagged in the PR with a recommendation, and written into the plan (§8) |

Defaults that hold most of the time:

- **A UI bug fix under `godot/src/ui/**` → Discarded.** The screen is RN now. Check the RN screen
  only when the bug was about *data or behaviour* (wrong count, wrong state), not layout.
- **A UI feature (new screen, overlay, tutorial, flow change) → Ported or Adapted in RN**, with its
  flags, analytics and persistence in the core.
- **An engine-side GDScript change (avatar, player, camera, components, fetcher) → Ported** onto
  its phase-5 Rust twin.
- **A Rust change → Taken**, translated to 2.x idioms where it conflicts.
- **Locale CSVs → Taken**: they are the source of RN's es/pt_BR (D-26) even when the GDScript using
  a key is gone.
- **A version bump → Discarded.** 2.x is 2.0.0 and a sync never changes it: keep `version = "2.0.0"`
  in `[workspace.package]` (`lib/Cargo.toml`) and the 2.0.0 lines in `godot/export_presets.cfg`, whatever
  main's bump says (the build string, TestFlight and Firebase all read it).

## 4. Merge: resolution only

```bash
git -c merge.renameLimit=100000 -c diff.renameLimit=100000 merge --no-ff --no-commit origin/main
```

Without the rename limit git does not see `lib/src/*` → `lib/crates/dcl_engine/src/*`, and every
Rust change turns into a modify/delete conflict.

The mechanical part:

```bash
# 2.x deleted it, main modified it: it stays deleted
git status --porcelain | awk '$1=="DU"{print $2}' | xargs git rm -q --
# main added anything under godot/src (.gd, .uid, GDScript-owned .tscn and art): drop it
git status --porcelain | awk '$1=="A" && $2 ~ /^godot\/src\//{print $2}' | xargs git rm -q --cached --
git status --porcelain | awk '$1=="??" && $2 ~ /^godot\/src\//{print $2}' | xargs rm -rf
```

Art a port will need (tutorial images, icons) is fetched later from `origin/main` into its 2.x
home (`rn/src/assets/…`, `godot/assets/…`), not kept under `godot/src`.

By hand, understanding both sides (`git show "${MB}:<old path>"` against each side):

- **A `.tscn` 2.x made script-less** (e.g. `avatar.tscn`) → take ours (`git checkout --ours`); the
  port commit brings main's structural change (nodes, resources) without scripts.
- **An asset main moved that 2.x's scenes still reference** → keep it where 2.x expects it; the
  port moves it together with the scene that stops needing it.
- **Rust content conflicts** → keep both intents, written the 2.x way (`port-main-pr-to-2x` §4).
  Never take theirs wholesale on a file 2.x reworked.
- **Files 2.x deleted on purpose** (`lib/src/notifications.rs` → `dcl_core`, the friends client,
  `src/check_gdscript.rs`) → stay deleted; the change is ported to where the logic lives now.
- **`src/consts.rs` `GODOT_USE_BRANCH`** → keep 2.x's pin (the iOS RN-host engine,
  `spike/ios-host-embedding`, engine PR #29, D-23): the typed bindings come from that branch's
  `extension_api.json`. If main pins another engine branch, merge that branch into the 2.x pin in
  the engine repo (and retarget the pinned PR onto it), reinstall, and call the new methods typed.
  Only when that cannot happen in the sync, use a guarded probe (`port-main-pr-to-2x` §4) and flag it.
- **`lib/Cargo.toml`** → 2.x's `[workspace]` layout, `version = "2.0.0"` in `[workspace.package]` (never main's); let
  cargo rewrite `Cargo.lock`. **`godot/export_presets.cfg`** → 2.x's rest, version lines stay 2.0.0.
- **zsh**: `$MB:lib/...` applies the `:l` (lowercase) modifier; write `"${MB}:lib/..."`.

The merge commit builds and passes §6's checks on its own. Its message lists the upstream PRs it
brings and the resolution choices; the behaviour comes in the next commits.

## 5. Port commits: one per upstream PR

Every Ported/Adapted PR gets its own commit (or a short series) on the sync branch, made as
`port-main-pr-to-2x` describes, titled with a conventional type and the upstream number, e.g.
`perf: hiccupless loading on the Rust engine (main #3015)`. A reviewer can then read and revert the
sync PR by PR.

Large syncs fan out well: one session or agent per domain (engine perf, avatar/player, RN flows,
core/notifications), each in its own worktree branched from the merge commit, each loading
`port-main-pr-to-2x`; then cherry-pick their commits onto the sync branch in upstream order and
re-run the checks. Ports that touch the same area (two avatar PRs) are told about each other so
they stay in separate modules, and the later one rebases onto the sync branch before finishing.
A new worktree may start from another checkout's HEAD (often `main`): each port first checks
`git log -1` and resets onto the merge commit.

## 6. Checks (green on the branch head before the PR)

```bash
find godot/src -name '*.gd'                                   # empty
cd lib && cargo fmt --all --check \
  && cargo clippy --workspace --all-targets -- -D warnings \
  && cargo test -p dcl_core && cargo test -p dclgodot --lib     # includes call_targets
cd ../rn && npx tsc --noEmit && npm run lint && npm test        # when rn/ or the FFI changed
cd rn && node scripts/i18n-engine.mjs --check                   # when a port touched strings
```

Device QA is not part of a sync unless a port changes something QA can see; then the test plan
covers it (REVIEW.md §4 format, the `pr-description` skill).

## 7. Draft PR into `2.x`

```bash
git push -u origin HEAD
gh pr create --draft --base 2.x --title "chore: sync main into 2.x (up to <short sha>, <date>)" --body-file <file>
```

**It is merged with "Create a merge commit", never squashed.** 2.x feature PRs are squashed; a
squashed sync loses `main` as a parent, the merge-base does not move, and the next sync re-conflicts
on everything this one resolved. Say so at the top of the body.

Body (the lead paragraph is enough to understand it; the table carries the detail):

```markdown
> Merge with **Create a merge commit** (not squash): the merge-base with `main` must move.

## What
Brings `main` up to <sha> (<date>) into 2.x: N upstream PRs. X taken as is, Y re-implemented in
Rust/RN, Z discarded (GDScript UI 2.x no longer has), W need a person.

## Why
2.x must ship everything 1.x users have (D-21), and `main` keeps moving until the December merge (D-31).

## Upstream PRs
| PR | On main | On 2.x | Where / why |
|---|---|---|---|
| #NNNN title | one line | Taken / Ported / Adapted / Already covered / Discarded / Needs follow-up | 2.x file(s), or the reason |

## Needs a person
- <item>: why, recommendation.

## Test plan
<REVIEW.md §4 cases only for what a port changed visibly; "No QA needed — no behaviour change on 2.x" otherwise>
```

## 8. Keep `plan/` true

- A 1.x feature 2.x now has, or still lacks → a row in `plan/phase-4-parity.md`. Parity is against
  what 1.x users have today, not a frozen 1.15.
- A follow-up that needs a person → an item where it belongs (usually `phase-6-testing-polish.md`
  6.1) naming the upstream PR.
- A sync that changes a decision (the engine pin, a dropped feature) → `decisions.md`.
- No history in the plan ("synced on…"): the PR holds that.

## 9. Done means

- Every upstream PR in range has a row with a verdict; no row says "Taken" for a `.gd`.
- The merge commit and every port commit build; §6 is green on the head.
- Every follow-up is in the plan with its upstream PR number; nothing is dropped silently.
- The PR is a draft into `2.x` that says "merge commit, not squash", and its test plan only asks QA
  for what a port changed visibly.
