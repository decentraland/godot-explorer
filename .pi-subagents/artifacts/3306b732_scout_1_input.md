# Task for scout

Explore the repo at /Users/lordmanuel/Projects/decentraland/godot-explorer/fix-arweave-images-mobile-1766 (read-only). Context: bug where Arweave-hosted NFT images (NftShape frames, loaded via OpenSea proxy -> image_url -> Global.content_provider.fetch_texture_by_url -> Rust load_image_texture in lib/src/content/texture.rs) render WHITE on Android/iOS but fine on desktop. White = either texture null (download/promise failed) or GPU blank.

Find and report:
1. ALL platform-specific branches (std::env::consts::OS == "ios"/"android", #[cfg(target_os)], OS.has_feature in GDScript) in texture/image/http/download code paths under lib/src/content/, lib/src/godot_classes/, godot/src/logic/ and godot/src/decentraland_components/.
2. What DclUrls.open_sea_proxy() returns and whether it differs per platform (find godot/src/logic/urls or similar).
3. What the NftShape loading material looks like (godot/src/decentraland_components/nft_frame_style_loader.gd, any .tscn/.tres with loading material) — is it white/unshaded?
4. How Global.http_requester works (godot/src/logic/http_requester*) — does it use Godot HTTPRequest and does it have any mobile-specific handling or known TLS issues?
5. Run `git log --oneline -15 -- lib/src/content/texture.rs` and `git log --oneline --grep=ETC2 -10` and `git log --oneline --grep=mobile -i -10` to see recent related changes (report output verbatim).
6. Check godot/export_presets.cfg or project.godot for Android/iOS rendering driver (gles3 vs vulkan/mobile) and texture compression settings.

Report file paths + line numbers, concise.

---
Update progress at: /Users/lordmanuel/Projects/decentraland/godot-explorer/fix-arweave-images-mobile-1766/.pi-subagents/artifacts/progress/3306b732/progress.md

---
**Output:**
Write your findings to exactly this path: /Users/lordmanuel/Projects/decentraland/godot-explorer/fix-arweave-images-mobile-1766/.pi-subagents/artifacts/outputs/3306b732/parallel-0/1-scout/context.md
This path is authoritative for this run.
Ignore any other output filename or output path mentioned elsewhere, including output destinations in the base agent prompt, system prompt, or task instructions.

## Acceptance Contract
Acceptance level: attested
Completion is not accepted from prose alone. End with a structured acceptance report.

Criteria:
- criterion-1: Return concrete findings with file paths and severity when applicable

Required evidence: review-findings, residual-risks

Finish with a fenced JSON block tagged `acceptance-report` in this shape:
Use empty arrays when no items apply; array fields contain strings unless object entries are shown.
`criteriaSatisfied[].status` must be exactly one of: satisfied, not-satisfied, not-applicable.
`commandsRun[].result` must be exactly one of: passed, failed, not-run.
`manualNotes` and `notes` are optional strings; an empty string means no note and does not satisfy `manual-notes` evidence.
```acceptance-report
{
  "criteriaSatisfied": [
    {
      "id": "criterion-1",
      "status": "satisfied",
      "evidence": "specific proof"
    }
  ],
  "changedFiles": [
    "src/file.ts"
  ],
  "testsAddedOrUpdated": [
    "test/file.test.ts"
  ],
  "commandsRun": [
    {
      "command": "command",
      "result": "passed",
      "summary": "short result"
    }
  ],
  "validationOutput": [
    "validation output or concise summary"
  ],
  "residualRisks": [
    "none"
  ],
  "noStagedFiles": true,
  "diffSummary": "short description of the diff",
  "reviewFindings": [
    "blocker: file.ts:12 - issue found, or no blockers"
  ],
  "manualNotes": "anything else the parent should know"
}
```