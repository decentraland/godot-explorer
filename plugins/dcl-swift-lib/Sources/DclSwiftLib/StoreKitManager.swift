import SwiftGodotRuntime
import Foundation
import StoreKit
import CryptoKit

/// Logs to both NSLog (Console.app) and Godot's stdout so messages show up
/// in `cargo run -- run --target ios` output as well as the device console.
private func gdLog(_ message: String) {
    NSLog("%@", message)
    GD.print(arg1: Variant(GString(stringLiteral: message)))
}

/// StoreKit 2 wrapper exposed to Godot.
///
/// Lifecycle from GDScript:
///
///     var sk = DclStoreKit.new()
///     sk.products_loaded.connect(_on_products_loaded)
///     sk.purchase_completed.connect(_on_purchase_completed)
///     sk.start_listening()                       # observe Transaction.updates
///     sk.load_products(PackedStringArray(["credits_100", "credits_500"]))
///     # later, when user taps a buy button:
///     sk.purchase("credits_100")
///     # after server-side validation:
///     sk.finish_transaction(tx_id)
///
/// IMPORTANT: never call `finish_transaction` until your backend has validated
/// the JWS. Without `finish()`, StoreKit re-delivers the transaction at every
/// app launch — that's the safety net for crashes / missing backend.
@Godot
class DclStoreKit: RefCounted, @unchecked Sendable {
    // MARK: - Signals

    /// Emitted with a JSON array of product info dicts after `load_products`.
    @Signal var productsLoaded: SignalWithArguments<String>
    /// Emitted with an error message string.
    @Signal var productsLoadFailed: SignalWithArguments<String>

    /// Emitted with a JSON object containing tx info + JWS for backend validation.
    @Signal var purchaseCompleted: SignalWithArguments<String>
    /// `(productId, errorMessage)`
    @Signal var purchaseFailed: SignalWithArguments<String, String>
    /// User cancelled the system purchase sheet. Argument: productId.
    @Signal var purchaseCancelled: SignalWithArguments<String>
    /// Purchase deferred (e.g. Ask-to-Buy). Argument: productId.
    @Signal var purchasePending: SignalWithArguments<String>

    /// Emitted for every `Transaction.updates` notification (re-delivery, async
    /// purchases from another device, etc.). Argument: JSON of the transaction.
    @Signal var transactionUpdated: SignalWithArguments<String>

    // MARK: - State

    private var loadedProducts: [String: Product] = [:]
    private var transactionUpdatesTask: Task<Void, Never>?

    deinit {
        transactionUpdatesTask?.cancel()
    }

    // MARK: - Public API

    @Callable(autoSnakeCase: true)
    func canMakePayments() -> Bool {
        return AppStore.canMakePayments
    }

    /// Start observing `Transaction.updates`. Idempotent. Call this early in
    /// app lifecycle so unfinished transactions from prior sessions surface
    /// via `transaction_updated`.
    @Callable(autoSnakeCase: true)
    func startListening() {
        guard transactionUpdatesTask == nil else {
            gdLog("[DclStoreKit] startListening: already listening")
            return
        }
        gdLog("[DclStoreKit] startListening: subscribing to Transaction.updates")
        transactionUpdatesTask = Task.detached { [weak self] in
            for await update in Transaction.updates {
                await self?.handleTransactionUpdate(update)
            }
        }
    }

    @Callable(autoSnakeCase: true)
    func loadProducts(productIds: PackedStringArray) {
        var ids: [String] = []
        let count = productIds.size()
        var i: Int64 = 0
        while i < count {
            ids.append(productIds.get(index: i))
            i += 1
        }
        gdLog("[DclStoreKit] loadProducts: requesting \(ids)")
        gdLog("[DclStoreKit] bundle: \(Bundle.main.bundleIdentifier ?? "<unknown>") sandbox: \(isSandboxEnvironment())")
        Task { [weak self] in
            guard let self = self else { return }
            do {
                let products = try await Product.products(for: ids)
                let returnedIds = products.map { $0.id }.joined(separator: ",")
                gdLog("[DclStoreKit] loadProducts: got \(products.count) products [\(returnedIds)]")
                for product in products {
                    self.loadedProducts[product.id] = product
                    gdLog("[DclStoreKit]   - \(product.id) | \(product.displayPrice) | \(product.type.rawValue)")
                }
                if products.count == 0 {
                    gdLog("[DclStoreKit] loadProducts: 0 products returned. Likely causes: (1) product still propagating in sandbox (5-30min after metadata save), (2) device not signed into Sandbox Account, (3) bundle id mismatch with ASC, (4) sandbox region != product availability region")
                }
                let json = self.serializeProducts(products)
                self.productsLoaded.emit(json)
            } catch {
                let msg = error.localizedDescription
                gdLog("[DclStoreKit] loadProducts failed: \(msg)")
                self.productsLoadFailed.emit(msg)
            }
        }
    }

    private func isSandboxEnvironment() -> Bool {
        // Best-effort: check if the app receipt URL contains "sandboxReceipt"
        // (works at install-time signal, not perfect for fresh installs)
        guard let url = Bundle.main.appStoreReceiptURL else { return false }
        return url.lastPathComponent == "sandboxReceipt"
    }

    /// Initiates a purchase. `walletAddress` is REQUIRED — it's hashed into a
    /// UUID that's passed to StoreKit as `appAccountToken`, embedded inside
    /// Apple's signed JWS. The backend re-derives this UUID from the wallet
    /// to prove the buyer actually intended to credit that address (defends
    /// against stolen-JWS redirect attacks). Without a wallet, the backend
    /// will reject the resulting transaction.
    @Callable(autoSnakeCase: true)
    func purchase(productId: String, walletAddress: String) {
        guard let product = loadedProducts[productId] else {
            gdLog("[DclStoreKit] purchase: product not loaded: \(productId)")
            purchaseFailed.emit(productId, "product not loaded; call load_products first")
            return
        }
        var options: Set<Product.PurchaseOption> = []
        if let token = appAccountToken(forWallet: walletAddress) {
            options.insert(.appAccountToken(token))
            gdLog("[DclStoreKit] purchase: starting \(productId) appAccountToken=\(token)")
        } else {
            gdLog("[DclStoreKit] purchase: starting \(productId) WITHOUT appAccountToken — backend will reject")
        }
        Task { [weak self] in
            guard let self = self else { return }
            do {
                let result = try await product.purchase(options: options)
                switch result {
                case .success(let verification):
                    self.handlePurchaseVerification(verification, productId: productId)
                case .userCancelled:
                    gdLog("[DclStoreKit] purchase cancelled: \(productId)")
                    self.purchaseCancelled.emit(productId)
                case .pending:
                    gdLog("[DclStoreKit] purchase pending: \(productId)")
                    self.purchasePending.emit(productId)
                @unknown default:
                    self.purchaseFailed.emit(productId, "unknown purchase result")
                }
            } catch {
                let msg = error.localizedDescription
                gdLog("[DclStoreKit] purchase failed: \(productId) - \(msg)")
                self.purchaseFailed.emit(productId, msg)
            }
        }
    }

    /// Marks a transaction as finished. Call this ONLY after your backend
    /// has validated the JWS and credited the user. Without finish(),
    /// StoreKit will keep re-delivering the transaction.
    @Callable(autoSnakeCase: true)
    func finishTransaction(transactionId: String) {
        gdLog("[DclStoreKit] finishTransaction: \(transactionId)")
        Task {
            for await result in Transaction.unfinished {
                if case .verified(let tx) = result, String(tx.id) == transactionId {
                    await tx.finish()
                    gdLog("[DclStoreKit] finished transaction \(transactionId)")
                    return
                }
            }
            gdLog("[DclStoreKit] finishTransaction: tx not found among unfinished: \(transactionId)")
        }
    }

    // MARK: - Private

    /// Deterministic wallet → UUID. Same algorithm as the backend's
    /// `wallet.ts::uuidFromWallet` so the appAccountToken inside the JWS
    /// matches what the server expects from the wallet param.
    private func appAccountToken(forWallet wallet: String) -> UUID? {
        let normalized = wallet.lowercased()
        guard !normalized.isEmpty else { return nil }
        let input = "dcl-iap:" + normalized
        let hash = SHA256.hash(data: Data(input.utf8))
        let bytes = Array(hash.prefix(16))
        guard bytes.count == 16 else { return nil }
        return UUID(uuid: (
            bytes[0], bytes[1], bytes[2], bytes[3],
            bytes[4], bytes[5], bytes[6], bytes[7],
            bytes[8], bytes[9], bytes[10], bytes[11],
            bytes[12], bytes[13], bytes[14], bytes[15]
        ))
    }

    private func handlePurchaseVerification(_ result: VerificationResult<Transaction>, productId: String) {
        switch result {
        case .verified(let tx):
            let json = serializeTransaction(tx, jws: result.jwsRepresentation)
            gdLog("[DclStoreKit] purchase verified: txId=\(tx.id) productId=\(productId)")
            purchaseCompleted.emit(json)
        case .unverified(_, let err):
            let msg = "unverified: \(err.localizedDescription)"
            gdLog("[DclStoreKit] purchase \(productId) - \(msg)")
            purchaseFailed.emit(productId, msg)
        }
    }

    private func handleTransactionUpdate(_ update: VerificationResult<Transaction>) async {
        switch update {
        case .verified(let tx):
            let json = serializeTransaction(tx, jws: update.jwsRepresentation)
            gdLog("[DclStoreKit] transaction update: txId=\(tx.id) productId=\(tx.productID)")
            transactionUpdated.emit(json)
        case .unverified(_, let err):
            gdLog("[DclStoreKit] unverified transaction update: \(err.localizedDescription)")
        }
    }

    private func serializeProducts(_ products: [Product]) -> String {
        let arr: [[String: Any]] = products.map { p in
            return [
                "id": p.id,
                "displayName": p.displayName,
                "description": p.description,
                "price": "\(p.price)",
                "displayPrice": p.displayPrice,
                "type": "\(p.type)",
            ]
        }
        if let data = try? JSONSerialization.data(withJSONObject: arr),
           let s = String(data: data, encoding: .utf8) {
            return s
        }
        return "[]"
    }

    private func serializeTransaction(_ tx: Transaction, jws: String) -> String {
        let obj: [String: Any] = [
            "id": "\(tx.id)",
            "originalId": "\(tx.originalID)",
            "productId": tx.productID,
            "purchaseDate": tx.purchaseDate.timeIntervalSince1970,
            "jwsRepresentation": jws,
        ]
        if let data = try? JSONSerialization.data(withJSONObject: obj),
           let s = String(data: data, encoding: .utf8) {
            return s
        }
        return "{}"
    }
}
