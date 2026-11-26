---
name: bug-hunter
description: Use when asked to hunt, look or search for bugs in the Decentraland mobile app (the React Native shell + Godot engine on `main`), run an exploratory QA pass, check "what is broken" on the newest build, smoke-test the last build from `main`, re-verify open bugs, or triage fresh crashes. Encodes the hunting loop — update the phone to the newest `main` CI build and prepare a desktop client, pick hunting grounds (Sentry on the newest releases, the phone's crash buffer, freshly merged PRs, a device exploratory pass, a desktop scene sweep, open issues to re-verify, test-infra rot), the verification bar a finding must pass, the known non-bugs that waste time, and the report format. Reports findings to the requester; files nothing without approval. Trigger on "bug hunt", "find bugs", "hunt for issues", "QA pass", "test the latest build", "is anything broken on main", "check the newest build".
---

# Bug hunter — Decentraland mobile app

The job: find real, reproducible defects in what players run, prove each one, and report them to
the person who asked. **Report, don't file.** Issues are filed only after the requester picks which
ones, with the `godot-issue-writer` skill. Nothing gets fixed during a hunt.

Read first: `rn/README.md` (how the RN shell, the Rust core and the engine split the app),
`CLAUDE.md`, `REVIEW.md` §1 (Android and iOS are the product; desktop-only findings are out of scope).
For driving the engine, the `mobile-dev-debug-tool` skill.

## 0. Ground rules

- **A finding is a claim with evidence.** Symptom, build, steps, how many times it reproduced (N/M),
  evidence (screenshot, log line, symbolized stack, Sentry link), duplicate check, confidence. Without
  a reproduction, say "code-verified" or "suspected" — never present it as reproduced.
- **Strength of proof, best first:** reproduced on a phone → reproduced on the desktop client →
  confirmed by a local experiment (patch, rebuild, symptom gone — then revert) → read in code.
- **Newest build only.** Re-check every lead on the build you report against; name it
  (`2.0.0.<build>-<sha>-staging`, shown at the bottom of Settings and in the in-world FPS label).
- **Do not destroy the tester's state.** CI builds share one signing key, so `adb install -r` keeps
  the session. A local build has another key and needs an uninstall that wipes the login: ask first.
  Never `pm clear`. For a fresh install use a temporary Android user (§3.4).
- **Experiments stay local.** Any patch made to test a hypothesis is reverted; `git status` is clean
  when the hunt ends.
- **Standing permission, one label:** when an *existing* issue reproduces on a device or in the
  running client, add `ia-confirmed` (`gh issue edit N --add-label ia-confirmed`). Never on a code
  read. No comments, assignments or closes without being asked.

## 1. Set up (update to the last build on `main`)

1. `git fetch origin main` and work from `origin/main`.
2. **Phone, newest `main` CI build.** `scripts/latest-main-apk.sh --install`: it walks
   `origin/main` from the top, finds the first commit whose APK is in R2
   (`mobile-artifacts…/android/main/<sha>/`), prints the build number and installs it with `-r`.
   CI publishes ~35 min after the merge (`gh run list --branch main`, job "Build Android"). A PR
   branch build of the same code may already be on the phone: compare the sha in the version string.
3. **Desktop client** (the engine on a desktop, phone window, debug hub attached). From an orca
   worktree:
   ```bash
   CARGO_TARGET_DIR=<scratch>/xtask-target cargo build --bin xtask   # never the shared target's xtask
   X=<scratch>/xtask-target/debug/xtask
   $X install --targets android                      # Godot branch build + Android templates
   T=<main checkout>/lib/target; mkdir -p lib/target  # xtask copies from ./lib/target/<mode>
   for d in debug release aarch64-linux-android; do ln -sfn $T/$d lib/target/$d; done
   $X build && $X import-assets
   $X debug-hub > <scratch>/hub.log 2>&1 &          # never pipe the hub through head/tail
   open -n .bin/godot/Godot.app --args --path "$PWD/godot" --emulate-ios \
     --realm https://realm-provider-ea.decentraland.org/main --location 0,0 \
     --scene-inspector=ws://127.0.0.1:9231
   ```
   Launch through `open -n` (exec'ing `godot4_bin` from a background shell stalls on macOS). Check
   `lsof -nP -iTCP -sTCP:LISTEN | grep 923` first; another session may own the hub ports (then
   `debug-hub --device-port 9331 --consumer-port 9330` and `PORT=9330` for the helpers).
4. **iOS** when the iPhone shows as `available` in `xcrun devicectl list devices` (it drops off when
   asleep). TestFlight builds; same observation limits as Android CI builds.

What each target can observe:

| Target | UI (RN) | Engine state | Logs |
|---|---|---|---|
| Phone, CI build | `scripts/android-ui.sh` (Maestro hierarchy, taps, screenshots) | — | `adb logcat`, `adb logcat -b crash` |
| Phone, local debug build | same | debug hub (auto-dials `127.0.0.1:9231` + `adb reverse`) | + hub log stream |
| Desktop client | none (no RN on desktop: world only, throwaway guest) | debug hub: `state`, `scenes`, `scene`, `avatar`, `screenshot`, `teleport` | `~/Library/Application Support/Godot/app_userdata/Decentraland/logs/godot.log` |

The hub cannot be attached to a **CI** build on 2.x: the `?scene-inspector=` deeplink reaches the
engine but host-driven mode never re-emits `deep_link_received`, the only signal the bridge listens
to. Observe CI builds through adb.

## 2. Hunting grounds — pick at least three per hunt, rotate between hunts

### 2.1 Fresh telemetry (Sentry)
- Org `dcl-regenesis-labs`, project `godot-explorer`. Releases are
  `org.decentraland.godotexplorer@<version>+<build>`. Filter by release, not environment: 2.0.0
  builds from `main` deploy as `staging` but their events arrive tagged `production`. Players run
  the 1.x releases (1.16.x); a bug they hit that 2.0.0 still has is worth more than a 2.0.0-only one.
- New issues on the newest builds: `is:unresolved release:org.decentraland.godotexplorer@2.0.0*`,
  sort `new`. A `newIssues` count on `find_releases` points at the build that introduced it.
- Scene crashes: `search_errors` on `"Scene crashed"` grouped by title — a popular place crashing for
  many players is a top finding even when the scene's code is at fault (§2.5).
- Every fatal event: read the breadcrumbs (`get_issue_breadcrumbs`). The last lifecycle crumbs tell
  you what the player did ("paused → stopped → destroyed" = the app was closing).
- Android vitals: the `play-vitals` skill.

### 2.2 The phone's own crash buffer
`adb logcat -b crash -d` survives the process: every tombstone there is a lead, including the
tester's own crashes from earlier in the day. Symbolize with
`scripts/symbolize-android-crash.sh <sha-of-installed-build>`: it downloads the unstripped
`libdclgodot-android-release` artifact of that commit's CI run and checks the Build ID. Then
reproduce: the tombstone's `Process uptime` and the surrounding `logcat` (ActivityManager,
`CoreBackPreview` = Back pressed) usually give the trigger.

### 2.3 Fresh merges
`git log origin/main --first-parent --since="5 days ago" | grep -v "chore(deps"`. Large or
platform-wide merges are where regressions are. Delegating a read of a big diff to a subagent is
fine: give it the merge commit, the architecture docs, what to look for (silently dropped
behaviour, thread and lifecycle mistakes, Back handling, dead ends) and ask for file:line evidence
plus an on-device repro per claim. **Verify every claim yourself** — reviewers report confident
claims that are latent or config-gated.

### 2.4 Device exploratory pass
Walk the app on the phone with `scripts/android-ui.sh` and observe after every step: screenshot,
`android-ui.sh state` (pid changed = crash; rotation; keyboard), `adb logcat -b crash -d`.

| Area | Do | Watch for |
|---|---|---|
| Discover (portrait) | hero card, rails, search, place sheet, `ACESSAR` | dead buttons, wrong counts, untranslated *new* text |
| Enter world | jump in, wait for landscape | HUD layout before the first touch (the joystick, safe areas), FPS label |
| World HUD | joystick, jump, emote wheel, chat, `Menu` drawer | input dead after a panel, controls at wrong positions |
| Drawer items | Discover, Social, Notifications, Backpack, Settings, Profile — each from the world | back arrow vs hardware Back, avatar preview = the player's real avatar |
| Settings | every section and row | rows that do nothing (`coreApi.ts` empty stubs) |
| Leave | Back → `LEAVE` → Discover → re-enter | second-entry differences, memory |
| Exit | hardware Back on the Discover home | native crash on teardown (crash buffer) |
| Lifecycle | home/resume in world, rotate, notifications prompt | video, audio, input after resume |

Compare what a screen shows with the source of truth: the deployed profile
(`peer.decentraland.org/lambdas/profiles/<address>`; get the address from `adb logcat | grep -o
"0x[0-9a-f]\{40\}"`), the places API, the scene's own data.

### 2.5 Desktop scene sweep
`scripts/scene-sweep.sh <outdir> --top 12` teleports the desktop client through the most active
places and prints, per place: load time, `scenes_loaded`, the scenes and their tick stats, error and
warning counts, crash/"not responding"/script-error lines, and saves a screenshot and the log slice.
A place with `scenes: []` after the load, or "Scene crashed: … not responding", is a lead.

Scene or explorer? A scene error alone is not our bug. Check:
1. Sentry history of `Scene crashed: <title>` across releases (production = players hit it).
2. The scene's code: `curl https://peer.decentraland.org/content/contents/<entity-hash>` →
   `metadata.main` → fetch that file and read the `<loader>:<line>:<col>` position from the error.
3. Does it depend on something our runtime does differently (`lib/crates/dcl_engine/src/dcl/js/js_modules/`,
   the ops behind them)? A local experiment settles it: patch, `xtask build`, relaunch, revisit, revert.
4. Unity parity: does the place work for players on the Unity explorer? Unity is the reference.

### 2.6 Re-verify open bugs
`gh issue list --label bug --state open`. Prefer issues without `ia-confirmed`, older than the last
big merge, or marked `mock-only` (check whether the real build has it too). Reproduced → add
`ia-confirmed` and report "still present on <build>". Not reproduced → report it as a candidate to
close, with what you tried; do not close it.

### 2.7 Logs
On desktop, summarise `godot.log` after a session:
`grep -E "^(ERROR|WARNING)|panicked" godot.log | sed -E 's/[0-9]+/N/g' | sort | uniq -c | sort -rn`.
On the phone, `adb logcat -d | grep -E "panicked|call failed|after it has been freed|Fatal|ANR"`.
New error *signatures* matter, not counts. Check the noise list (§5) before chasing one.

### 2.8 Test-infra rot
- Maestro flows (`rn/.maestro/`) assert literal strings; grep each `visible:`/`tapOn:` string in
  `rn/src` (matching ignores case). A string no longer in the app = the flow and everything after it
  in `ensure-home` cannot pass. No CI job runs Maestro, so nothing else catches this.
- Scene tests: `grep -rn skipTest tests/scene-tests` — each skip names a known bug; check the issue is
  still open and the bug still reproduces.

## 3. Verify

1. **Reproduce twice** on the newest build, from a clean start when the bug might depend on state.
2. **Minimal steps** that start from opening the app, with on-screen names (`ACESSAR`, `Menu` →
   `Profile`), as QA would run them (`REVIEW.md` §4).
3. **Root-cause pointer when cheap** — a symbolized frame, a file:line, an experiment — never a fix plan.
4. **Fresh install without wiping the tester** (Android): a temporary user has its own app data and
   device id, so "play as guest" creates a new account.
   ```bash
   U=$(adb shell pm create-user bughunt | grep -o "id [0-9]*" | awk '{print $2}')
   adb shell pm install-existing --user $U org.decentraland.godotexplorer
   adb shell settings put --user $U secure user_setup_complete 1
   adb shell am switch-user $U
   adb shell am start --user $U -n org.decentraland.godotexplorer/com.godot.game.GodotAppLauncher
   # ... test ...
   adb shell am switch-user 0 && adb shell pm remove-user $U
   ```
   Switching back leaves the phone **locked behind the owner's PIN**, which adb cannot unlock, and an
   app launched under the lock screen fails to create its window. Do this last and tell the requester
   the phone needs unlocking.
5. **Duplicates**, before reporting: `gh search issues --repo decentraland/godot-explorer "<keywords>"`
   open and closed, `gh issue list --search "<word> in:title"`, Sentry, and `plan/` (phase 6 lists known
   gaps). Same symptom already filed → report "duplicate of #N" or "still present", not a new finding.

## 4. Report

Reply to the requester with findings ranked by severity, most severe first:

```markdown
### N. <symptom as a player sees it> — <severity>, <platform>
- **Build:** 2.0.0.<build>-<sha> · **Reproduced:** N/M (device | desktop | code only)
- **Steps:** 1. … 2. … → <what happens>
- **Expected:** <observable>
- **Evidence:** <screenshot path, log line, symbolized frame, Sentry link + counts>
- **Pointer:** <file:line or experiment result — not a fix>
- **Known?** <new | duplicate of #N | still present in #N (labelled ia-confirmed) | listed in plan/…>
```

Severity: **critical** crash or data loss most players can hit · **high** a feature broken on a
common path, or a crash on exit · **medium** wrong result with a workaround, popular content broken ·
**low** cosmetic, rare, dev tooling. Production evidence (Sentry user counts) moves an item up.

End the report with **Checked, nothing found** (one line each) and **Not verified** (leads you could
not prove, and why). Offer to file the confirmed ones with `godot-issue-writer`; file only the ones
the requester picks. Put screenshots in the scratchpad and give full paths.

## 5. Known non-bugs (do not report)

- **Untranslated es/pt_BR text**: tracked debt, budgeted by `UNTRANSLATED_MAX` in
  `rn/__tests__/i18n-coverage.test.ts`. Report only a hardcoded literal or a broken key/placeholder.
- **Friends rail shows "User p name", "Username"**: mock friends until plan 6.9 (`discover/coreApi.ts`).
- **Profile picture placeholder** in edit profile: ADR-290 profiles carry no picture URL (plan 6.1).
- **Zero-padded user counts** ("007"): `formatUsers` in `discover/format.ts`, by design.
- **`CRDT: unknown LWW component <large number>`**: scene-defined custom components, not SDK ones.
- **Jolt `Failed to correctly scale body`** for non-uniform scale on a sphere/capsule collider: Unity
  cannot do that either; not in Sentry.
- **`Failed to save impostor PNG … No such file or directory`**: two saves of one avatar share a
  `.tmp`; the loser's rename fails. Harmless.
- glTF `ERR_FILE_CORRUPT at png_read_image_no_gamma` (JPEG tried as PNG first), particle
  `Bones array must be a PackedInt32Array`, `get_meta … dcl_shape`, `StreamProtocol … closed`,
  Android `AppOps attributionTag not declared`.
- Desktop-only behaviour (REVIEW §1) and `mock-only` crashes, unless the real build has them too.

## 6. Driving the phone — practical notes

- `scripts/android-ui.sh dump` lists texts and accessibility labels with bounds; `tap "<regex>"`
  taps by label. Screenshots are physical pixels (1220×2712 portrait / 2712×1220 landscape on the
  Moto Edge 60 Pro); images shown to you are downscaled, so take coordinates from `dump`, not from a
  picture.
- `uiautomator dump` hangs on animated screens; Maestro's hierarchy does not (~15 s per call).
- Landscape = in a world. `android-ui.sh wait-landscape` after `ACESSAR`; give the scene ~20 s more.
- Screenshot while a finger is down: `(adb shell input swipe x1 y1 x2 y2 4000 &); sleep 2; adb
  exec-out screencap -p > f.png`.
- `adb install` can restart the adb server and drop `adb reverse`; re-run it.
- If adb loses the phone, Chrome's `chrome://inspect` may hold the USB interface.
- Engine logs carry the `godot` logcat tag (`adb logcat -d | grep " godot "`). CI builds print no
  JS console output; RN problems show up as crashes, Sentry events or on screen.
