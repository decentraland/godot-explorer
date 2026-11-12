# iOS track — 2.x vs 1.x on iPhone 16e (issue #3206)

Device: `00008140-000529060C82801C`, iPhone 16e (`iPhone17,5`), iOS 26.6.2.
Scene: Genesis Plaza, launched with `decentraland://open?position=0,0`.
Date of the sessions below: 2026-11-12, ~10:22–10:29 local device time.

## Installed app version per half

| Half | Installed | Version string (verbatim, `xcrun devicectl device info apps`) |
|---|---|---|
| 2.x | yes | `Decentraland  org.decentraland.godotexplorer  2.0.0  1` |
| 1.x (`/Users/lordmanuel/Projects/decentraland/ios-export/Decentraland.ipa`) | **no — install failed** | (not installed) |

The in-app build string shown by the engine HUD for the 2.x half is
`2.0.0.2723-e3b7aa1-staging` (devicectl only reports `2.0.0` / build `1`).

## Did each half reach Genesis Plaza?

**2.x — yes.** After launch, the accessibility tree showed the HUD with the version and FPS:

```
StaticText   | 2.0.0.2723-e3b7aa1-staging - 56 FPS
```

and the screenshot taken just before the first session
(`gp-hi-ios/2x-ios-gp-noon-r4/screen.png`, and the same view in `2x-ios-gp-noon-r1/screen.png`)
shows the Genesis Plaza scene (DCL STORE facade, Bloom Garden sign) with the HUD line
`2.0.0.2723-e3b7aa1-staging - 40 FPS` / `- 51 FPS` overlaid at the bottom.

**1.x — no, never launched.** The IPA never installed, so no FPS line and no session exist for it.

## Samples (2.x, HUD-sampled, 1 Hz, 20 samples per session)

`SETTLE=10 SAMPLES=20 scripts/bench/2x-vs-1x/ios_hud_sample.sh <label>`; the app was re-checked with
`iphone ui` between sessions and was still foreground and in-world (FPS line present) every time.
At least 20 s of idle separation was kept between consecutive sessions.

| Session label | median fps | min | max | n samples |
|---|---|---|---|---|
| 2x-ios-gp-noon-r1 | 40.0 | 38 | 43 | 20 |
| 2x-ios-gp-noon-r2 | 35.5 | 30 | 40 | 20 |
| 2x-ios-gp-noon-r3 | 37.5 | 29 | 39 | 20 |
| 2x-ios-gp-noon-r4 | 39.0 | 37 | 47 | 20 |
| 2x-ios-gp-noon-r5 | 36.0 | 33 | 41 | 20 |
| 2x-ios-gp-noon-r6 | 36.0 | 34 | 37 | 20 |
| 2x-ios-gp-noon-r7 | 36.0 | 33 | 37 | 20 |

Pooled over the 140 samples: median 37.0, min 29, max 47.
Median of the seven session medians: 36.0.

| 1.x session label | median fps | min | max | n samples |
|---|---|---|---|---|
| 1x-ios-gp-noon-r1 … r6 | missing | missing | missing | 0 (app never installed) |

`gp-hi-ios/2x-ios-r1/` (20 samples, median 38.0, min 37, max 40, recorded 2026-11-11 with the same HUD
method but a 25 s settle and a `meta.json`) predates this run and is not part of the seven sessions above.

## Failures and what was done

1. **Void recordings carried over from the earlier attempt** — `2x-ios-gp-noon-r4`, `-r5` had a 0-byte
   `samples.txt` and `-r6` had none; their `screen.png` were ~176 KB blank captures. Cause given by the
   previous attempt: the app had dropped to the background. No error text was produced.
   Action: did not reuse them. Verified the app was backgrounded (`iphone ui` showing Tailscale), relaunched
   with `devicectl … process launch --terminate-existing --payload-url "decentraland://open?position=0,0"`,
   waited for a `FPS` line, then re-recorded r4, r5 and r6; added r7 as an extra full session. All seven now
   have 20 non-empty samples and ~3.7–4.0 MB real screenshots.

2. **1.x install failed — provisioning profile expired.** Exact output of
   `xcrun devicectl device install app --device 00008140-000529060C82801C /Users/lordmanuel/Projects/decentraland/ios-export/Decentraland.ipa`:

   ```
   ERROR: Failed to install the app on the device. (com.apple.dt.CoreDeviceError error 3002 (0xBBA))
          NSURL = file:///Users/lordmanuel/Projects/decentraland/ios-export/Decentraland.ipa
          ----------------------------------------
              Unable to Install "Decentraland" (IXUserPresentableErrorDomain error 14 (0x0E))
              NSLocalizedRecoverySuggestion = Failed to install embedded profile for org.decentraland.godotexplorer : 0xe8008011 (This provisioning profile has expired.)
              NSLocalizedFailureReason = This app cannot be installed because its integrity could not be verified.
          ----------------------------------------
                  Failed to install embedded profile for org.decentraland.godotexplorer : 0xe8008011 (This provisioning profile has expired.) (MIInstallerErrorDomain error 13 (0x0D))
                  LegacyErrorString = ApplicationVerificationFailed
                  SourceFileLine = 325
                  LibMISErrorNumber = -402620399
                  FunctionName = -[MIInstallableBundle _installEmbeddedProfilesWithError:]
   ```

   Diagnosis from the IPA itself: `CFBundleShortVersionString = 1.1`, `CFBundleVersion = 27`,
   `MinimumOSVersion = 16.0` (so the OS version is not the blocker), profile
   `iOS Team Provisioning Profile: org.decentraland.godotexplorer`, team `8T73XM973P`,
   `ExpirationDate = Thu Jul 23 10:21:11 -03 2026` — expired ~4 months before this run. The device UDID
   `00008140-000529060C82801C` *is* listed in the profile's `ProvisionedDevices`, so expiry alone blocks it.
   Action: retried once (2 attempts total, the cap), got the identical error, and stopped the 1.x half. The
   1.x comparisons are therefore missing, not estimated.

3. **2.x restore after the failed 1.x attempts.** The failed install had sent the app to the background
   (`iphone ui` showed the home screen). The 2.x app was still installed and unmodified
   (`… 2.0.0  1`), so nothing was lost; it was relaunched and confirmed back in-world
   (`StaticText | 2.0.0.2723-e3b7aa1-staging - 47 FPS`).

   Restoring 2.x from a local artifact was **not possible**: the only local 2.x iOS artifact,
   `.bench-dl/ios-2x-export.tar.gz` (658 entries, inspected with `tar -tzf`), is a *source project export*
   (`decentraland-godot-client.xcodeproj/`, `decentraland-godot-client.xcframework/`, `MoltenVK.xcframework/`,
   `decentraland-godot-client.pck`, `rn/`, `PrivacyInfo.xcprivacy`) — it contains **no `.app`, no `.ipa` and no
   `.xcarchive`**, so `devicectl device install app` has nothing to install from it. No restore was attempted
   or guessed.

## Limits

- iOS exposes no per-frame present timeline to third parties (no Perfetto/FrameTimeline equivalent), so the
  metric here is the engine's own **1 Hz FPS counter read from the accessibility tree** — an average with no
  p99, and 20 samples per session do not resolve frame-time tails.
- The only 1.x build available locally is **1.1 (build 27)**, not the 1.16.1 used on Android, and it does not
  install at all (expired provisioning profile). A like-for-like 1.x iOS comparison does not exist in this run.
- The 1.x half has no numbers. Every 1.x cell is `missing`, not estimated.
