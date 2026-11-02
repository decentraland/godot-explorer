# Phase 8: after the release

Team, after 2.0.0 ships. Nothing here is needed for the full-migration gate or the release.

## Exit criteria

- [ ] `cargo run -- export --target linux|macos|windows` exports the emulator, published under its
      own name (D-28).
- [ ] No Godot plugin left, if 2.0.0 shipped the video-only plugin under the D-39 fallback (8.2).

## Work items

- [ ] **8.1 Desktop build is the emulator**. Export presets, `README.md`, `docs/MOBILE_PREVIEW.md`,
  `docs/scene-architecture.md` updated; `--emulate-ios/android` reduced to window sizes; the
  emulator published under its own name (D-28).
- [ ] **8.2 Video into the host** (only if 2.0.0 shipped the video-only plugin, D-39 and D-41). The
  host implements `VideoBackend`, the engine's video component switches to it, and `plugins/`, its
  build scripts and committed binaries are deleted.

## Depends on

The 2.0.0 release. Closes the project.
