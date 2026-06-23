import SwiftGodotRuntime
import Foundation

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
}
