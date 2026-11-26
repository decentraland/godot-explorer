# Phase 8: after the release

Team, after 2.0.0 ships. Nothing here is needed for the full-migration gate or the release.

## Exit criteria

- [ ] `cargo run -- export --target linux|macos|windows` exports the emulator, published under its
      own name (D-28).
- [x] No Godot plugin left: video moved into the host before the release (D-41, 24 Nov 2026), so 8.2 is not needed.

## Work items

- [ ] **8.1 Desktop build is the emulator**. Export presets, `README.md`, `docs/MOBILE_PREVIEW.md`,
  `docs/scene-architecture.md` updated; `--emulate-ios/android` reduced to window sizes; the
  emulator published under its own name (D-28).
- [x] **8.2 Video into the host** (only if 2.0.0 shipped the video-only plugin, D-39 and D-41). Done before the release. The
  host implements `VideoBackend`, the engine's video component switches to it, and `plugins/`, its
  build scripts and committed binaries are deleted.

## Depends on

The 2.0.0 release. Closes the project.
