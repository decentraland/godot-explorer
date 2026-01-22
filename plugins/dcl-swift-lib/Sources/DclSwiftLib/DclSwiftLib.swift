import SwiftGodotRuntime
import Foundation

// Entry point for DclSwiftLib GDExtension
// NOTE: We use a unique symbol name to avoid conflicts with other GDExtensions
// (like Sentry) that also use "gdextension_init". On iOS, we patch dummy.cpp
// via ios_xcode.rs to call this symbol instead of the default "gdextension_init".
#initSwiftExtension(cdecl: "dcl_walletconnect_init", types: [
    DclWalletConnect.self
])
