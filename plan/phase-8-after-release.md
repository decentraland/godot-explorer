# Phase 8: after the release

Team, after 2.0.0 ships. Nothing here is needed for the full-migration gate or the release.

## Exit criteria

- [ ] `cargo run -- export --target linux|macos|windows` exports the emulator, published under its
      own name (D-28).
- [ ] The host calls RN libraries, not the Godot-era Kotlin/Swift plugin code, for wallets, push,
      StoreKit, share and calendar (D-21, D-25).

## Work items

- [ ] **8.1 Desktop build is the emulator**. Export presets, `README.md`, `docs/MOBILE_PREVIEW.md`,
  `docs/scene-architecture.md` updated; `--emulate-ios/android` reduced to window sizes; the
  emulator published under its own name (D-28).
- [ ] **8.2 RN libraries for the host's plugin code**. Replace the Kotlin/Swift
  plugin code the host calls (wallets, push, StoreKit, share, calendar) with RN libraries (D-21, D-25).
  The same IAP library that serves Android in 7.4 can take over iOS StoreKit.

## Depends on

The 2.0.0 release. Closes the project.
