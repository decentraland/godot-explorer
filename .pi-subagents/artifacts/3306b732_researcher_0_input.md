# Task for researcher

Research these two technical questions with web sources:

1. In Godot Engine 4.x (specifically 4.6, custom fork similar to vanilla), is `Image.compress(CompressMode.ETC2)` available at RUNTIME in export template builds (release templates for Android/iOS), or is ETC2/ASTC compression only compiled into editor (TOOLS_ENABLED) builds? Check Godot source (modules/etcpak, core/io/image.cpp register functions like _image_compress_etc2_func) and GitHub issues. Also: what does `PortableCompressedTexture2D.create_from_image(image, CompressionMode.ETC2)` do in an export template if runtime ETC2 compression is unavailable — does it fail, produce an empty texture, or store uncompressed data? Any known issues where textures appear WHITE on mobile when using PortableCompressedTexture2D or runtime-compressed ETC2 textures on the Compatibility/GLES3 renderer?

2. reqwest 0.11 (Rust) built with default-features=false + features=["rustls-tls"] (which maps to rustls-tls-native-roots using rustls-native-certs): what happens on Android and iOS regarding root certificate loading? Does rustls-native-certs return an empty root store on Android (no system CA access), causing ALL https requests to fail with certificate errors? Check rustls-native-certs platform support. Would switching to webpki-roots (rustls-tls-webpki-roots) fix Android/iOS TLS?

Report concise findings with source URLs for each.

---
Update progress at: /Users/lordmanuel/Projects/decentraland/godot-explorer/fix-arweave-images-mobile-1766/.pi-subagents/artifacts/progress/3306b732/progress.md

---
**Output:**
Write your findings to exactly this path: /Users/lordmanuel/Projects/decentraland/godot-explorer/fix-arweave-images-mobile-1766/.pi-subagents/artifacts/outputs/3306b732/parallel-0/0-researcher/research.md
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