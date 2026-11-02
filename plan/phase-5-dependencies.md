# Phase 5: Rust dependency review

Measured 17 Oct on a Linux release build of `dclgodot` (`cargo bloat --release --crates`): `.text` is
66 MiB, the `.so` 368 MiB (debug info on, `strip = false`; the shipped size is the `.text` column plus
data). 646 crates in the workspace graph. Sizes are bloat's attribution, approximate.

## Where the code is

| Crate | .text | Verdict |
|---|---|---|
| `v8` (+ `deno_core`, `serde_v8`) | 16.5 MiB + 0.9 | Needed: the SDK scene runtime. The one real lever is V8 build flags / snapshot, not a crate swap. |
| `webrtc_sys` + `livekit` + `libwebrtc` | 8.0 MiB + 0.8 | Needed for voice and streams. Already optional (`use_livekit`); a build without it is the size lever for a "scene + avatar only" export. |
| `godot_core` / `godot_ffi` | 7.1 + 1.9 MiB | The bindings; the API-json build binds only to what the fork exposes. |
| `dclgodot` / `dcl_core` | 6.6 + 1.7 MiB | Our code. |
| `std` | 4.8 MiB | Fixed. |
| `tokio` | 1.3 MiB | `features = ["full"]` in `dcl_engine`; could be trimmed to what is used. |
| `reqwest` | 0.5 MiB | In use (38 sites). Two stacks coexist, see below. |
| `regex_*` + `aho_corasick` | ~0.9 MiB | 2 call sites, `regex = "*"` unpinned. Candidate to replace with plain string code. |
| `rustls`, `ring`, `h2`, `hyper` | 0.9 MiB | Needed by the HTTP/WS stack. |
| `ethers_providers` | 0.18 MiB | Pulled by `dcl_core` (`ws`, `rustls`) and by `ethers-signers`' tree. |
| everything else | each < 0.3 MiB | Not worth a swap on size alone. |

The 86 smallest crates add 150 KiB together. Dependencies are not the size problem: V8, WebRTC and the
bindings are 60% of the code; a rewrite of small crates would move the binary by well under 1%.

## Done in this commit

- Removed nine unused direct dependencies from `dcl_engine` (`cargo machete` found them, `cargo clippy` and
  the 511 unit tests confirm nothing used them): `cpal`, `ethers-providers`, `num`, `poll-promise`,
  `protobuf`, `thiserror`, `tokio-util`, `tracing-subscriber`, `tracing-test`. 266 lines left `Cargo.lock`.
- `reqwest`'s `blocking` feature is off in `dcl_engine` (0 uses).

## Candidates, not done (each needs a decision or a device check)

- **Duplicated stacks** (`cargo tree -d`): `hyper` 0.14 and 1.x, `http` 0.2 and 1.x, `h2` 0.3 and 0.4,
  `reqwest` 0.11 (pulls the old stack), `prost` 0.11 and 0.12 (livekit), `base64` 0.13 and 0.21,
  `thiserror` 1 and 2, `syn` 1, 2 and 3. Moving `reqwest` to 0.12 removes most of the hyper/http/h2 doubles;
  it touches every HTTP call, so it is its own change.
- **`ethers-core` / `ethers-signers`** (maintenance mode, pulls `k256`, `elliptic-curve`, `bls`/`scrypt`
  trees): used at 55 sites for signing. `alloy` is the maintained replacement; a migration, not a swap.
- **`async-trait`** (59 uses): native `async fn` in traits covers most; the dyn-compatible ones keep it.
- **`once_cell` (11), `futures-lite` 1.x (5)**: `std::sync::LazyLock` / `futures` already in the tree.
- **`json5` (4), `multipart` 0.18 (14), `simple-easing` (31), `fastnoise-lite` (2), `modular-bitfield` (2)**:
  small and single-purpose; keep unless a call site is touched anyway.
- **`tokio` `full`**: trim to the used features.
- **`regex = "*"`**: pin it at least; 2 uses.
- **Release profile**: `debug = 1`, `strip = false`, `lto = "thin"`. For shipped binaries `lto = "fat"`,
  `codegen-units = 1`, `panic = "abort"` (only if no code relies on unwinding) and stripping are the
  actual size wins; measure on the Moto before adopting, since `panic = "abort"` changes crash reporting.
- **Per-build feature split** for the planned scene-only / avatar-only exports: `use_livekit` and `use_deno`
  are already features; the avatar-only export would not need `use_deno` (V8 is the largest block).
