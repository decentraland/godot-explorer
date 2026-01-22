import SwiftGodotRuntime
import Foundation
import UIKit
import WalletConnectSign
import WalletConnectNetworking
import WalletConnectPairing
import WalletConnectRelay
import Starscream
import CryptoSwift
import Combine

// MARK: - WebSocket Wrapper (Starscream 4.x compatible)

class StarscreamWebSocket: WebSocketConnecting {
    var isConnected: Bool = false
    var onConnect: (() -> Void)?
    var onDisconnect: ((Error?) -> Void)?
    var onText: ((String) -> Void)?
    var request: URLRequest

    private var socket: WebSocket?

    // Shared instance to track connection state globally
    static var currentInstance: StarscreamWebSocket?

    init(url: URL) {
        self.request = URLRequest(url: url)
        NSLog("[DclSwiftLib] WebSocket created for URL: \(url)")
        StarscreamWebSocket.currentInstance = self
    }

    private func setupCallbacks() {
        socket?.onEvent = { [weak self] event in
            switch event {
            case .connected(let headers):
                NSLog("[DclSwiftLib] WebSocket connected: \(headers)")
                self?.isConnected = true
                self?.onConnect?()
            case .disconnected(let reason, let code):
                self?.isConnected = false
                self?.onDisconnect?(nil)
                NSLog("[DclSwiftLib] WebSocket disconnected: \(reason), code: \(code)")
            case .text(let text):
                self?.onText?(text)
            case .binary, .ping, .pong, .viabilityChanged, .reconnectSuggested:
                break
            case .cancelled:
                self?.isConnected = false
                self?.onDisconnect?(nil)
            case .error(let error):
                NSLog("[DclSwiftLib] WebSocket error: \(String(describing: error))")
                self?.isConnected = false
                self?.onDisconnect?(error)
            case .peerClosed:
                self?.isConnected = false
                self?.onDisconnect?(nil)
            }
        }
    }

    func connect() {
        NSLog("[DclSwiftLib] WebSocket connecting: \(request.url?.absoluteString ?? "nil")")
        socket = WebSocket(request: request)
        setupCallbacks()
        socket?.connect()
    }

    func disconnect() {
        socket?.disconnect()
    }

    func write(string: String, completion: (() -> Void)?) {
        socket?.write(string: string) {
            completion?()
        }
    }
}

// MARK: - WebSocket Factory

struct DefaultSocketFactory: WebSocketFactory {
    func create(with url: URL) -> WebSocketConnecting {
        return StarscreamWebSocket(url: url)
    }
}

// MARK: - Crypto Provider (using CryptoSwift)

struct DefaultCryptoProvider: CryptoProvider {
    public func recoverPubKey(signature: EthereumSignature, message: Data) throws -> Data {
        // For dApp use case, we don't need to recover pub keys - wallet handles signing
        throw NSError(domain: "DclSwiftLib", code: -1,
                     userInfo: [NSLocalizedDescriptionKey: "Not implemented - using wallet for signing"])
    }

    public func keccak256(_ data: Data) -> Data {
        let digest = SHA3(variant: .keccak256)
        let hash = digest.calculate(for: [UInt8](data))
        return Data(hash)
    }
}

// MARK: - DclWalletConnect (SwiftGodot Extension)
// Named DclWalletConnect to avoid conflicts with other plugins

@Godot
class DclWalletConnect: RefCounted, @unchecked Sendable {

    // Signals for async communication with Godot
    @Signal var connectionStateChanged: SignalWithArguments<String>
    @Signal var signStateChanged: SignalWithArguments<String>
    @Signal var errorOccurred: SignalWithArguments<String>

    // State (exposed via getters)
    private var _isInitialized: Bool = false
    private var _connectionState: String = "disconnected"
    private var _connectedAddress: String = ""
    private var _signState: String = "idle"
    private var _signResult: String = ""
    private var _errorMessage: String = ""
    private var _pairingUri: String = ""
    private var _sessionTopic: String = ""

    private var cancellables = Set<AnyCancellable>()

    // MARK: - Initialization

    @Callable
    func walletConnectInit(projectId: String) -> Bool {
        if _isInitialized {
            NSLog("[DclSwiftLib] Already initialized")
            return true
        }

        let metadata = AppMetadata(
            name: "Decentraland",
            description: "Decentraland Explorer",
            url: "https://decentraland.org",
            icons: ["https://decentraland.org/images/decentraland.png"],
            redirect: try! AppMetadata.Redirect(native: "decentraland://walletconnect", universal: nil)
        )

        // Use App Group for keychain sharing
        let appGroup = "group.org.decentraland.godotexplorer"
        NSLog("[DclSwiftLib] Initializing with app group: \(appGroup)")

        Networking.configure(
            groupIdentifier: appGroup,
            projectId: projectId,
            socketFactory: DefaultSocketFactory()
        )

        Pair.configure(metadata: metadata)
        Sign.configure(crypto: DefaultCryptoProvider())

        setupEventSubscriptions()

        // Clean up stale sessions and pre-connect to relay
        Task {
            await cleanupStaleSessions()

            // Pre-warm the relay connection by connecting early
            // This avoids "cold start" delays when user clicks WalletConnect
            NSLog("[DclSwiftLib] Pre-warming relay connection...")
            do {
                try Networking.instance.connect()
                NSLog("[DclSwiftLib] Relay pre-connected successfully")
            } catch {
                NSLog("[DclSwiftLib] Relay pre-connect failed (will retry on pairing): \(error)")
            }
        }

        _isInitialized = true
        _connectionState = "disconnected"
        NSLog("[DclSwiftLib] WalletConnect initialized with projectId: \(projectId)")
        return true
    }

    @Callable
    func walletConnectIsInitialized() -> Bool {
        return _isInitialized
    }

    @Callable
    func walletConnectIsRelayConnected() -> Bool {
        return StarscreamWebSocket.currentInstance?.isConnected ?? false
    }

    // MARK: - Connection

    @Callable
    func walletConnectCreatePairing() {
        guard _isInitialized else {
            _errorMessage = "Not initialized"
            emitError(_errorMessage)
            return
        }

        _connectionState = "connecting"
        _pairingUri = ""
        _errorMessage = ""
        emitConnectionState()

        NSLog("[DclSwiftLib] Creating pairing Task...")
        Task { @MainActor in
            NSLog("[DclSwiftLib] Task started on MainActor")

            // Wait for relay to be connected (with timeout)
            NSLog("[DclSwiftLib] Waiting for relay connection...")
            let maxWaitTime = 5.0 // seconds
            let checkInterval = 0.1 // seconds
            var waited = 0.0

            while !(StarscreamWebSocket.currentInstance?.isConnected ?? false) && waited < maxWaitTime {
                try? await Task.sleep(nanoseconds: UInt64(checkInterval * 1_000_000_000))
                waited += checkInterval
            }

            if StarscreamWebSocket.currentInstance?.isConnected == true {
                NSLog("[DclSwiftLib] Relay connected after \(waited)s")
            } else {
                NSLog("[DclSwiftLib] Relay not connected after \(waited)s, proceeding anyway...")
            }

            do {
                NSLog("[DclSwiftLib] Calling Sign.instance.connect()...")

                // Use withTimeout to avoid indefinite blocking
                let uri = try await withThrowingTaskGroup(of: WalletConnectURI.self) { group in
                    group.addTask {
                        return try await Sign.instance.connect(
                            requiredNamespaces: [:],
                            optionalNamespaces: [
                                "eip155": ProposalNamespace(
                                    chains: [Blockchain("eip155:1")!],
                                    methods: ["personal_sign", "eth_signTypedData", "eth_signTypedData_v4"],
                                    events: ["chainChanged", "accountsChanged"]
                                )
                            ]
                        )
                    }

                    group.addTask {
                        try await Task.sleep(nanoseconds: 15_000_000_000) // 15 second timeout
                        throw NSError(domain: "DclSwiftLib", code: -2,
                                     userInfo: [NSLocalizedDescriptionKey: "Connection timeout - relay did not respond"])
                    }

                    let result = try await group.next()!
                    group.cancelAll()
                    return result
                }

                NSLog("[DclSwiftLib] Sign.instance.connect() returned")
                self._pairingUri = uri.absoluteString
                NSLog("[DclSwiftLib] Pairing URI created: \(self._pairingUri)")
            } catch {
                self._connectionState = "error"
                self._errorMessage = "Pairing failed: \(error.localizedDescription)"
                NSLog("[DclSwiftLib] Pairing error: \(error)")
                self.emitConnectionState()
                self.emitError(self._errorMessage)
            }
        }
        NSLog("[DclSwiftLib] Pairing Task created")
    }

    @Callable
    func walletConnectGetPairingUri() -> String {
        return _pairingUri
    }

    @Callable
    func walletConnectGetConnectionState() -> String {
        return _connectionState
    }

    @Callable
    func walletConnectGetAddress() -> String {
        return _connectedAddress
    }

    @Callable
    func walletConnectOpenWallet(scheme: String) -> Bool {
        guard !_pairingUri.isEmpty else {
            _errorMessage = "No pairing URI available"
            return false
        }

        var urlString: String
        if scheme.isEmpty {
            urlString = _pairingUri
        } else {
            let encodedUri = _pairingUri.addingPercentEncoding(withAllowedCharacters: .urlQueryAllowed) ?? _pairingUri
            urlString = "\(scheme)wc?uri=\(encodedUri)"
        }

        guard let url = URL(string: urlString) else {
            _errorMessage = "Invalid URL: \(urlString)"
            return false
        }

        NSLog("[DclSwiftLib] Opening wallet with URL: \(urlString)")

        DispatchQueue.main.async {
            UIApplication.shared.open(url, options: [:]) { success in
                if !success {
                    self._errorMessage = "Failed to open wallet app"
                    NSLog("[DclSwiftLib] Failed to open URL: \(urlString)")
                } else {
                    NSLog("[DclSwiftLib] Successfully opened wallet")
                }
            }
        }

        return true
    }

    // MARK: - Signing

    @Callable
    func walletConnectRequestSign(message: String) -> Bool {
        guard _connectionState == "connected", !_sessionTopic.isEmpty else {
            _errorMessage = "Not connected"
            return false
        }

        _signState = "pending"
        _signResult = ""
        _errorMessage = ""
        emitSignState()

        // Convert message to hex for personal_sign
        let hexMessage = "0x" + message.data(using: .utf8)!.map { String(format: "%02x", $0) }.joined()

        Task { @MainActor in
            do {
                let request = try Request(
                    topic: self._sessionTopic,
                    method: "personal_sign",
                    params: AnyCodable([hexMessage, self._connectedAddress]),
                    chainId: Blockchain("eip155:1")!
                )

                try await Sign.instance.request(params: request)
                NSLog("[DclSwiftLib] Sign request sent")

                // Try to open wallet to prompt user
                _ = self.walletConnectOpenWallet(scheme: "")
            } catch {
                self._signState = "error"
                self._errorMessage = "Sign request failed: \(error.localizedDescription)"
                NSLog("[DclSwiftLib] Sign request error: \(error)")
                self.emitSignState()
                self.emitError(self._errorMessage)
            }
        }

        return true
    }

    @Callable
    func walletConnectGetSignState() -> String {
        return _signState
    }

    @Callable
    func walletConnectGetSignResult() -> String {
        return _signResult
    }

    @Callable
    func walletConnectResetSignState() {
        _signState = "idle"
        _signResult = ""
    }

    // MARK: - Misc

    @Callable
    func walletConnectGetError() -> String {
        return _errorMessage
    }

    @Callable
    func walletConnectDisconnect() -> Bool {
        guard !_sessionTopic.isEmpty else {
            return true
        }

        Task {
            do {
                try await Sign.instance.disconnect(topic: _sessionTopic)
                NSLog("[DclSwiftLib] Disconnected")
            } catch {
                NSLog("[DclSwiftLib] Disconnect error: \(error)")
            }
        }

        resetState()
        return true
    }

    // MARK: - Event Subscriptions

    private func setupEventSubscriptions() {
        // Session settled (connection approved)
        Sign.instance.sessionSettlePublisher
            .receive(on: DispatchQueue.main)
            .sink { [weak self] (session, _) in
                self?.handleSessionApproved(session)
            }
            .store(in: &cancellables)

        // Session deleted
        Sign.instance.sessionDeletePublisher
            .receive(on: DispatchQueue.main)
            .sink { [weak self] (topic, _) in
                self?.handleSessionDeleted(topic: topic)
            }
            .store(in: &cancellables)

        // Session response (sign result)
        Sign.instance.sessionResponsePublisher
            .receive(on: DispatchQueue.main)
            .sink { [weak self] response in
                self?.handleSessionResponse(response)
            }
            .store(in: &cancellables)

        // Session rejection
        Sign.instance.sessionRejectionPublisher
            .receive(on: DispatchQueue.main)
            .sink { [weak self] (_, reason) in
                self?._connectionState = "disconnected"
                self?._errorMessage = "Session rejected: \(reason.message)"
                NSLog("[DclSwiftLib] Session rejected: \(reason.message)")
                self?.emitConnectionState()
                self?.emitError(self?._errorMessage ?? "")
            }
            .store(in: &cancellables)
    }

    // MARK: - Event Handlers

    private func handleSessionApproved(_ session: Session) {
        _connectionState = "connected"
        _sessionTopic = session.topic

        // Extract address from first account (format: eip155:1:0x...)
        for (_, namespace) in session.namespaces {
            if let account = namespace.accounts.first {
                _connectedAddress = account.address
                break
            }
        }

        NSLog("[DclSwiftLib] Session approved - address: \(_connectedAddress), topic: \(_sessionTopic)")
        emitConnectionState()
    }

    private func handleSessionDeleted(topic: String) {
        if topic == _sessionTopic {
            NSLog("[DclSwiftLib] Session deleted")
            resetState()
            emitConnectionState()
        }
    }

    private func handleSessionResponse(_ response: Response) {
        NSLog("[DclSwiftLib] Received response for topic: \(response.topic)")

        switch response.result {
        case .response(let value):
            _signState = "success"
            if let stringValue = try? value.get(String.self) {
                _signResult = stringValue
            } else {
                _signResult = String(describing: value)
            }
            NSLog("[DclSwiftLib] Sign success: \(_signResult)")
            emitSignState()
        case .error(let error):
            _signState = "error"
            _errorMessage = error.message
            NSLog("[DclSwiftLib] Sign error: \(_errorMessage)")
            emitSignState()
            emitError(_errorMessage)
        }
    }

    // MARK: - Helpers

    private func resetState() {
        _connectionState = "disconnected"
        _connectedAddress = ""
        _sessionTopic = ""
        _pairingUri = ""
        _signState = "idle"
        _signResult = ""
    }

    private func cleanupStaleSessions() async {
        NSLog("[DclSwiftLib] Cleaning up stale sessions...")

        // Disconnect all existing sessions
        let sessions = Sign.instance.getSessions()
        for session in sessions {
            do {
                try await Sign.instance.disconnect(topic: session.topic)
                NSLog("[DclSwiftLib] Disconnected stale session: \(session.topic)")
            } catch {
                NSLog("[DclSwiftLib] Failed to disconnect session \(session.topic): \(error)")
            }
        }

        // Clean up old pairings (keep only recent ones)
        let pairings = Pair.instance.getPairings()
        NSLog("[DclSwiftLib] Found \(pairings.count) pairings")
        // Note: We don't disconnect pairings here as they may be needed for reconnection

        NSLog("[DclSwiftLib] Cleanup complete. Sessions: \(sessions.count), Pairings: \(pairings.count)")
    }

    private func emitConnectionState() {
        connectionStateChanged.emit(_connectionState)
    }

    private func emitSignState() {
        signStateChanged.emit(_signState)
    }

    private func emitError(_ message: String) {
        errorOccurred.emit(message)
    }
}
