# Prompt: Continue iOS In-App Purchase development

## How to use this prompt

Paste the entire content below into a fresh Claude Code session inside the `godot-explorer` repo. The session will pick up exactly where development left off, with full context — no backtracking, no re-discovering decisions.

If the user is also unblocked on the Paid Apps Agreement (see sibling prompt `iap-paid-apps-agreement-escalation.md`), they should mention that at the top of their first message so the assistant skips the local-mock path and goes straight to sandbox testing.

---

## Prompt content (copy from here down)

You are resuming work on the **iOS In-App Purchases (IAPs) feature** for the Decentraland Godot Explorer (`godot-explorer` repo). This is a continuation of a previous session. Read this entire prompt before doing anything. Do NOT restart the design — the architecture is decided and partially implemented.

### Project at a glance

- **Repo**: `/Users/leandro/github/godot-explorer` (Godot 4.5.1 + Rust + JS V8 client for Decentraland)
- **iOS bundle ID**: `org.decentraland.godotexplorer`
- **Apple team**: Decentraland Foundation (`8T73XM973P`)
- **Working branch**: `feat/swift-infra-base` (created off `main`, contains both the Swift infra and the in-progress IAP work — has not been split into separate PRs yet)
- **User**: Leandro Mendoza (`leandro@dclregenesislabs.xyz`), engineer at Decentraland Foundation. Spanish-speaker, prefers concise responses, writes Swift/Rust/GDScript fluently.

### What was decided and why

**Decision 1: Use Apple's StoreKit 2 (not StoreKit 1).** StoreKit 2 has async/await, JWS-signed transactions verifiable client-side, and is the modern API. Requires iOS 15+ which is fine since the project already targets iOS 17.

**Decision 2: Implement the native bridge as a Swift GDExtension, NOT as additions to the existing Objective-C++ plugin** (`plugins/dcl-godot-ios/`). Reason: a Swift GDExtension scaffold (`plugins/dcl-swift-lib/`) was rescued from a closed-but-working branch (`feat/wallet-connect-integration`) — it uses `SwiftGodot` (Miguel de Icaza's binding generator) + Swift Package Manager. This Swift infra is now in `main`-ish state, and StoreKit 2 (Swift-only API) plugs in naturally. Adding StoreKit 2 to the ObjC++ plugin would have required mixing Swift into a SCons-driven static lib build, which is fragile.

**Decision 3: Keep server-side receipt validation as a stub for now.** The current `IapManager._validate_with_backend()` returns true unconditionally and is marked with a TODO. Reason: the user has no backend yet, and forcing a real backend would block all client work. Critical safety net: **the code never calls `finish_transaction` until validation succeeds**. Without `finish()`, StoreKit re-delivers the transaction at every app launch, so a crash or fake validation never silently swallows a purchase — the user can replay it. This is the correct behavior to keep until a real backend exists.

**Decision 4: Snake_case method names in GDScript via `@Callable(autoSnakeCase: true)`.** SwiftGodot's `@Callable` does NOT auto-convert camelCase to snake_case by default — only `@Signal` does. So `func canMakePayments()` in Swift must be marked `@Callable(autoSnakeCase: true)` to be reachable as `can_make_payments()` from GDScript. All public StoreKit methods follow this pattern.

**Decision 5: Logs flow through both `NSLog` (iOS Console.app) AND `GD.print` (Godot stdout).** A helper `gdLog()` in `StoreKitManager.swift` does both. Reason: `cargo run -- run --target ios` shows Godot stdout but not NSLog, so without the dual-routing the user couldn't see Swift-side logs in their normal workflow.

**Decision 6: One placeholder product (`credits_10`) hardcoded in `PRODUCT_IDS`.** This is the only IAP currently registered in App Store Connect. More packs are intended later — when added in ASC, just append the IDs and credits-per-pack to `PRODUCT_IDS` and `_CREDITS_BY_PRODUCT` in `iap_manager.gd`.

### What's already built and working

#### Swift GDExtension infrastructure (`plugins/dcl-swift-lib/`)

- `Package.swift` declares iOS 17+ target, only depends on `SwiftGodot` (the wallet-connect deps were stripped). Builds an `xcframework`.
- `Makefile` + `build_ios_swift.sh` produce `bin/DclSwiftLib.xcframework` and copy the `ios-arm64` slice to `godot/ios/dcl-swift-lib/DclSwiftLib.framework/`.
- `Sources/DclSwiftLib/DclSwiftLib.swift`: entry point. Registers extension via `#initSwiftExtension(cdecl: "dcl_swift_lib_init", types: [DclSwiftLib.self, DclStoreKit.self])`. The unique entry symbol is critical to avoid clashes with other GDExtensions (Sentry's `gdextension_init`, etc.).
- `DclSwiftLib` class is a smoke-test class with `ping()` returning `"ok"` and `version()` returning `"0.1.0"` — confirmed working on device (Leandro saw `[DclSwiftLib] ping() -> ok | version() -> 0.1.0` in Godot logs).

#### StoreKit 2 wrapper (`plugins/dcl-swift-lib/Sources/DclSwiftLib/StoreKitManager.swift`)

`@Godot class DclStoreKit: RefCounted` exposes:

| Swift method | GDScript name | Purpose |
|---|---|---|
| `canMakePayments()` | `can_make_payments()` | Sync, returns Bool. Wraps `AppStore.canMakePayments`. |
| `startListening()` | `start_listening()` | Idempotent. Spawns a `Task` that observes `Transaction.updates` for re-delivered / async-arrived transactions. |
| `loadProducts(productIds:)` | `load_products(product_ids)` | Async. Calls `Product.products(for:)` and emits `productsLoaded` (JSON array) or `productsLoadFailed` (string). |
| `purchase(productId:)` | `purchase(product_id)` | Async. Calls `product.purchase()`, switches on the result, emits one of: `purchaseCompleted` (JSON tx + JWS), `purchaseFailed`, `purchaseCancelled`, `purchasePending`. |
| `finishTransaction(transactionId:)` | `finish_transaction(transaction_id)` | Finds the unfinished tx by ID and calls `tx.finish()`. **Only call after validation succeeds.** |

Signals (auto-snake_case'd by SwiftGodot's `@Signal` macro):
- `productsLoaded(json: String)` — JSON array of `{id, displayName, description, price, displayPrice, type}`
- `productsLoadFailed(error: String)`
- `purchaseCompleted(json: String)` — JSON `{id, originalId, productId, purchaseDate, jwsRepresentation}`
- `purchaseFailed(productId: String, error: String)`
- `purchaseCancelled(productId: String)`
- `purchasePending(productId: String)`
- `transactionUpdated(json: String)` — same shape as `purchaseCompleted`, fired from `Transaction.updates`

Diagnostic logging includes bundle ID, sandbox detection (best-effort via `appStoreReceiptURL.lastPathComponent == "sandboxReceipt"`), and a "likely causes" message when 0 products return.

#### GDScript autoload (`godot/src/iap/iap_manager.gd`, registered as `Iap` in `project.godot`)

- On `_ready()`, instantiates `DclStoreKit` via `ClassDB.instantiate("DclStoreKit")` (returns null on non-iOS — handled gracefully with `is_available()`).
- Connects all signals.
- Calls `start_listening()` and `load_products(PRODUCT_IDS)` on boot.
- `_handle_verified_transaction(tx)` is the central post-purchase path: it calls `_validate_with_backend(tx)` (stub returning true), and ONLY if true grants credits locally and calls `finish_transaction(tx_id)`. If validation fails, **the tx is NOT finished**, so StoreKit will re-deliver next launch.
- Public API: `is_available() -> bool`, `get_products() -> Array`, `purchase(product_id: String)`. Public signals: `products_ready`, `products_load_failed`, `purchase_completed(product_id, credits)`, `purchase_failed`, `purchase_cancelled`, `purchase_pending`.

#### Build / signing config (`godot/export_presets.cfg`)

- `application/app_store_team_id="8T73XM973P"` (Decentraland Foundation)
- `application/code_sign_identity_debug="Apple Development"` (hardened from empty)
- `application/code_sign_identity_release="Apple Distribution"` (hardened from "iPhone Developer")
- `application/bundle_identifier="org.decentraland.godotexplorer"`
- IAP capability is **not** in this file because StoreKit IAP needs no entitlement and no Info.plist key. The capability lives at developer.apple.com/identifiers — confirmed enabled on the App ID (already CHECKED at the team level, cannot be unchecked because ASC is using it).

#### Verified working on device

The bridge has been validated end-to-end on a real iPhone. The latest test (Leandro's last log) showed:

```
[IAP] starting StoreKit listener; can_make_payments=true
[DclStoreKit] startListening: subscribing to Transaction.updates
[DclStoreKit] loadProducts: requesting ["credits_10"]
[DclStoreKit] bundle: org.decentraland.godotexplorer sandbox: true
[DclStoreKit] loadProducts: got 0 products []
[IAP] products_loaded: 0 products
```

The bridge works perfectly. Bundle ID matches ASC. Sandbox is active. The 0-product result is **NOT a code bug** — see "Open blocker" below.

### Open blocker (read this carefully)

**The Paid Apps Agreement at App Store Connect is in status "New" (i.e. not signed) for Decentraland Foundation.** Without it, no IAP product loads on any device — sandbox or production. This is gated at the team level by Apple. It's not a code issue, not an Apple delay, not propagation: products are simply not served until an Account Holder / Admin completes the agreement (banking info, W-8BEN-E tax form, contact assignments).

The user has been given an escalation prompt (sibling file `iap-paid-apps-agreement-escalation.md`) to coordinate with Decentraland Foundation's finance/legal/leadership to sign it. **Until that's done, real sandbox testing is impossible.**

**Workaround for development**: use a `.storekit` Configuration File. Xcode intercepts StoreKit calls and serves products from this local file, bypassing ASC entirely. Works without Paid Apps Agreement, without sandbox tester, even works in Simulator. When the user runs the app from Xcode (not from `cargo run`), Xcode reads the scheme's StoreKit Configuration setting and applies it. This is the recommended path for keeping development moving.

### Repo layout you should know about

```
plugins/dcl-swift-lib/                              ← Swift GDExtension SPM package
  Package.swift
  Makefile                                           ← `make xcframework` builds + installs
  build_ios_swift.sh                                 ← convenience wrapper
  Sources/DclSwiftLib/
    DclSwiftLib.swift                                ← entry point + smoke test
    StoreKitManager.swift                            ← DclStoreKit StoreKit 2 wrapper

godot/
  dcl_swift_lib.gdextension                          ← entry_symbol = "dcl_swift_lib_init"
  ios/.gitignore                                     ← excludes built framework
  ios/dcl-swift-lib/DclSwiftLib.framework/           ← gitignored, rebuilt by Makefile
  src/iap/iap_manager.gd                             ← GDScript autoload (registered as `Iap`)
  src/global.gd                                      ← contains a smoke-test call to DclSwiftLib (lines ~210)
  project.godot                                      ← `Iap="*res://src/iap/iap_manager.gd"` autoload
  export_presets.cfg                                 ← iOS preset hardened for IAP

plugins/build_swift_lib.sh                           ← top-level build wrapper

.claude/prompts/                                     ← THIS prompt + escalation prompt
```

The Godot custom fork (`plugins/dcl-godot-ios/godot/`) is a submodule and was NOT modified for this work — IAP doesn't need engine changes.

### How to rebuild and verify

```bash
# 1. Rebuild the Swift xcframework after any .swift change
cd plugins/dcl-swift-lib
./build_ios_swift.sh

# Verifies: BUILD SUCCEEDED, framework installed at godot/ios/dcl-swift-lib/DclSwiftLib.framework/
# Sanity check the entry symbol is exported:
nm -gU /Users/leandro/github/godot-explorer/godot/ios/dcl-swift-lib/DclSwiftLib.framework/DclSwiftLib | grep "_dcl_swift_lib_init"

# 2. Validate GDScript (catches autoload registration errors etc.)
cd /Users/leandro/github/godot-explorer
cargo run -- check-gdscript

# 3. Build & deploy to iPhone
cargo run -- run --target ios

# 4. Watch logs in stdout (works because of dual NSLog/GD.print routing)
# Look for [DclStoreKit] and [IAP] prefixes.
```

If you change `iap_manager.gd`: also run `gdformat godot/` and `gdlint godot/` (the project enforces these).

### What to work on next (priority order)

#### Path A: User has signed Paid Apps Agreement and sandbox is now working

1. **Build a credit purchase UI**. There's currently no way for the user to trigger `Iap.purchase("credits_10")` from inside the app. Need a settings screen entry or a dedicated "Get Credits" view. Reference `godot/src/ui/components/settings/` for the existing settings architecture. The screen should: (a) display loaded products (name, price, description), (b) on tap, call `Iap.purchase()`, (c) listen to `purchase_completed` / `purchase_failed` signals to show success/error state, (d) display the user's current credit balance (currently nothing persists, see #2).

2. **Local credit balance persistence**. Right now `_grant_credits_locally()` just prints. Pick a store: `UserDefaults` via the existing iOS plugin, or a small SQLite via the existing `NotificationDatabase` infra in `plugins/dcl-godot-ios/plugins/dcl_godot_ios/NotificationDatabase.{h,mm}`, or a simple `ConfigFile` JSON. Whatever you pick, expose `Iap.get_balance() -> int` for UI consumers. **Important**: persist atomically with `finish_transaction`, so a crash between grant and finish doesn't double-credit on next launch.

3. **Add more credit packs to ASC and the code**. Currently only `credits_10`. Once the agreement is signed, the user (or whoever owns ASC) should add `credits_50`, `credits_100`, `credits_500`, etc. Append to `PRODUCT_IDS` and `_CREDITS_BY_PRODUCT` in `iap_manager.gd`.

4. **Plan the backend validation handoff**. The `_validate_with_backend()` stub needs to be replaced with a real HTTP call to a server endpoint that:
   - Receives `tx.jwsRepresentation`
   - Verifies the JWS signature against Apple's public key (Apple publishes a JWKS at https://appleid.apple.com/auth/keys for some flows; for StoreKit specifically use the App Store Server API or verify locally with the TR-ECDSA-P-256 public keys Apple ships with the SDK)
   - Records the credit grant in the canonical user account (matched by Decentraland wallet address or whatever the canonical user identifier is)
   - Returns OK only after the credit is durably persisted server-side
   - Implements idempotency keyed on `originalId` (Apple may re-send the same tx multiple times)
   - **Until the backend exists, do NOT ship IAPs to production. The current stub is INSECURE — a malicious client can fake purchases.**

#### Path B: Paid Apps Agreement still pending, develop against StoreKit Configuration File

1. **Create `Configuration.storekit`** in the repo (suggest location: `godot/ios/Configuration.storekit`). Content: define `credits_10` as a Consumable product with display name, description, and a USD 0.99 price. Apple's official template:

   ```json
   {
     "identifier" : "...",
     "products" : [
       {
         "displayPrice" : "0.99",
         "familyShareable" : false,
         "internalID" : "credits_10",
         "localizations" : [
           { "description" : "Get 10 credits to spend in Decentraland.", "displayName" : "10 Credits", "locale" : "en_US" }
         ],
         "productID" : "credits_10",
         "referenceName" : "Credits 10",
         "type" : "Consumable"
       }
     ],
     ...
   }
   ```

2. **Wire it into Xcode scheme**: after `cargo run -- export --target ios`, open the generated `.xcodeproj`, Product → Scheme → Edit Scheme → Run → Options → StoreKit Configuration → select `Configuration.storekit`. (Note: this scheme setting is NOT preserved across re-exports. Document the manual step OR investigate scripting it via `xcodebuild -configuration` or a post-export script.)

3. Then proceed with Path A items in order — UI, persistence, etc. — using the local mock as the data source.

#### Path C: Polish and PR

The current branch `feat/swift-infra-base` mixes two logical changes:

(a) Swift GDExtension scaffold (the infra: `dcl-swift-lib/` with smoke-test class, `.gdextension`, build scripts)
(b) IAP feature on top of it (`StoreKitManager.swift`, `iap_manager.gd`, autoload registration)

Leandro mentioned eventually wanting to split (a) out as a separate PR for cleaner review. If asked to do this, the natural split is the latest commit `6e3e313c setup` (which contains the infra) vs the unstaged changes (which contain the IAP feature). Use `git log --stat` to confirm before splitting.

### Things to be careful about

- **Never call `finish_transaction` before backend validation succeeds.** This is the safety net for the missing backend. The current code respects this — preserve the invariant.
- **`@Callable(autoSnakeCase: true)`** is required on every public method of `DclStoreKit`. Forgetting this manifests as runtime errors like `"Invalid call. Nonexistent function 'can_make_payments' in base 'DclStoreKit'"`.
- **`Product.products(for:)` returns `[]` for unknown IDs without throwing.** If `loadProducts` returns 0 products, that's not an exception — diagnose by checking sandbox state, bundle ID, and Paid Apps Agreement status (in that order, the agreement is the most common cause).
- **The framework is gitignored** at `godot/ios/.gitignore` (entry: `dcl-swift-lib/`). Don't accidentally commit the 20MB binary. `cargo run -- run --target ios` requires the framework to be built first; if it's missing, run `plugins/build_swift_lib.sh`.
- **iPhone IAP review screenshots are dimension-strict.** If asked to upload one, target exactly `1242x2208` (iPhone 6+/7+/8+ Plus resolution) — Apple is more permissive with this size than with newer iPhone resolutions.
- **Read `MEMORY.md`** at session start. Notable entries: gdformat/gdlint must run on entire `godot/` folder, not individual files; the user is on Godot 4.5.1 (the CLAUDE.md was updated mid-session).

### Communication style

The user prefers Spanish, concise responses, and skipping over re-explanations of things already in this prompt. They appreciate when you proactively use Chrome DevTools MCP to verify things in App Store Connect / Apple Developer portal rather than guessing. They're responsive in real-time and will tell you if you're going off track — trust their corrections.

If they ask for something the prompt covered (like "what's the next step"), don't repeat this prompt — just give them the answer scoped to what they asked.

### Begin by

Reading `MEMORY.md`, then `git status` and `git log --oneline -5`, then asking the user one specific question: **"¿Salió la firma del Paid Apps Agreement, o seguimos con `.storekit` Configuration File para dev?"** That answer determines whether to follow Path A or Path B.

Do NOT begin coding until you get that answer.
