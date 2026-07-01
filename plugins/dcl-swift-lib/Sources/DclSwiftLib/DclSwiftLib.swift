import SwiftGodotRuntime
import Foundation
import os

// Entry point for the DclSwiftLib GDExtension.
//
// We use a unique entry symbol (rather than the default `gdextension_init`)
// to avoid clashes with other GDExtensions linked into the iOS export
// (e.g. Sentry). The matching `entry_symbol` is set in
// `godot/dcl_swift_lib.gdextension`.
#initSwiftExtension(cdecl: "dcl_swift_lib_init", types: [
    DclSwiftLib.self,
    DclStoreKit.self,
    DclDevRelay.self,
])

/// Smoke-test class so we can confirm the Swift GDExtension actually loads
/// in Godot before adding real features. Usage from GDScript:
///
///     var lib = DclSwiftLib.new()
///     print(lib.ping())  # -> "ok"
@Godot
class DclSwiftLib: RefCounted {
    @Callable
    func ping() -> String {
        NSLog("[DclSwiftLib] ping() called")
        return "ok"
    }

    @Callable
    func version() -> String {
        NSLog("[DclSwiftLib] version() called")
        return "0.1.0"
    }

    /// Logging self-test for the **Swift stack**: emit at every level via every
    /// Swift logging form, so the unified channel + Sentry pipeline can be
    /// verified end-to-end. Invoked from GDScript's `_run_logging_selftest()` via
    /// `DclSwiftLibPlugin.test_logging()` (registered snake_case as `test_logging`).
    /// `print`/`NSLog`/`fputs` reach the iOS fd capture; `os.Logger` exercises the
    /// unified logging system (`.error`/`.fault` are the Sentry-relevant levels).
    @Callable(autoSnakeCase: true)
    func testLogging() -> String {
        print("[LOGTEST][swift] info via print() (stdout)")
        NSLog("[LOGTEST][swift] info via NSLog")
        let log = Logger(subsystem: "org.decentraland.godotexplorer", category: "logtest")
        log.debug("[LOGTEST][swift] debug via Logger.debug")
        log.info("[LOGTEST][swift] info via Logger.info")
        log.notice("[LOGTEST][swift] notice via Logger.notice")
        log.warning("[LOGTEST][swift] warning via Logger.warning")
        log.error("[LOGTEST][swift] error via Logger.error (expect Sentry)")
        log.fault("[LOGTEST][swift] fault via Logger.fault (expect Sentry)")
        fputs("[LOGTEST][swift] stderr via fputs\n", stderr)
        GD.print("[LOGTEST][swift] info via GD.print (Godot logger)")
        return "swift-logtest-done"
    }
}
