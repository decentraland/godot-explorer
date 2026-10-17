import DeviceCheck
import Security
import UIKit
import React
import React_RCTAppDelegate
import ReactAppDependencyProvider

@main
class AppDelegate: UIResponder, UIApplicationDelegate {
  var window: UIWindow?

  var reactNativeDelegate: ReactNativeDelegate?
  var reactNativeFactory: RCTReactNativeFactory?

  func application(
    _ application: UIApplication,
    didFinishLaunchingWithOptions launchOptions: [UIApplication.LaunchOptionsKey: Any]? = nil
  ) -> Bool {
    // The process's one core, before React Native and before any engine.
    CoreHost.start()
    let delegate = ReactNativeDelegate()
    let factory = RCTReactNativeFactory(delegate: delegate)
    delegate.dependencyProvider = RCTAppDependencyProvider()

    reactNativeDelegate = delegate
    reactNativeFactory = factory

    // The root view controller stacks the React Native view over Godot's.
    let reactView = factory.rootViewFactory.view(
      withModuleName: "DclRn",
      initialProperties: nil,
      launchOptions: launchOptions
    )
    window = UIWindow(frame: UIScreen.main.bounds)
    window?.rootViewController = DclGodotHost.rootViewController(withReactView: reactView)
    window?.makeKeyAndVisible()
    DclGodotHost.forwardLaunchOptions(launchOptions)

    return true
  }

  func application(
    _ app: UIApplication,
    open url: URL,
    options: [UIApplication.OpenURLOptionsKey: Any] = [:]
  ) -> Bool {
    DclGodotHost.forwardOpenURL(url, options: options)
    return RCTLinkingManager.application(app, open: url, options: options)
  }

  func application(
    _ application: UIApplication,
    continue userActivity: NSUserActivity,
    restorationHandler: @escaping ([UIUserActivityRestoring]?) -> Void
  ) -> Bool {
    DclGodotHost.forwardUserActivity(userActivity)
    return RCTLinkingManager.application(
      application, continue: userActivity, restorationHandler: restorationHandler)
  }

  func application(
    _ application: UIApplication,
    didRegisterForRemoteNotificationsWithDeviceToken deviceToken: Data
  ) {
    DclGodotHost.forwardDeviceToken(deviceToken, error: nil)
  }

  func application(
    _ application: UIApplication,
    didFailToRegisterForRemoteNotificationsWithError error: Error
  ) {
    DclGodotHost.forwardDeviceToken(nil, error: error)
  }
}

class ReactNativeDelegate: RCTDefaultReactNativeFactoryDelegate {
  override func sourceURL(for bridge: RCTBridge) -> URL? {
    self.bundleURL()
  }

  override func bundleURL() -> URL? {
#if DEBUG
    RCTBundleURLProvider.sharedSettings().jsBundleURL(forBundleRoot: "index")
#else
    Bundle.main.url(forResource: "main", withExtension: "jsbundle")
#endif
  }
}

/// What the core asks of the host: the engine view when it needs the engine, and the surface order
/// while the avatar preview shows.
final class HostEvents: HostListener {
  func engineRequired() {
    DispatchQueue.main.async { DclGodotHost.startEngine() }
  }

  func avatarPreviewMode(on: Bool) {
    DispatchQueue.main.async { DclGodotHost.setGodotOnTop(on) }
  }
}

/// The OS half of iOS attestation (App Attest); the core owns the session, the cache and the
/// retries. Key ids and attestation objects travel as base64url, as the GDScript service had them.
final class AppAttestAttestation: AttestationService {
  func isSupported() -> Bool {
    DCAppAttestService.shared.isSupported
  }

  func playIntegrityToken(requestHashB64url: String) async throws -> String {
    throw AttestationFailure.Failed(reason: "Play Integrity is Android only")
  }

  func iosGenerateKey() async throws -> String {
    do {
      return Self.base64url(fromBase64: try await DCAppAttestService.shared.generateKey())
    } catch {
      throw AttestationFailure.Failed(reason: error.localizedDescription)
    }
  }

  func iosAttestKey(keyId: String, clientDataHash: Data) async throws -> String {
    do {
      let object = try await DCAppAttestService.shared.attestKey(
        Self.base64(fromBase64url: keyId), clientDataHash: clientDataHash)
      return Self.base64url(fromBase64: object.base64EncodedString())
    } catch {
      throw AttestationFailure.Failed(reason: error.localizedDescription)
    }
  }

  private static func base64url(fromBase64 value: String) -> String {
    value.replacingOccurrences(of: "+", with: "-")
      .replacingOccurrences(of: "/", with: "_")
      .replacingOccurrences(of: "=", with: "")
  }

  private static func base64(fromBase64url value: String) -> String {
    let standard = value.replacingOccurrences(of: "-", with: "+")
      .replacingOccurrences(of: "_", with: "/")
    return standard + String(repeating: "=", count: (4 - standard.count % 4) % 4)
  }
}

/// Creates the core through the uniffi Swift bindings of the same library the engine is linked
/// into (D-6). The data dir is Godot's `user://` on iOS (D-19).
enum CoreHost {
  private static let listener = HostEvents()

  static func start() {
    let documents = FileManager.default.urls(for: .documentDirectory, in: .userDomainMask)[0]
    let version = Bundle.main.object(forInfoDictionaryKey: "CFBundleShortVersionString") as? String ?? ""
    install(
      dataDir: documents.path,
      platform: .ios,
      deviceAnchor: deviceAnchor(),
      appVersion: version
    )
    _ = try? addHostListener(listener: listener)
    try? installAttestation(service: AppAttestAttestation())
  }

  /// The Keychain UUID that survives reinstalls (the guest wallet derives from it): the same item
  /// 1.x used, created on first use.
  private static func deviceAnchor() -> String {
    let service = "xyz.decentraland.guest-anchor"
    let account = "device-id"
    let read: [String: Any] = [
      kSecClass as String: kSecClassGenericPassword,
      kSecAttrService as String: service,
      kSecAttrAccount as String: account,
      kSecReturnData as String: true,
      kSecMatchLimit as String: kSecMatchLimitOne,
    ]
    var result: CFTypeRef?
    if SecItemCopyMatching(read as CFDictionary, &result) == errSecSuccess,
      let data = result as? Data, let uuid = String(data: data, encoding: .utf8), !uuid.isEmpty
    {
      return uuid
    }
    let uuid = UUID().uuidString
    let add: [String: Any] = [
      kSecClass as String: kSecClassGenericPassword,
      kSecAttrService as String: service,
      kSecAttrAccount as String: account,
      kSecValueData as String: Data(uuid.utf8),
      kSecAttrAccessible as String: kSecAttrAccessibleAfterFirstUnlock,
    ]
    SecItemDelete(
      [
        kSecClass as String: kSecClassGenericPassword,
        kSecAttrService as String: service,
        kSecAttrAccount as String: account,
      ] as CFDictionary)
    SecItemAdd(add as CFDictionary, nil)
    return uuid
  }
}
