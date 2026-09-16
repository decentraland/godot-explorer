---
name: pr-description
description: Use whenever writing, rewriting or reviewing the description of a pull request in this repo — a development PR (base `main`) or a Release Candidate PR (base `release`, head `release-X.Y.Z`). Encodes the team's AG (AI-generated) communication guideline as it applies to PRs — the author owns understanding and communicating the change — plus the exact description shape: `## What` and `## Why` sections anyone can understand in a few sentences, an optional `## Details` section (collapsed), and a QA-executable `## Test plan` per `REVIEW.md` §4. For RCs, the recipe to build the changelog from each promoted PR. Trigger on "open a PR", "write the PR description/body", "gh pr create", "release candidate", "RC", "promote main to release", "changelog for the release", or when a PR body reads like raw AI output.
---

# Decentraland Godot Explorer — PR descriptions

There are two kinds of PR here and each has its own shape:

| Kind | Base ← head | Title | Shape |
|---|---|---|---|
| **Development PR** | `main` ← feature branch | conventional commit (`feat:` / `fix:` / `chore:` / `refactor:`) | §2 |
| **Release Candidate** | `release` ← `release-X.Y.Z` | `Release Candidate X.Y.Z` | §3 |

Both follow the same principle (§1). Read it first; it decides what goes in and what stays out.

## 1. The principle: the author owns the understanding

The team's guideline for anything with your name on it — messages, PRs, docs, issues — is the
**AG scale** (AG-0 raw AI output … AG-5 you wrote it, AI helped). The target is **AG-3 to AG-5**:
the author has read every sentence, changed what didn't sound right, and can explain any part of
it if asked. *AI can replace knowledge, but it can't replace our understanding.*

For PRs this means:

- **What and why come first, in plain language.** Whoever reads `## What` and `## Why` — a reviewer,
  QA, someone on another team, someone reading the changelog in three months — understands what
  changed and why without reading anything else. If they still have to guess, the work was moved
  from the author to them.
- **Short beats complete.** More text is not more clarity. Leave things out rather than pack
  them in. Implementation detail goes in a collapsed block, or nowhere.
- **No raw AI output.** Do not paste a model's summary of the diff. Reviewers read a description
  to *validate the approach and decide what to trust*, not to re-derive the change from a wall of
  bullets. Text that restates the diff file by file, narrates the work ("I've implemented…",
  "This PR introduces a comprehensive…"), or hedges with filler is a rewrite request.
- **Tell the people affected before merging, not after.** If the change touches something
  another team, service, SDK or workflow depends on (shared UI components, the auth flow, the
  comms protocol, the asset pipeline, the mobile-bff contract, the Unity-parity behaviours, CI
  workflows), the PR is not the notification. Post a short message in the relevant shared channel
  — `#ext-foundation` for cross-org — *before* investing in the full change when possible:
  *"I'm planning to merge X, it changes Y and may affect Z. Any concerns?"* Say in the PR that
  you did.
- **QA runs the test plan by hand on a phone.** Every case must be executable cold by someone
  who has never seen the code. The full rules are in `REVIEW.md` §4 → *Writing test steps QA can
  execute*; the short form is in §2.3 below.

You (Claude) are drafting on the author's behalf. A draft is AG-2 until the author has read it.
**Always print the full body in your reply** so the author reads it before it goes out, and never
describe a change you have not read the diff for.

## 2. Development PR (base `main`)

### 2.1 Shape

```markdown
## What
<What is different after this merges — for the player, the creator, the reviewer or the
 build. 1–3 sentences, plain language. Name at most one or two identifiers.>

## Why
<The bug, the request, the measurement, the parity gap. 1–3 sentences. Include what the
 change deliberately does NOT do when a reader might assume otherwise.>

Closes #<issue>

## Details
<details>
<summary>Expand</summary>

<Only what a reviewer needs beyond What/Why: root cause, approach and trade-offs,
 screenshots/video, per-file notes when non-obvious. Short paragraphs or a tight bullet
 list. Omit the whole section when What/Why say it all.>

</details>

## Test plan

<See 2.3. Either "No QA needed — no behavior change" or numbered cases.>
```

Optional extras, in this order, only when they add clarity: a **Heads-up** line right after
`## Why` naming who was told and where (*"Heads-up posted in #ext-foundation — touches the shared
`ModalShell`"*); a **Changes** list (file → what it does) inside Details; a **Future plans** or
**Known gaps** line at the end.

### 2.2 Writing What and Why

`## What` and `## Why` are the deliverable. Everything else is optional. Write them so that
reading *only* those two sections answers three questions:

1. **What** is different for the player, the creator, the reviewer or the build after this merges.
2. **Why** — the bug, the request, the measurement, the parity gap. One clause is usually enough
   (*"…so screenshots were almost never attached"*, *"…26.5% completion vs 92%"*).
3. **What it does not do**, when a reader might reasonably assume otherwise (*"migrating the
   five existing modals is left as a follow-up"*).

Rules of thumb:

- 1–3 sentences each. If they need more, the PR probably needs splitting, or the extra belongs
  in Details. Together they must fit on one screen with the Test plan headline visible.
- Name at most one or two identifiers; describe the rest in words. Code goes in Details.
- No narration of the work, no adjectives about the work ("comprehensive", "robust", "clean").
- State behaviour changes that ride along outside the feature's scope — reviewers and QA need
  those most, and they are the easiest thing to lose in a long description.
- If the change is a port from the Unity Foundation Client, say so and name the reference.

Good What/Why (adapted from #2779):

> **What** — Replaces the "Report a Bug" Google Form deep link in Settings with a native in-app
> bug report flow that files Intercom tickets through the Decentraland `intercom-proxy`. The
> form pre-fills a screenshot of the game and lands in the same Intercom buckets as the Unity
> Explorer client.
>
> **Why** — The old flow bounced the player out to an external browser and required a Google
> sign-in to attach an image, so screenshots were almost never included.

Not a What (rewrite it): *"This PR introduces a comprehensive refactor of the bug reporting
system. Key changes include: a new `BugReportService` class…"* — the reader learns the shape of
the diff and nothing about what a player gets or why anyone wanted it.

### 2.3 Test plan

Follow `REVIEW.md` §4 exactly. In short:

- **No behaviour change** (build tooling, CI, metadata, logging level, pure refactor, docs):
  write `No QA needed — no behavior change` and, if useful, one optional verification line
  (*"CI: Static checks + Clippy green"*). Do not invent cases.
- **Behaviour change**: one block per case. **Setup** line only when the required state is
  non-obvious (specific wearables, second account, guest vs signed-in, a deeplink or flag).
  **Steps** numbered, one user action per line, **starting from opening the app**, with concrete
  on-screen names and values. **Expected** result observable enough to mark pass/fail without
  reading code. A **Regression** line whenever shared code was touched. Platform only when the
  case is iOS- or Android-specific. Device, build download and TestFlight/Firebase install are
  assumed — never spend steps on them.
- Each Expected/Regression line is a `- [ ]` checkbox so QA can tick it.
- What the author verified themselves (headless test, fmt/clippy, a device run) can go in
  Details as ticked `- [x]` items. Keep the QA section for what QA still has to do.

### 2.4 Procedure

1. Read the actual diff: `git diff origin/main...HEAD --stat` then the files that matter. Do not
   write from the branch name or the commit messages alone.
2. Find the issue it closes (`gh issue view N`) so the *why* is the real one.
3. Decide whether the change can affect another team/service/SDK/workflow. If yes, draft the
   Slack heads-up message for the author alongside the PR body and add the Heads-up line.
4. Write the body to the scratchpad and print it in full in your reply.
5. Open it only when asked, always against `origin` (`decentraland/godot-explorer`), never `fork`:
   ```bash
   git push -u origin <branch>
   gh pr create -R decentraland/godot-explorer --base main \
     --title "<type>: <summary>" --body-file <scratchpad>/pr-body.md
   ```
   To rewrite an existing PR's body: `gh pr edit <N> -R decentraland/godot-explorer --body-file …`.
6. Before handing over, run the checklist below on your own draft.

### 2.5 Self-check before it goes out

- [ ] Reading only `## What` and `## Why`, a teammate on another team knows what changed and why.
- [ ] Nothing in the description is a restatement of the diff or of the commit list.
- [ ] Every claim is something you verified in the code or the issue — no guessed behaviour.
- [ ] Ride-along behaviour changes outside the feature are stated, not buried.
- [ ] Test plan cases start from opening the app and end in an observable result.
- [ ] Affected teams are named and were (or will be) told before merge.
- [ ] Visible text outside `<details>` fits on one screen.

## 3. Release Candidate PR (base `release`)

An RC promotes `main` (or a cherry-picked subset) into `release`. Its description is the
release's changelog and QA sheet — QA works from it directly, and it is what people read when
asking "what shipped in 1.13.1?". It is built **from the promoted PRs' own descriptions**, which
is why §2 matters: a bad dev PR What/Why makes a bad release line.

### 3.1 Shape

```markdown
## Release Candidate X.Y.Z

<Lead: what is promoted (main sha or the cherry-picked set), whether it is a clean
 promotion or not, what was deliberately excluded, where the version bump lives.
 Then one or two sentences on the theme of the release in player terms.>

- **Base:** `release` · **Head:** `release-X.Y.Z`
- <N> commits on top of `release`: <one clause each when cherry-picked; omit for clean promotions>

## What's included

**Features**
- <one line, player/creator terms, ending in (#PR)>

**Fixes**
- <symptom that was fixed, with the number when it was measured, ending in (#PR)>

**Technical** — no QA needed
- <CI, tooling, back-merges, logging, dependency bumps (#PR)>

## Test plan

**Build:** `vX.Y.Z.<build>-<short sha>-prod` · one Android + one iPhone.

<When stacked on a previous RC: "Everything QA'd for X.Y.0 (#prev) carries over unchanged —
 the list below is only what this RC adds.">

- [ ] **Version** — login screen and **Settings → About** read `vX.Y.Z.<build>-<sha>-prod`
- [ ] **<Feature name>** — <the one action that proves it on a phone → what QA should see> `#PR`
- [ ] …one line per QA-relevant PR, same order as What's included…
- [ ] **Regression** — enter a few scenes, chat, change a wearable, play an emote from the wheel: no crashes

**Known gap:** <anything shipped without device validation, with why>
```

When one promoted PR is large enough to need its own paragraph (an FTUE change, a Sentry
overhaul), *What's included* may use a bold `**Name — #PR (closes #issue)**` heading with a short
paragraph under it instead of a single line — see #2797. Keep that for one or two items, not all.

### 3.2 Procedure

1. **Establish the range.** From an up-to-date checkout:
   ```bash
   git fetch origin main release
   git log --oneline origin/release..origin/main            # what main has that release doesn't
   git log --oneline origin/release..origin/release-X.Y.Z   # what the RC branch actually carries
   ```
   If the two differ, the RC is a cherry-pick: list what is in and, in the lead, what was left
   out and why. Commit subjects carry the PR number as `(#NNNN)`; extract them:
   ```bash
   git log --format=%s origin/release..origin/release-X.Y.Z | grep -o '#[0-9]\+' | sort -u
   ```
2. **Read every promoted PR**, not just its title:
   ```bash
   gh pr view <N> -R decentraland/godot-explorer --json title,body,closingIssuesReferences
   ```
   From each take: `## What` and `## Why` (→ one changelog line), the behaviour changes that ride
   along (→ often their own line, or a Known gap), and the one or two test-plan cases that prove
   the change on a phone (→ one Test plan line). Do not copy a PR's full test plan into the RC.
3. **Classify** each PR: Feature (new capability a player or creator sees), Fix (a symptom went
   away — quote the measurement when the PR has one), Technical (nothing QA can observe). A
   `chore:` that changes runtime behaviour is not Technical.
4. **Confirm the version.** `lib/Cargo.toml` and `godot/export_presets.cfg` carry it; say in the
   lead which PR bumped it. The Test plan's **Version** line is not optional — it is how QA proves
   they are on the right build.
5. **Carry-over.** For a patch RC on top of a previous one (1.13.1 after 1.13.0), reference the
   previous RC and list only the additions, both in *What's included* and in the Test plan.
6. Write the body to the scratchpad, print it in full, then when asked:
   ```bash
   gh pr create -R decentraland/godot-explorer --base release --head release-X.Y.Z \
     --title "Release Candidate X.Y.Z" --body-file <scratchpad>/rc-body.md
   ```

### 3.3 Writing the changelog lines

- Player and creator terms first, the mechanism only when it is the news:
  *"Nearby players rendered as discrete frames, and bounced after landing (#2778)"* — not
  *"Interpolate remote avatar transforms and clamp ground snap (#2778)"*.
- One line per PR. Two PRs that ship one feature share a line with both numbers.
- Numbers only when they came from the PR and change what the reader thinks
  (*"26.5% completion vs 92%, ~17 logins/day"*).
- Under *Technical* the reader should be able to skip the block entirely; nothing there needs a
  test case.
- Test plan lines pair a **bold name** with one concrete action and the visible outcome, and end
  with the PR number in backticks so QA can jump to the source when a case fails.

### 3.4 Self-check

- [ ] Every commit in `origin/release..head` is accounted for by a line or explicitly excluded.
- [ ] Each line was written from the PR's body, not from its title alone.
- [ ] Every Feature and Fix line has a Test plan line; every Technical line has none.
- [ ] Version line present and matching `lib/Cargo.toml`.
- [ ] Anything shipping without a device run is under **Known gap**, not silently omitted.
- [ ] The lead says clean promotion or cherry-pick, and what was left behind.
