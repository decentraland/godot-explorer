# Decentraland Godot Explorer - Platform Plugins

This directory contains platform-specific plugins for the Decentraland Godot Explorer, including Android and iOS build scripts and their respective plugin projects.

## Directory Structure

```
plugins/
├── dcl-godot-android/   # Android plugin project
├── dcl-godot-ios/       # iOS plugin project
├── build_android.sh     # Android build script
├── build_ios.sh         # iOS build script
└── README.md            # This file
```

## Building Plugins

### 🤖 Android Plugin

#### Prerequisites

1. **Android SDK** - Required for building Android plugins
   - Install via [Android Studio](https://developer.android.com/studio) (recommended)
   - Or install [Command Line Tools](https://developer.android.com/studio#command-tools) only
   - The build script will auto-detect common SDK locations:
     - macOS: `~/Library/Android/sdk`
     - Linux: `~/Android/Sdk`
     - Windows: `%LOCALAPPDATA%\Android\Sdk`

2. **Java JDK** - Required by Gradle
   - JDK 11 or higher recommended
   - Install via your package manager or [download from Oracle](https://www.oracle.com/java/technologies/downloads/)

#### Building

```bash
cd plugins
./build_android.sh
```

The script will:
1. Auto-detect or set up Android SDK location
2. Run `./gradlew assemble` to build the plugin
3. Copy the generated AAR files to `../godot/addons/dcl-godot-android/`

#### Output

- Debug AAR: `godot/addons/dcl-godot-android/bin/debug/dcl-godot-android-debug.aar`
- Release AAR: `godot/addons/dcl-godot-android/bin/release/dcl-godot-android-release.aar`

### 🍎 iOS Plugin

#### Prerequisites

1. **macOS** - iOS builds require macOS with Xcode
   
2. **Xcode** - Required for iOS compilation
   - Install from the [Mac App Store](https://apps.apple.com/app/xcode/id497799835)
   - Ensure command line tools are installed: `xcode-select --install`

3. **SCons** - Build system for Godot plugins
   ```bash
   # Install via pip
   pip3 install scons
   
   # Or via Homebrew
   brew install scons
   ```

4. **Godot Source Code** - Required for building iOS plugins
   
   The iOS plugin uses a git submodule for Godot source. Initialize it with:
   ```bash
   cd plugins/dcl-godot-ios
   git submodule update --init --recursive
   ```
   
   This will fetch the Godot source from: https://github.com/decentraland/godotengine
   
   **Note**: The plugin is configured to use a specific Godot commit that matches the engine version used by the project.

#### Building

```bash
cd plugins
./build_ios.sh
```

The script will:
1. Check all prerequisites (macOS, Xcode, SCons, Godot headers)
2. Generate necessary headers via `./scripts/generate_headers.sh`
3. Build XCFrameworks via `./scripts/release_xcframework.sh`
4. Copy the frameworks to `../godot/ios/plugins/`

#### Output

The build creates XCFrameworks for both debug and release configurations:
- `godot/ios/plugins/dcl_godot_ios/` - Contains the built iOS plugins

Each XCFramework includes:
- Device architectures (arm64)
- Simulator architectures (x86_64, arm64)

## Troubleshooting

### Android Issues

**Error: SDK location not found**
- Set `ANDROID_HOME` environment variable: `export ANDROID_HOME=/path/to/android/sdk`
- Or let the script create `local.properties` automatically

**Error: Gradle build failed**
- Ensure you have a compatible JDK version: `java -version`
- Clear Gradle cache: `cd dcl-godot-android && ./gradlew clean`

### iOS Issues

**Error: SCons is not installed**
```bash
pip3 install scons
# or
brew install scons
```

**Error: Godot source headers not found**
```bash
cd plugins/dcl-godot-ios
git submodule update --init --recursive
```

**Error: No SConstruct file found**
- This means the Godot submodule is not properly initialized
- Follow the submodule initialization steps above

**Build fails with header errors**
- Ensure the Godot submodule is on the correct commit
- The project uses a custom Godot fork, don't update to upstream Godot

## Integration with Main Project

After building, the plugins are automatically copied to the appropriate directories in the Godot project:

- **Android**: `godot/addons/dcl-godot-android/`
- **iOS**: `godot/ios/plugins/`

These plugins are then included during the export process when building the final application.

## Additional Resources

- [Godot Android Plugin Documentation](https://docs.godotengine.org/en/stable/tutorials/platform/android/android_plugin.html)
- [Godot iOS Plugin Documentation](https://docs.godotengine.org/en/stable/tutorials/platform/ios/ios_plugin.html)
- [Android Gradle Plugin](https://developer.android.com/build)
- [XCFramework Documentation](https://developer.apple.com/documentation/xcode/creating-a-multi-platform-binary-framework-bundle)
