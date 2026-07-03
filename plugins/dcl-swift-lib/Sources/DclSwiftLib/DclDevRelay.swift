import SwiftGodotRuntime
import Foundation
import Network

/// DEV-ONLY localhost relay for iOS — the equivalent of Android's `adb reverse`.
///
/// iOS resolves `localhost` / `127.0.0.1` in-process and never sends those
/// requests through a proxy or VPN, so a hard-coded `http://localhost:<port>`
/// (e.g. an OAuth callback whose only allowed redirect is `localhost`) cannot be
/// pointed at a dev server on your Mac the way `adb reverse` does on Android.
///
/// This class works around that from *inside* the app: it binds
/// `127.0.0.1:<localPort>` on the device loopback and pipes every accepted
/// connection to `<macHost>:<macPort>` over the LAN. A webview that loads
/// `http://localhost:<localPort>` (or receives the OAuth redirect there) then
/// reaches your Mac's dev server, transparently.
///
/// Requirements:
/// - The Mac dev server must listen on the LAN, not just loopback —
///   `vite --host` (binds `0.0.0.0`). The browser still sends `Host: localhost`,
///   which Vite's host check always allows, so no `allowedHosts` change is needed.
/// - Info.plist:
///     * `NSLocalNetworkUsageDescription` — connecting OUT to the Mac's LAN
///       address triggers the iOS local-network permission prompt.
///     * `NSAppTransportSecurity → NSAllowsLocalNetworking = true` — lets the
///       webview load plain-HTTP `http://localhost:<port>`.
/// - Only effective while the app is foreground. An in-app `WKWebView` /
///   `SFSafariViewController` keeps the app foreground (fine); switching to the
///   standalone Safari app backgrounds this process and the OS suspends the
///   listener.
///
/// `macHost` may be an IP (`"192.168.2.7"`) or a Bonjour name
/// (`"my-mac.local"`); the latter survives Wi-Fi IP changes on the same network.
///
/// GDScript (keep a strong reference — RefCounted is freed when the var drops,
/// which kills the listener):
///
///     # in an autoload, NOT a local var
///     var _dev_relay = null
///     func _ready():
///         if OS.is_debug_build():
///             _dev_relay = DclDevRelay.new()
///             _dev_relay.start("my-mac.local", 5173, 5173)  # mac_host, mac_port, local_port
@Godot
class DclDevRelay: RefCounted, @unchecked Sendable {
    private let queue = DispatchQueue(label: "dcl.dev.relay")
    private var listener: NWListener?

    // Connections are retained here so Network.framework doesn't deallocate them
    // mid-transfer; each pair is dropped on teardown.
    private let lock = NSLock()
    private var live: [ObjectIdentifier: NWConnection] = [:]

    /// Start (or restart) the relay. Returns false if the listener can't bind.
    @Callable(autoSnakeCase: true)
    func start(macHost: String, macPort: Int, localPort: Int) -> Bool {
        stop()
        guard
            localPort > 0, localPort <= 65535, macPort > 0, macPort <= 65535,
            let lport = NWEndpoint.Port(rawValue: UInt16(localPort)),
            let rport = NWEndpoint.Port(rawValue: UInt16(macPort))
        else {
            NSLog("[DclDevRelay] invalid port (local=\(localPort) mac=\(macPort))")
            return false
        }

        let params = NWParameters.tcp
        params.requiredInterfaceType = .loopback   // bind 127.0.0.1 only
        params.allowLocalEndpointReuse = true
        do {
            let l = try NWListener(using: params, on: lport)
            l.newConnectionHandler = { [weak self] client in
                self?.handle(client: client, macHost: macHost, macPort: rport)
            }
            l.start(queue: queue)
            listener = l
            NSLog("[DclDevRelay] listening 127.0.0.1:\(localPort) -> \(macHost):\(macPort)")
            return true
        } catch {
            NSLog("[DclDevRelay] listener failed to bind :\(localPort): \(error)")
            return false
        }
    }

    /// Stop the listener and tear down all in-flight connections. Idempotent.
    @Callable(autoSnakeCase: true)
    func stop() {
        listener?.cancel()
        listener = nil
        lock.lock()
        let all = Array(live.values)
        live.removeAll()
        lock.unlock()
        all.forEach { $0.cancel() }
    }

    deinit {
        stop()
    }

    private func retain(_ c: NWConnection) {
        lock.lock(); live[ObjectIdentifier(c)] = c; lock.unlock()
    }

    private func release(_ c: NWConnection) {
        lock.lock(); live[ObjectIdentifier(c)] = nil; lock.unlock()
    }

    private func handle(client: NWConnection, macHost: String, macPort: NWEndpoint.Port) {
        let server = NWConnection(host: NWEndpoint.Host(macHost), port: macPort, using: .tcp)
        retain(client)
        retain(server)
        let teardown: () -> Void = { [weak self] in
            client.cancel()
            server.cancel()
            self?.release(client)
            self?.release(server)
        }
        client.start(queue: queue)
        server.start(queue: queue)
        // Splice both directions; either side closing tears the pair down.
        pump(from: client, to: server, teardown: teardown)
        pump(from: server, to: client, teardown: teardown)
    }

    private func pump(from: NWConnection, to: NWConnection, teardown: @escaping () -> Void) {
        from.receive(minimumIncompleteLength: 1, maximumLength: 65536) { data, _, isComplete, error in
            if let data = data, !data.isEmpty {
                to.send(content: data, completion: .contentProcessed { _ in })
            }
            if isComplete || error != nil {
                teardown()
                return
            }
            self.pump(from: from, to: to, teardown: teardown)
        }
    }
}
