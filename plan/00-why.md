# Why this plan

Why the client changes shape, what it buys and costs, and why the shell is React Native and not
Flutter or another UI stack. The numbers about today come from [01-today.md](01-today.md).

## The problem on main: nothing decides where code goes

A feature on 1.x can land in GDScript or in Rust, and no rule picks one. The result:

- **Logic in both languages.** Sign-in is a GDScript flow over Rust auth; equip rules, settings and
  notifications are GDScript; analytics is Rust called from GDScript.
- **Coupling by string.** Rust calls ~40 GDScript methods by name and reaches 4 hardcoded node paths;
  a rename breaks at runtime, not at build.
- **Pure Rust that cannot run alone.** 20–25k lines of Rust have no real Godot need, but every module
  reaches shared state through the `DclGlobal` node, so none of it runs or tests without Godot.
- **GDScript itself.** One of the biggest pain points: it is easy to make mistakes that cause crashes, and it is a language that
  does not scale (optional types, as in Python; anything reached by name is checked only when it runs).
  Rust and TypeScript check types at build time, and Rust rules out whole families of crashes.
- **GDScript is where it piles up.** 86,739 lines; the four largest files sit within 40 lines of the
  1,900-line lint cap, raised four times since December 2025.

## Where code goes in 2.0.0

The layer follows from what the code does:

| The code… | Goes to | Language |
|---|---|---|
| draws a screen, a modal or the HUD | Shell | TypeScript (React Native) |
| holds a rule, app state, a network call, the identity, settings or a cache policy | Core | Rust, no Godot |
| renders or simulates 3D, or runs a scene | Engine | Rust on Godot |
| needs an OS API that JS must not or cannot use | Native host | Kotlin / Swift |
| is an app feature with a maintained RN library | that library | none of ours |

Tie-breakers: anything the shell and the engine both read, and anything that must work while the
engine is not running, goes in the core ([05-system-placement.md](05-system-placement.md)). Nothing
talks sideways: RN and the engine only talk to the core (rule 3).

## Pros and cons

| Pros | Cons |
|---|---|
| **Fewer crashes.** Rust and TypeScript catch mistakes at build time. | **A big refactor.** A lot of work now that pays off for years: a short-term cost. |
| **Clear placement rule.** The layer follows from what the code does. | **More memory.** Hermes next to V8; budget +100 MB RSS (R-1). |
| **Testable without a device.** `cargo test` for the core, Jest for screens. | **Two render surfaces.** Surface order, touch, keyboard focus (R-2, R-3). |
| **Native UI and integrations.** Text input, accessibility, system UI, RN libraries. | **A bigger build.** Node and Metro next to Cargo, Godot, Gradle, Xcode. |
| **Faster start, less battery.** No engine for the first screen or behind menus (D-42). |  |
| **Fewer languages and builds.** TypeScript and Rust; three builds instead of six (D-41). |  |
| **An engine with one job.** Scenes only; it is also the desktop emulator. |  |
| **A replaceable shell.** The core and the engine carry over to any UI (D-open-16). |  |

## Why a native UI, not the engine's UI

A game engine draws its own UI; a native shell uses the OS's. For an app that is mostly menus,
social and commerce around a 3D world, the OS does most of the work:

- **Text input.** The OS keyboard, autocorrect, password managers and autofill, paste menus,
  selection handles, emoji and every input method, with keyboard insets handled by the OS.
- **Text and accessibility.** OS text rendering and font fallback for every script; VoiceOver,
  TalkBack, system font size and dark mode work through the OS's own support.
- **Platform behaviour.** Scroll physics, back gesture, safe areas, haptics and virtualized lists
  with an image cache, the way users expect them on each OS.
- **System UI is native anyway.** Share sheet, in-app browser and auth sessions, permission prompts,
  notifications, the purchase sheet. In 1.x each is a Godot plugin call; in RN it is one import.
- **Start-up and battery.** The first screen draws in the time RN takes to start, with no engine;
  menus do not keep a 3D renderer running.
- **Iteration.** Fast refresh in under a second; screens are built from Figma (all 286 frames have one).

## Why React Native

1. **Native views.** RN renders real `UIView` / `android.view` objects, so the Godot view is one more
   native view next to them and the host orders the surfaces. No embedding layer between the two.
2. **Libraries for every app feature we need** (D-41): push and local notifications, IAP for StoreKit
   and Play Billing, Reown AppKit, Firebase, install referrer, share, calendar, review, in-app
   browser, Sentry. Each is maintained code we do not own.
3. **One binding generator.** uniffi produces the TypeScript bindings (`uniffi-bindgen-react-native`,
   JSI) and the host's Kotlin/Swift bindings from the same Rust definitions (D-17, D-41).
4. **TypeScript.** The language of the SDK and of Decentraland's web apps; types and libraries can be
   shared.
5. **It exists.** ~27,700 lines of TypeScript; every Figma frame has a screen.
6. **AI-assisted work.** React and TypeScript are where AI tools are strongest; the spike's screens
   were built that way, and the whole plan depends on it.
7. **Mainstream and maintained.** New architecture (JSI, Fabric, bridgeless) and Hermes by default;
   screens testable with Jest. Over-the-air UI fixes stay possible later.

## Why not Flutter or another

| Option | Memory on top of the engine | With a Godot surface | Status |
|---|---|---|---|
| **React Native** (chosen) | Highest of the mainstream options in public tests: ~68 MB iOS baseline vs ~45 for Flutter; +45 MB (iOS) and +33 MB (Android) in a list test vs +10 and +6 native | Native views; Godot is a sibling view | **Selected.** Solid, mainstream |
| **Flutter** | Lower than RN in pure-app tests (+25 / +14 MB in the same list test), but brings a **second GPU renderer** (Impeller) with its own GPU memory next to Godot's | Godot's SurfaceView as a platform view merges Flutter's raster and UI threads (lower fps) or loses SurfaceView support; the fast mode (HCPP) is experimental and needs Android 14 with Vulkan | **Not preferable.** Solid; two GPU renderers, non-native widgets, Dart |
| **Native: SwiftUI + Compose** | Lowest | Native views | **Possible** (Plan B2). Solid; two UI codebases |
| **Godot UI on the new architecture** | None added | One surface | **Possible** (Plan B1). Our stack; keeps the engine-UI limits above |
| **Compose Multiplatform** | Kotlin/Native heap; Skia on iOS | iOS draws its own canvas, like Flutter | **Not preferable.** Stable; its own canvas on iOS, third-party uniffi bindings |
| **Lynx** (ByteDance) | Claimed lower than RN, no independent numbers | Native views | **Not preferable.** Too young; no wallet, IAP or push libraries |
| **Valdi** (Snap) | Snap claims a quarter of competitors', unverified | Native views (TSX compiled) | **Not preferable.** Too young (beta); not React |
| **Slint, egui, Dioxus** (Rust UI) | Smallest (Slint: under 300 KB) | Draws its own widgets | **Discarded.** Too young on iOS; no library ecosystem |
| **WebView** (Capacitor) | A browser engine in-process, more than RN | Web view | **Discarded.** Solid; heaviest memory |

Flutter is the closest alternative. What makes it not preferable here is not its memory: next to a 3D engine it
means two GPU renderers and an embedding problem on Android that RN does not have.

## If memory is the problem

1. **Measure first.** 2.9 compares a release RN-host build with the Godot-exported app from the same
   commit on the Moto Edge 60 Pro and an iPhone. Budget: no worse fps, +100 MB RSS at most (R-1).
   Public numbers put RN's own cost in tens of MB, so a miss is more likely in how we use it.
2. **Fix the RN host**, time-boxed (D-open-16): an opaque Godot surface except on the backpack, the
   engine paused behind RN screens (D-42), HUD state at a low rate, screens behind the world frozen,
   a bounded image cache, rarely used screens loaded lazily.
3. **Then Plan B, at the 1 Nov checkpoint (D-39).** B1 for January: the UI back in Godot on the new
   architecture. B2 later if needed: native UIs on uniffi's Kotlin/Swift bindings. Both keep the core,
   the engine and the state machine.

## Sources

Public benchmarks disagree and few state their method; only our own measurement (2.9) decides.

- SynergyBoat, Flutter vs React Native vs Native, 2025 (list test, iPhone 16 Plus, Galaxy Z Fold 6):
  https://www.synergyboat.com/blog/flutter-vs-react-native-vs-native-performance-benchmark-2025
- CatDoes, Flutter vs React Native 2026 (iOS baselines, Flutter 3.41, RN 0.84):
  https://catdoes.com/blog/flutter-vs-react-native-2026
- Flutter, Android platform views (hybrid composition, SurfaceView, HCPP):
  https://docs.flutter.dev/platform-integration/android/platform-views
- JetBrains, Compose Multiplatform 1.8.0, iOS stable:
  https://blog.jetbrains.com/kotlin/2025/05/compose-multiplatform-1-8-0-released-compose-multiplatform-for-ios-is-stable-and-production-ready/
- Lynx vs React Native: https://thenewstack.io/cross-platform-ui-framework-lynx-competes-with-react-native/
- Valdi: https://github.com/snapchat/valdi
- Slint 1.12 (iOS tech preview): https://slint.dev/blog/slint-1.12-released
