import Foundation
import UIKit

/// Device readings for the engine (`dcl_core::platform`). Called from the engine's thread, which
/// is the main thread on iOS.
final class IosDevice: DeviceService {
  func info() -> DeviceInfo {
    let os = ProcessInfo.processInfo.operatingSystemVersion
    let version =
      os.patchVersion > 0
      ? "\(os.majorVersion).\(os.minorVersion).\(os.patchVersion)"
      : "\(os.majorVersion).\(os.minorVersion)"
    return DeviceInfo(
      brand: "Apple",
      model: Self.sysctl("hw.machine") ?? "",
      osVersion: "iOS \(version)",
      totalRamMb: Int32(ProcessInfo.processInfo.physicalMemory / (1024 * 1024)),
      socModel: "",
      socManufacturer: "",
      boardPlatform: "",
      hardware: "",
      processorType: "",
      processorMaxFreqMhz: -1
    )
  }

  func readings() -> DeviceReadings {
    let thermal: String
    switch ProcessInfo.processInfo.thermalState {
    case .nominal: thermal = "nominal"
    case .fair: thermal = "fair"
    case .serious: thermal = "serious"
    case .critical: thermal = "critical"
    @unknown default: thermal = "unknown"
    }
    let (level, state) = Self.onMain {
      UIDevice.current.isBatteryMonitoringEnabled = true
      return (UIDevice.current.batteryLevel, UIDevice.current.batteryState)
    }
    let charging: String
    switch state {
    case .unplugged: charging = "unplugged"
    case .charging: charging = "plugged"  // iOS does not tell USB from wireless
    case .full: charging = "full"
    default: charging = "unknown"
    }
    return DeviceReadings(
      memoryUsageMb: Self.footprintMb(),
      // iOS exposes no device temperature.
      temperatureCelsius: -1,
      thermalState: thermal,
      batteryPercent: level >= 0 ? level * 100 : -1,
      chargingState: charging
    )
  }

  /// The engine's main loop is the UI thread on iOS: blocking it is the engine's own fallback.
  func blockUiThread(ms: UInt32) -> Bool {
    false
  }

  /// `phys_footprint`, the figure Xcode shows.
  private static func footprintMb() -> Int32 {
    var info = task_vm_info_data_t()
    var count = mach_msg_type_number_t(
      MemoryLayout<task_vm_info_data_t>.size / MemoryLayout<natural_t>.size)
    let result = withUnsafeMutablePointer(to: &info) {
      $0.withMemoryRebound(to: integer_t.self, capacity: Int(count)) {
        task_info(mach_task_self_, task_flavor_t(TASK_VM_INFO), $0, &count)
      }
    }
    return result == KERN_SUCCESS ? Int32(info.phys_footprint / (1024 * 1024)) : -1
  }

  private static func sysctl(_ name: String) -> String? {
    var size = 0
    guard sysctlbyname(name, nil, &size, nil, 0) == 0, size > 0 else { return nil }
    var buffer = [CChar](repeating: 0, count: size)
    guard sysctlbyname(name, &buffer, &size, nil, 0) == 0 else { return nil }
    return String(cString: buffer)
  }

  private static func onMain<T: Sendable>(_ body: @MainActor () -> T) -> T {
    Thread.isMainThread
      ? MainActor.assumeIsolated(body) : DispatchQueue.main.sync { MainActor.assumeIsolated(body) }
  }
}

/// The engine's scene video on iOS: one `AVPlayerWrapper` per player id, frames handed over as
/// `IOSurfaceRef`s. Called from the engine's thread.
final class AVVideoBackend: VideoBackend, @unchecked Sendable {
  private let lock = NSLock()
  private var players: [Int32: AVPlayerWrapper] = [:]
  private var nextId: Int32 = 1

  private func player(_ id: Int32) -> AVPlayerWrapper? {
    lock.lock()
    defer { lock.unlock() }
    return players[id]
  }

  func create() -> Int32 {
    lock.lock()
    defer { lock.unlock() }
    let id = nextId
    nextId += 1
    players[id] = AVPlayerWrapper(id: id)
    return id
  }

  func initSurface(player: Int32, width: Int32, height: Int32) -> Bool {
    (self.player(player)?.initializeSurface(withWidth: width, height: height) ?? -1) > 0
  }

  func setSource(player: Int32, source: String, local: Bool) -> Bool {
    guard let p = self.player(player) else { return false }
    return local ? p.setSourceLocal(source) : p.setSourceURL(source)
  }

  func play(player: Int32) { self.player(player)?.play() }

  func pause(player: Int32) { self.player(player)?.pause() }

  func stop(player: Int32) { self.player(player)?.stop() }

  func seek(player: Int32, seconds: Double) { self.player(player)?.setPosition(Float(seconds)) }

  func position(player: Int32) -> Double { Double(self.player(player)?.getPosition() ?? 0) }

  func duration(player: Int32) -> Double { Double(self.player(player)?.getDuration() ?? 0) }

  func isPlaying(player: Int32) -> Bool { self.player(player)?.isPlaying ?? false }

  func setVolume(player: Int32, volume: Float) { self.player(player)?.volume = volume }

  func setLooping(player: Int32, looping: Bool) { self.player(player)?.isLooping = looping }

  func setRate(player: Int32, rate: Float) { self.player(player)?.setPlaybackRate(rate) }

  func takeSizeChanged(player: Int32) -> Bool {
    guard let p = self.player(player), p.hasVideoSizeChanged() else { return false }
    p.clearVideoSizeChangedFlag()
    return true
  }

  func videoSize(player: Int32) -> VideoSize {
    guard let p = self.player(player) else { return VideoSize(width: 0, height: 0) }
    return VideoSize(width: p.videoWidth, height: p.videoHeight)
  }

  func pollFrame(player: Int32) -> UInt64 {
    guard let p = self.player(player), p.hasNewPixelBuffer() else { return 0 }
    return p.acquireIOSurfacePtr()
  }

  func release(player: Int32) {
    lock.lock()
    let p = players.removeValue(forKey: player)
    lock.unlock()
    p?.releasePlayer()
  }
}
