---
name: godot-issue-writer
description: "Create GitHub issues and bug reports in decentraland/godot-explorer in the Regenesis Labs PM house style — features/tasks use Problem/Scope/Deliverables/References, bugs use the repo's bug_report.yml form. Mines prior art, sets labels, type, DAO Explorers project, Status=planning."
---

# Godot Explorer — Issue & Bug Writer

Repo: `decentraland/godot-explorer` · Board: org `decentraland` project 43 ("DAO Explorers"), https://github.com/orgs/decentraland/projects/43

## Which template

| Kind | Template |
|---|---|
| Feature / Task (any repo) | House template below — Problem / Scope / Deliverables / References. |
| Bug | Repo bug form (`.github/ISSUE_TEMPLATE/bug_report.yml`) — see Bug reports. Never the house template. |

Never infer style from existing issues in the destination repo. Cross-repo features/tasks (e.g. `decentraland/docs`) still take the full house template.

## Voice — applies to every word written into an issue or bug

Maximum density. Direct speech. Cut anything removable without losing meaning.

- No adjectives or adverbs unless load-bearing. No "comprehensive", "seamless", "robust", "clear", "proper", "overall", "significantly".
- No hedges: "should", "aims to", "in order to", "we will look at", "it is important that".
- Drop articles, copulas, and connective filler where the meaning survives. Broken English beats padded English.
- Fragments and telegraphic style are correct here. `Console/Debug button — placement, icon, states` not `Design the placement, icon and states of the Console/Debug button.`
- One idea per line. Never restate a heading in the text beneath it.
- Symbols over words: `+` `→` vs "and"/"to". Numbers as digits.
- Problem ≤ 2 sentences. Scope ≤ 2 sentences. Deliverables ≤ 12 words each. Notes = bare questions.
- Before submitting, reread each line and delete a word. If the meaning holds, keep it deleted.

## Non-negotiables

| Field | Rule |
|---|---|
| Issue Type | Feature if code ships. Task if not (research, audit, docs, definition, design spec, store/legal). Bug for defects. |
| Project | Always DAO Explorers (org decentraland, project 43). |
| Status | Always planning. |
| Labels | ≥1 platform label + ≥1 domain label. |
| Body | Feature/Task → house template; Problem and Deliverables never empty. Bug → bug form. |

## House template (Features & Tasks)

Reproduce exactly — emoji headers, bold-inside-heading (see #2265, #2255, #2348).

```
##  **⚠️ Problem:**
<≤2 sentences. User pain, player/creator POV.>

## **🏁 Scope:** <optional: [See PRD](link) or [SEE FIGMA](link)>
<≤2 sentences. What ships. Bounded — reader can tell what's out.>

## **📝 Deliverables:**
- [ ] <verifiable, imperative, ≤12 words>
- [ ] <analytics/tracking item, if the feature needs instrumentation>

## **🔗 References:**
- [decentraland/unity-explorer#NNNN](url) — <why it matters here, one line>
- [decentraland/docs#NNN](url) — <protocol/SDK behavior, or the doc gap>
```

No Metrics section. Optional, only when carrying real information: `## Dependency` (#2593), `## Context` (#2533), `## Notes` (#2595), `## Implementation Specs:` (#2280, #2368). Figma/Notion links and `<img>` inline, as the repo does.

### Problem

≤2 sentences. User pain from the player/creator POV — what breaks, blocks, or frustrates, and when. No metrics, KPIs, targets, or percentages. Describe the pain, not the solution.

> Guest players lose their account when switching devices — no way to upgrade a guest session to a recoverable account.

### Scope

≤2 sentences. What we implement. Bounded, not aspirational. Link PRD/Figma in the heading (`## **🏁 Scope:** [See PRD](...)`), as #2348.

### Deliverables

Checkboxes. Imperative. Each independently closeable by a reviewer. Conditions that must be true at close, not an engineering task breakdown. Feature needs tracking → one deliverable is the instrumentation, named by event:

- `SCREEN_VIEW` event for `ACCOUNT_UPGRADE_MODAL_SHOW` implemented

Design/component work: two levels OK (components, then variants) — #1157.

### Dependency

Issue depends on another → `## Dependency` naming the blocker issue and what the dependency means in practice; also list the blocker in References with its why-line. GitHub's native blocked-by link can't be set via MCP → tell the user to link it by hand.

## Bug reports

Bugs use the repo's bug form shape, matching how other bug issues render in the repo (reference: #2885). Engineers scan these fields — keep them exact.

Title: `[Bug]: <symptom> (<platform>)`

Body — rendered headings, exactly:

```
### 🕹️Platform
<iOS / Android / Desktop — all that apply>

### 🔢App Version
<version/build, or "Not specified">

### 📱Device Information
<device + OS, or "Not specified — reproduced on mobile build">

### 📄Issue description
<symptom · trigger · root cause (file/flag names) · why it differs from any closed lookalike · repro snippet · workaround>

**Related**
- [repo#NNN](url) — <why, one line>

### ✅Expected behavior
<observable condition — flag values, timings, visible result. Not an engineering task list.>

### 🖼️Screenshots / Media
<links/inline media, or _No response_>

### ▶️Steps to reproduce
1. <step>
2. <step>

### 👁️Occurrence
<Always / Often / Sometimes / Rarely, or _No response_>
```

Rules:

- Unknown field → say so in place (`Not specified`, `_No response_`). Never invent a device, version, or occurrence rate.
- Issue description carries the technical weight. Related links each get a why-line.
- Search for duplicates and closed lookalikes first (godot-explorer + unity-explorer `[QA]` issues). Same root cause elsewhere → link both.
- House voice still applies inside the fields.
- Labels: `bug` + platform + domain.
- Issue Type → Bug (settable via MCP update). Project 43 + Status planning still need manual setting when tools can't.

## References — mine the other repos first

Never file without checking whether the feature exists, was specified, or broke elsewhere in the org.

| Repo | Use for |
|---|---|
| `decentraland/unity-explorer` | Client prior art. Desktop is usually the reference implementation and parity target. Behavior, spec issue, QA bugs on the same surface. |
| `decentraland/docs` | Protocol/SDK behavior. Component definitions, proto fields, defaults, creator-facing contracts. Doc gaps our change creates. |

Secondary, only when a hit surfaces them: `decentraland/protocol`, `decentraland/js-sdk-toolchain`, ADRs (e.g. ADR-290). Follow the trail, don't search them cold.

### How

1. 3–6 keywords from the Problem — feature name, SDK component, UI surface (`AvatarModifierArea`, nametag, emote wheel, credits checkout).
2. `mcp__github__search_issues`, `repo:decentraland/unity-explorer <keywords>`, open + closed. Closed carries the resolution and the PR.
3. Same on `repo:decentraland/docs`.
4. Nothing → `mcp__github__search_code` on unity-explorer for the class/component name, to point engineers at the implementation.
5. Open the best 2–4. Cite only what you read.

Search output is often too large for context. `mcp__github__search_issues` on broad terms returns 100k+ chars and gets spilled to a file. Narrow with `in:title` and specific terms; if it still spills, grep the spill file for `"title"` / `"number"` rather than reading it whole.

### Reading unity-explorer hits

- `[QA]` prefix = QA filing; severity in `1-high` / `2-medium` + a Priority field. High-severity QA on the same surface → mention it in the Problem, not just References.
- Bodies: Build version / Issue Description / STR / Expected / Actual. Their Expected Result is often the cleanest statement of intended behavior — quote in Scope (or in a bug's Expected behavior).
- Intended-and-shipped on desktop → parity gap → label `feature parity`. Broken there too → shared root cause → link both.

### Rules

- 2–5 links. Curated, not a search dump.
- Every link gets one line of why. Bare URL is not a reference.
- Verify each resolves and says what you claim. Never cite an issue you did not open.
- Negative result stated explicitly, as the repo does (#2595): `No prior art in unity-explorer or docs (searched <terms>, <date>).`
- Creator-visible behavior change → add a deliverable to file the `decentraland/docs` issue, link it here once filed.

## Labels

Smallest set that is true.

- Kind (one): `enhancement` (default for Feature) · `bug`
- Platform (all that apply): `mobile` (default) · `iOS` · `Android` · `desktop`
- Domain: `credits` · `controls` · `rendering` · `metrics` · `feature parity`
- Process: `needs design` · `need definition` · `blocked` · `triage` · `release` · `claw-created`

`claw-created` only when the request came via Slack/Discord rather than the user authoring it directly; then close the body with `**Requested by <name> via Slack**` (#2091, #2089). Direct authorship → omit both.

Unsure → check similar recent issues: `mcp__github__list_issues` with labels, or `mcp__github__search_issues` with `repo:decentraland/godot-explorer <keywords>`.

## Board fields

After creating, add to project 43 and set:

- Status → planning (always)
- Priority → 0-Critical / 1-High / 2-Medium / 3-Low — ask if not stated
- Estimate (Days) → only if given
- Sprint → only if explicitly targeted

Don't guess option names. Read field definitions first (GraphQL `projectV2 { fields }`, or `gh project field-list 43 --owner decentraland`) and match exact option IDs.

`gh` and the GraphQL API are often unavailable — the sandbox has no `gh`, and the GitHub MCP exposes no project-field mutation. When that happens: create the issue anyway, then tell the user plainly which fields need manual setting (Type, Project, Status, Priority) and link both the issue and the board. Do not silently skip this.

## Workflow

1. Gather. Request + source. Read linked Figma/Notion/PRD if accessible. For bugs: platform, version, device, repro steps, media.
2. Classify. Bug → bug form. Otherwise Feature vs Task → house template. Labels.
3. Check duplicates and parents. `mcp__github__search_issues` on `repo:decentraland/godot-explorer`. Surface near-duplicates before creating. Find the epic to link.
4. Mine cross-repo prior art. Before drafting — findings reshape Problem and Scope (or a bug's root cause), not just References.
5. Ask, don't invent. Missing design link, unclear boundary, unknown bug field → ask or mark unknown. Filing with `## Notes` + `need definition` is fine. Fabricated data is not.
6. Draft and confirm. Show the full issue — title, body, labels, type, project, status. Get approval before creating.
7. Create. `mcp__github__create_issue`. Then type, project 43, Status = planning.
8. Report. Issue URL + which fields were set + which need manual attention.

## Titles

Short, specific, no ticket-speak.

- Feature: `Controls Customization: Adaptive Controls Implementation`, `Show Community Restriction Modal`
- Workstream prefix: `IAP - Update Pricing and Quantities`, `[Data] Guest User Data Model - Implementation`
- Bugs: `[Bug]: <symptom> (<platform>)`
- Design and implementation split into two issues when both needed (#2265 → #2348).

## Gold standards

| Issue | Why |
|---|---|
| #2265 | Canonical structure; problem framed as quality/frustration risk. |
| #2255 | Problem grounded in player pain (lost guest accounts across devices). |
| #2348 | #2265 restated for implementation; Scope links the PRD. |
| #2368 | Problem / Scope / hard-vs-soft limits table. Model for definition issues. |
| #2187 | Deliverables include regression guards + analytics audit. |
| #2336 | Tracking as a deliverable. |
| #2280 | Task type: Problem / Scope / Implementation Specs with production values inline. |
| #2668 | Design-definition Task in the condensed voice above. Reference for density. |
| #2885 | Bug in the repo bug form — reference for bug reports. |

## Anti-patterns

- Padded prose. Adjectives. Hedges. Full sentences where a fragment works.
- Metrics, KPIs, or target percentages in the Problem, or a Metrics section.
- Problem that describes the solution.
- Scope that restates the Problem or lists deliverables.
- Deliverables that restate the title.
- A bug written in the house template, or a feature in the bug form.
- Invented percentages, revenue, user counts, devices, or app versions.
- Filing without checking for a parent epic or duplicate.
- Bare URLs in References, or padding with loose hits.
- Citing an issue you did not open.
- Skipping cross-repo search because the feature "obviously" has no precedent.
- Status unset, or not added to DAO Explorers.
