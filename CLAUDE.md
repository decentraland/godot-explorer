# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

Decentraland Godot Explorer is a cross-platform metaverse client that combines:
- **Godot Engine 4.6.2** (custom fork) for 3D rendering and UI
- **Rust** for core systems and performance-critical components
- **Rust** also owns the game logic and the in-world UI: there is no GDScript in the engine project
- **JavaScript/V8** runtime for executing Decentraland SDK scenes

## Essential Commands

All commands use the xtask pattern via `cargo run --`:

### System Health & Dependencies
```bash
# Check system health and dependencies
cargo run -- doctor

# Install dependencies (specify platforms: linux, windows, macos, android, ios)
cargo run -- install                      # Installs protoc and Godot only (fresh download)
cargo run -- install --targets linux      # Also installs Linux export templates
cargo run -- install --targets android    # Also installs Android tools and templates
cargo run -- install --targets ios        # iOS templates (keeps debug symbols)
cargo run -- install --targets ios --strip-ios  # iOS templates (strips debug symbols to save disk)
cargo run -- install --cache              # Use cached downloads instead of re-downloading
cargo run -- install --cache --targets ios --strip-ios  # Cached + strip iOS

# Strip iOS templates manually (saves ~1.9GB disk space)
cargo run -- strip-ios-templates

# Clear download cache
cargo run -- clean-cache
```

### Development
```bash
# Build the Rust library
cargo run -- build                        # Build for host platform
cargo run -- build -r                     # Release mode
cargo run -- build --target android       # Android build (no cargo-ndk, uses direct cargo)
cargo run -- build --target ios           # iOS build (macOS only)

# Run the client (automatically builds first)
cargo run -- run                          # Run client (always a phone layout: --emulate-ios by default)
cargo run -- run -- --emulate-android     # Run client with the Android layout instead
cargo run -- run -r                       # Release mode
cargo run -- run -e                       # Run editor
cargo run -- run -e --target android      # Run editor and also build for Android
cargo run -- run --target android         # Build, export APK, and deploy to device
cargo run -- run --target ios             # Build, export IPA, and deploy to device
cargo run -- run --target android --only-lib  # Build and push .so only (requires debug APK)
cargo run -- run --target android --only-lib -- --skip-lobby  # Hotreload with app parameters

# Feature flags
cargo run -- build --no-default-features --features use_livekit,use_deno
cargo run -- run --no-default-features --features use_livekit,use_deno

# Run tests
cargo run -- run --itest                  # Integration tests
cargo run -- run --stest                  # Scene tests
```

### Android Development
```bash
# Export Android builds (keystores are generated automatically if needed)
cargo run -- export --target android --format apk           # Build debug APK (uses debug keystore)
cargo run -- export --target android --format apk --release # Build signed APK (uses release keystore)
cargo run -- export --target android --format aab --release # Build AAB for Play Store

# Note: Keystores are automatically created in .bin/ folder:
# - .bin/debug.keystore for debug builds
# - .bin/release.keystore for release builds
```

### Mobile app (`rn/`)
The shipping iOS/Android app is the React Native app in `rn/`, which embeds the Godot client (see
`rn/README.md`). `export`/`run --target android|ios` run Godot's export and then build the RN app from it
(`--godot-only` stops after Godot's export). The engine is optional at build time, so the UI can be iterated on
without it:
```bash
cargo run -- run --target ios --no-godot       # simulator, mock bridge + placeholders
cargo run -- run --target android --no-godot   # emulator
cd rn && npm start                              # Metro + fast refresh; then, in another shell:
cd rn && npm run android:no-godot               # or: npm run ios (simulator), npm run expo (Expo Go)
```
iOS needs the engine built with `embedded_host=yes`: the `ios_host_release.a` template (`install --targets ios`)
or `GODOT_HOST_LIB=<path to libgodot.ios.template_release.arm64.a>`.

### Code Quality
```bash
# Rust formatting and linting
cd lib
cargo fmt --all
cargo clippy -- -D warnings

# Generate test coverage
cargo run -- coverage --dev

# Import assets
cargo run -- import-assets
```

### Export & Distribution
```bash
# Export for target OS
cargo run -- export --target linux
cargo run -- export --target windows
cargo run -- export --target macos
cargo run -- export --target android --format apk
cargo run -- export --target android --format aab
cargo run -- export --target ios            # dev export (debug template)
cargo run -- export --target ios --release  # what CI ships to TestFlight / the App Store
```

iOS exports run a **windowed** Godot editor (not `--headless`) because the shader baker only works
with a RenderingDevice renderer, and they need the **Metal toolchain** (`xcodebuild -downloadComponent
MetalToolchain` on Xcode 26) to compile the baked shaders to `.metallib`. The export fails unless
Godot's output shows `Started Baking shaders (N steps)` and packed `.metal.cache` files, and it also
fails when the SPIR-V-only warning (missing toolchain) appears; `DCL_SKIP_SHADER_BAKE_CHECK=1`
bypasses that check for local debugging only. The baked files live in
`godot/.godot/exported/<hash>/shader_baker/iOS/metal/`, not next to the exported IPA.

## Architecture

### Directory Structure
- **`lib/`**: Cargo workspace (target dir, lockfile and version live here)
  - `crates/dcl_core/`: engine-free core (no `godot` dependency, `cargo test -p dcl_core` runs without Godot): `Core` runtime, `env`/`urls`/`deep_link`, `session` (wallet, auth chain, thirdweb, `session.json`), `http` queue, `realm`, `flags`, `destination`, `analytics`, `notifications`
  - `crates/dcl_engine/`: the Godot GDExtension (package `dclgodot`); the `Dcl*` classes are adapters over the core
    - `src/dcl/`: Decentraland protocol types, SDK bindings, JS runtime glue
    - `src/scene_runner/`: Scene threads, CRDT handling, per-component systems (`components/`)
    - `src/av/`: Audio/video processing (video player)
    - `src/comms/`: WebRTC, voice chat (livekit)
    - `src/auth/`: `DclPlayerIdentity` adapter (the auth logic is in `dcl_core::session`)
    - `src/content/`: Asset loading and caching
- **`godot/`**: Godot project
  - `src/decentraland_components/`: Custom Godot nodes for DCL features
    - `avatar/impostor/`: Distance-based avatar LOD (billboard sprites for far avatars). Render handled by `MultiMeshInstance3D` + `Texture2DArray` in Rust `AvatarScene`; capture path uses scene-tree off-screen `AvatarPreview`.
  - `src/ui/`: the in-world controls only (explorer world, joypad, emote wheel, pointer tooltip, safe-area layouts); every screen and modal is React Native in `rn/`
  - `src/tool/`: Editor tools
- **`src/`**: xtask build system

### Key Systems

1. **Scene Management**: Scenes are loaded via the JavaScript runtime (deno_core/v8) which executes Decentraland SDK code
2. **Avatar System**: Handles wearables, animations, and customization through GLTF models
3. **Content Delivery**: Uses IPFS and content servers for asset distribution
4. **Voice Chat**: Integrated livekit WebRTC for spatial audio
5. **Ethereum Integration**: Wallet connection for Web3 features

### Platform Support
- Desktop: Linux, Windows, macOS
- Mobile: Android (API 29+), iOS

## Development Workflow

1. **Always run formatting before commits**:
   ```bash
   cd lib && cargo fmt --all
   ```

2. **Test changes with**:
   ```bash
   cargo run -- run
   ```

3. **For Android development**:
   ```bash
   # First install Android tools and dependencies
   cargo run -- install --targets android

   # Build for Android
   cargo run -- build --target android

   # Export APK or AAB (keystore is automatically generated and configured)
   cargo run -- export --target android --format apk --release

   # Or use Docker (for CI/CD):
   docker run -v $(pwd):/app/ -it kuruk/dcl-godot-android-builder-rust
   ```

4. **For iOS development** (macOS only):
   ```bash
   # Install iOS templates (keeps debug symbols by default)
   cargo run -- install --targets ios

   # Install with stripping (saves ~1.9GB disk space)
   cargo run -- install --targets ios --strip-ios

   # Strip already-installed templates manually
   cargo run -- strip-ios-templates
   ```

   **Important notes about iOS templates:**
   - Local installs **keep debug symbols by default** (useful for Sentry crash symbolication)
   - Use `--strip-ios` to strip debug symbols and save disk space (~2.1GB -> ~234MB)
   - Use `--cache` to reuse previously downloaded files instead of re-downloading
   - The strip command auto-detects already-stripped templates (< 1GB) and skips

5. **Android APK hosting & mobile distribution**:
   Every Android CI build (`android_builds.yml`, on every push/PR) uploads the signed
   APK to the R2 mobile-artifacts bucket and links it from the PR build-report comment.
   Required repo secrets: `MOBILE_ARTIFACTS_R2_{ACCESS_KEY_ID,SECRET_ACCESS_KEY,ENDPOINT,BUCKET}`.
   APKs are served at `https://mobile-artifacts.dclregenesislabs.xyz/android/<branch>/<sha>/decentraland.godot.client.apk`.

   `mobile_distribute.yml` builds & ships iOS → TestFlight and, once the commit's APK is
   in R2, surfaces the APK (and, for **`main`/`release`** builds, the **AAB** — only uploaded
   for those). It does not rebuild Android and does not push to any store.

   Status reporting lives in the mobile board (`dcl-regenesislabs/mobile-board`, a Cloudflare
   Worker): every leg POSTs one event with `.github/scripts/report-build-event.py` (secret
   `BUILD_REPORTER_TOKEN`; env-var interface: `STATUS`, `IOS_LINE`, `ANDROID_LINE`, `APK_URL`…,
   plus `MESSAGE` for a Slack thread reply and `LOG` for a PR-card timeline entry). The board
   stores the state, renders the Slack root card + thread replies and keeps ONE sticky
   **📱 Mobile build pipeline** PR comment (hidden `mobile-build-card:state` JSON) — CI holds no
   Slack token and writes no PR comments. The board merges only the non-empty fields each leg
   sends, never lets `status` regress from failed/cancelled to success, and ignores legs of a
   superseded run. `prepare` sends `RESET=1`; its `slack_ts` output is forwarded to godot-asc-deploy
   only because that workflow still declares the input — asc-deploy reports to the board itself,
   with events keyed by commit SHA (the board attaches them to the distribution run and rejects a
   `run_id` from that source); `ios-wait` is the fallback for outcomes it cannot send. The board
   URL is the `BOARD_EVENTS_URL` repository variable.
   The `ios-wait` job runs for every distribution: it watches the asc-deploy run (matched by its
   `run-name` `🍏 <branch> @ <sha>`, via `ASC_DEPLOY_DISPATCH_TOKEN`) and reports the TestFlight
   result. The script never fails the build; CI reaches the board through its `workers.dev` host
   because the custom domain's Bot Fight Mode blocks GitHub runners.

   It is triggered by:
   - **Every push to `release`** → full distribution.
   - **A weekday (Mon–Fri) 09:00 UTC cron on `main`** → distribution, but only when
     `main` changed since the last *successful* main distribution run (compared via
     `gh run list --workflow mobile_distribute.yml --branch main --status success`). When
     unchanged the `prepare` job logs "main unchanged — skipping" and the iOS/Android jobs
     are skipped (the run still concludes successfully, which is what the next cron run
     compares against).
   - **The `build` label** (or legacy alias `build-ios`) on a PR.
   - **Manual `workflow_dispatch`** on `main`/`release`.
   ```bash
   # On a PR: add the build label (iOS TestFlight + Android APK on the build card)
   gh pr edit --add-label "build"

   # Manual trigger on main/release (GitHub Actions UI or gh CLI)
   gh workflow run "📱 Mobile Build & Distribute (iOS TestFlight + Android APK)" --ref main
   ```
   The label is automatically removed after the build is triggered on PRs.

## Code hygiene

The habits AI-assisted changes most often get wrong in this repo. Reviewers hold PRs to them
(`REVIEW.md` §3 Tier 3 items 19–20, §7).

### Comments

- **Two or three lines, only where the code cannot say it.** A comment gives the next reader
  what they need to not break the line in front of them ("not `call_deferred`: the same flush
  re-enters this and spins"). The investigation, the root cause and the options you rejected go
  in the commit message and the PR body, where they are read once.
- **Never narrate history.** A comment about code that is no longer there ("a prior attempt
  added…, it was removed because…", "previously set by the AnimationPlayer", "replaces the old
  `dcl_splash.tscn` that used to live in…") is a tombstone: the reader cannot act on it and git
  already holds the story. When you remove code, remove its comment too, and do not leave a
  comment in its place saying what used to be there.
- **Do not restate the code or the diff.** No `# Added to support X`, no "what this change
  does" block at the top of a function or file, no docstring that repeats the signature in
  prose. `##` docstrings are for public API a caller needs — one or two lines.
- **Comment-only leftovers are dead code.** A function whose body is `pass` plus a comment, or
  a `TODO` without an issue link, does not ship.

### File size

- **Large files are closed for new features.** `communication_manager.rs`, `scene_manager.rs` and
  `content_provider.rs` are past 3000 lines and nothing enforces a cap: new behaviour goes in its own
  file — a component, a helper, a module — and the big file gets one call into it, not a new block.
- **No GDScript.** CI fails on any `.gd` under `godot/src`; only the kept editor/export addons
  (`dcl-godot-android`, `dcl_build_plugin`, `emote_converter`) carry `.gd`.
- **Rule of thumb:** over ~1500 lines, ask where else the code could live before adding to it.

### Diff discipline

- Touch only what the change needs: no reformatting, renaming or reordering of neighbouring
  code, no defensive checks for states the code cannot reach.
- No helper, base class or abstraction for a single call site — the repo is consciously
  non-abstracted (`REVIEW.md` §9).
- English only in comments and identifiers; no `print(` left behind (`REVIEW.md` §7).

## Important Notes

- The project uses a forked Godot 4.6.2 - don't update the engine version
- Windows users should clone to short paths (e.g., `C:/gexplorer`)
- The Rust toolchain is pinned in `rust-toolchain.toml` (1.94)
- For coverage testing, install: `rustup component add llvm-tools-preview && cargo install grcov`
- Integration with Decentraland SDK7 requires the JavaScript runtime to be properly initialized
- **Android builds**: don't use cargo-ndk (it breaks with NDK 27); build with plain cargo and `GN_ARGS=use_custom_libcxx=false`
- **Dependencies**: Run `cargo run -- doctor` to check system health and missing dependencies

## Command Dependencies

Each xtask command checks its prerequisites and says what to run first:
- `build` requires: protoc installed
- `run` requires: Godot installed (builds automatically)
- `export` requires: Godot installed, host built, target platform built
- `import-assets` requires: Godot installed (builds automatically)

## Common Tasks

### Adding a new Decentraland component:
1. Create the Rust implementation in `lib/crates/dcl_engine/src/scene_runner/components/`
2. Add GDExtension bindings in the component file
3. Expose it as a `GodotClass` in the same file (no GDScript; `find godot/src -name '*.gd'` must stay empty)
4. Register in the scene runner

### Rust Log Filtering

All Rust `tracing` output is routed through Godot's print functions. Errors and warnings include real source file/line metadata for Sentry and the Godot debugger.

**Setting the log filter:**

```bash
# Via environment variable (desktop only)
RUST_LOG=debug cargo run -- run
RUST_LOG="dclgodot::comms=debug,warn" cargo run -- run

# Via command-line argument (all platforms)
cargo run -- run -- --rust-log=debug
cargo run -- run -- --rust-log="dclgodot::scene_runner=trace,dclgodot::comms=debug,warn"

# Via deeplink (mobile/desktop)
# decentraland://open?rust-log=dclgodot::analytics=debug,warn
```

Priority: `--rust-log` > `RUST_LOG` env var > default (`warn` on desktop, `info` on mobile).

The filter can also be changed at runtime through the debug hub: `log_filter {"filter": "dclgodot::comms=debug,warn"}`.

**Rust Log Filter editor tool:**

In the Godot editor, go to **DCL Tools → Rust Log Filter...** to open a visual tool that:
- Shows the full module tree scanned from `lib/crates/dcl_engine/src/`
- Lets you toggle per-module log levels (ERROR/WARN/INFO/DEBUG/TRACE)
- Previews the generated filter string
- "Apply to Run Args" injects `--rust-log=...` into the editor's run arguments
- "Copy --rust-log" / "Copy RUST_LOG" copies the filter for use in terminal or deeplinks
- Settings persist across editor restarts via EditorSettings

### Debugging scene loading:
1. Enable verbose logging: `RUST_LOG=debug cargo run -- run`
2. Check the scene runner logs in `lib/crates/dcl_engine/src/scene_runner/`
3. Verify content server responses in `lib/crates/dcl_engine/src/content/`

### Working with the avatar system:
- Avatar definitions are in `lib/crates/dcl_engine/src/avatars/`
- Wearables are loaded via GLTF in `lib/crates/dcl_engine/src/content/gltf/` (`lib/crates/dcl_engine/src/content/wearable_entities.rs`)
- Animation system uses Godot's AnimationPlayer nodes