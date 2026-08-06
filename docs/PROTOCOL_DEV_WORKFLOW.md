# Protocol Development Workflow

This document describes the end-to-end process for adding or changing something in the Decentraland protocol and rolling it out across the SDK and the explorer.

## Repositories involved

| Repo | Role |
| --- | --- |
| [`decentraland/protocol`](https://github.com/decentraland/protocol) | Protocol definition (protobuf). Source of truth for components, messages and RPCs. |
| [`decentraland/js-sdk-toolchain`](https://github.com/decentraland/js-sdk-toolchain) | The Decentraland SDK. Consumes the protocol to generate the component definitions scenes use. |
| [`decentraland/godot-explorer`](https://github.com/decentraland/godot-explorer) | The mobile explorer. Consumes the protocol to interpret scenes and render them. |

`js-sdk-toolchain` and `godot-explorer` communicate over the protocol, so any change made here must land in both before it reaches users.

## Branches and npm channels in `protocol`

| Branch / event | npm artifact | Audience |
| --- | --- | --- |
| Open PR | A per-PR CDN tarball (URL posted in the PR) | Local testing, draft PRs in dependent repos |
| Merge to `main` | `@dcl/protocol@next` | Production. Default channel for both SDK releases and the explorer. |
| Merge to `protocol-squad` | `@dcl/protocol@protocol-squad` | Legacy iteration channel. Kept for changes that still need a staging period before committing to `main`. |

### Why two branches?

`main` is production and **must not break compatibility**. Once a scene is deployed against a given protocol shape, that scene lives with that shape forever — we cannot retroactively patch deployed scenes.

`protocol-squad` is the legacy iteration channel for cases where we want to expose a change to real explorer users before committing to `main`. Most protocol changes today go straight to `main` via Flow A; `protocol-squad` is reserved for changes whose shape is still in flux.

**Invariant:** `protocol-squad` is always a superset of `main` and is always backward compatible with it. That guarantee is what makes shipping the explorer against `protocol-squad` safe when Flow B is needed.

---

## Which flow should I use?

There are two flows. Default to Flow A; only fall back to Flow B if the change really needs a staging period.

| Flow | Use when | Risk |
| --- | --- | --- |
| **A. Straight to `main`** (default) | The default for any protocol change. The shape is settled (or close to it) and end-to-end validation against the per-PR tarball will catch issues before merge. | Anything that lands in `main` is permanent for deployed scenes — no second chances. Validate thoroughly before merging. |
| **B. Through `protocol-squad`** (legacy) | The shape might still change after exposure to real explorer users, or you specifically want a staging period before committing to `main`. | Lower in the short term — the iteration branch absorbs mistakes — but adds a cherry-pick step and another dependency channel to keep in sync. |

When in doubt, use Flow A. Reserve Flow B for changes where you genuinely expect to revise the shape after seeing it in production-shaped use.

---

## Flow A — Straight to `main` (default)

Example scenario: adding a new optional field to an existing component.

### 1. Open the protocol PR against `main`

- Branch off `main` in `decentraland/protocol`.
- Add the `.proto` change.
- Open the PR with `main` as the base.
- CI publishes a per-PR tarball — copy the URL from the PR comment.

### 2. Open a `js-sdk-toolchain` PR pointing at the per-PR tarball

- Branch off `main` in `js-sdk-toolchain`.
- Point the `@dcl/protocol` dependency at the per-PR tarball.
- Implement SDK-side support.
- Open the PR. Do **not** merge yet.

### 3. Open a `godot-explorer` PR pointing at the per-PR tarball

- Branch off the explorer's default branch.
- Point the protocol dependency at the same per-PR tarball.
- Implement explorer-side support.
- Open the PR. Do **not** merge yet.

### 4. Validate end-to-end

With all three PRs wired to the per-PR tarball, verify the feature actually works:
- A test scene built with the SDK PR renders correctly in the explorer PR.
- Existing scenes still work (no regressions).
- Iterate. If the protocol shape needs to change, push to the protocol PR — a new tarball will be published — and update the SDK / explorer PRs to consume it.

**Stop point:** if validation surfaces anything that makes the shape feel uncertain, switch to Flow B — re-target the protocol PR at `protocol-squad` and continue from Flow B step 5. It is cheap to switch now; it is impossible to switch later.

### 5. Merge the protocol PR into `main`

- Merge the protocol PR.
- CI publishes `@dcl/protocol@next`.

### 6. Update both dependent PRs to use `@dcl/protocol@next`

- In the `js-sdk-toolchain` PR, replace the per-PR tarball with `@dcl/protocol@next`.
- In the `godot-explorer` PR, replace the per-PR tarball with `@dcl/protocol@next`.
- Confirm CI is green on both.

### 7. Merge the explorer and SDK PRs

- Merge `godot-explorer`.
- Merge `js-sdk-toolchain`.

### 8. Release the SDK

- Cut an SDK release containing the change.
- Scene creators can now use it.

---

## Flow B — Through `protocol-squad` (legacy)

Use this only when the shape might still need to change after real-world exposure, or when you specifically want a staging period in front of explorer users before committing to `main`. If your change does not need that, use Flow A instead.

Example scenario: adding a new component whose shape is still being negotiated.

### 1. Open the protocol PR against `protocol-squad`

- Branch off `protocol-squad` in `decentraland/protocol`.
- Add / modify the `.proto` definitions.
- Open the PR with `protocol-squad` as the base.
- CI will publish a per-PR tarball — copy that URL from the PR comment, you will need it in the next steps.

### 2. Open a `js-sdk-toolchain` PR pointing at the per-PR tarball

- Branch off `main` in `js-sdk-toolchain`.
- Replace the `@dcl/protocol` dependency with the per-PR tarball URL from step 1.
- Implement the SDK-side support for the new component (types, helpers, tests).
- Open the PR. Do **not** merge yet.

### 3. Open a `godot-explorer` PR pointing at the per-PR tarball

- Branch off the explorer's default branch.
- Point the protocol dependency at the same per-PR tarball from step 1.
- Implement the explorer-side support (deserialization, rendering, systems).
- Open the PR. Do **not** merge yet.

### 4. Validate end-to-end

With all three PRs wired to the per-PR tarball, verify the feature actually works:
- A test scene built with the SDK PR renders correctly in the explorer PR.
- Existing scenes still work (no regressions).
- Iterate. If the protocol shape needs to change, push to the protocol PR — a new tarball will be published — and update the SDK / explorer PRs to consume it.

### 5. Merge the protocol PR into `protocol-squad`

Once the shape is validated:
- Merge the protocol PR into `protocol-squad`.
- CI publishes `@dcl/protocol@protocol-squad` with the new feature included.

### 6. Open a cherry-pick PR from `protocol-squad` to `main`

- Cherry-pick the merged commit onto a new branch off `main`.
- Open a PR targeting `main`.
- This is the moment to be conservative: anything that lands here is permanent for deployed scenes. If you are not 100% sure about a field name, default value, or semantics, **keep iterating on `protocol-squad` and delay the cherry-pick**.

### 7. Update the `godot-explorer` PR to use `@dcl/protocol@protocol-squad`

- Replace the per-PR tarball in the explorer PR with `@dcl/protocol@protocol-squad`.
- Confirm CI is green.
- Merge the explorer PR.

Because `protocol-squad` is always backward compatible with `main`, shipping the explorer against `protocol-squad` is safe for existing scenes.

### 8. Merge the cherry-pick into `protocol` `main`

- Merge the PR from step 6.
- CI publishes `@dcl/protocol@next`.

### 9. Update the `js-sdk-toolchain` PR to use `@dcl/protocol@next`

- Replace the per-PR tarball in the SDK PR with `@dcl/protocol@next`.
- Confirm CI is green.

### 10. Merge the `js-sdk-toolchain` PR

- Merge it into the SDK's default branch.

### 11. Release the SDK

- Cut an SDK release containing the new component.
- Scene creators can now use it.

---

## Why this order matters: the backward compatibility contract

Scenes deployed to the world are immutable. If a scene was deployed using a component shape that turns out to be wrong, that scene will keep sending that wrong shape forever. We cannot patch it.

Consequences:
- **The explorer can be fixed.** If the explorer mis-handles a shape, we ship a new explorer.
- **The SDK can be fixed.** If the SDK generated bad code, we cut a new SDK and creators redeploy.
- **A deployed scene cannot be fixed.** Whatever shape it serializes is what we are stuck with.

That asymmetry is why `protocol-squad` exists at all. It is the staging branch where we are allowed to be wrong. `main` is the branch where we are not — so Flow A leans on the per-PR tarball for end-to-end validation before anything touches `main`.

### Rules of thumb

- Never merge a protocol change to `main` until the SDK and explorer have validated it end-to-end against the per-PR tarball (Flow A) or against `@dcl/protocol@protocol-squad` (Flow B).
- Never name a field "temporary" — there is no such thing once it reaches `main`.
- If something feels uncertain during Flow A validation, switch to Flow B and let it bake on `protocol-squad` for another iteration. Cherry-pick later.
- When running Flow B, the explorer ships from `@dcl/protocol@protocol-squad` while the SDK ships from `@dcl/protocol@next` (i.e. `main`). That asymmetry is what keeps the invariant intact.

---

## Quick reference: which tag do I depend on?

### Flow A (straight to `main`)

| You are working in… | While iterating (PR open) | After feature lands |
| --- | --- | --- |
| `js-sdk-toolchain` | per-PR tarball URL | `@dcl/protocol@next` |
| `godot-explorer` | per-PR tarball URL | `@dcl/protocol@next` |
| A scene (creator) | n/a | whichever SDK version is released |

### Flow B (through `protocol-squad`)

| You are working in… | While iterating (PR open) | After feature lands |
| --- | --- | --- |
| `js-sdk-toolchain` | per-PR tarball URL | `@dcl/protocol@next` |
| `godot-explorer` | per-PR tarball URL | `@dcl/protocol@protocol-squad` |
| A scene (creator) | n/a | whichever SDK version is released |
