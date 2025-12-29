import com.android.build.gradle.internal.tasks.factory.dependsOn

plugins {
    id("com.android.library")
    id("org.jetbrains.kotlin.android")
}

val pluginName = "dcl-godot-android"

val pluginPackageName = "org.decentraland.godotexplorer"

android {
    namespace = pluginPackageName
    compileSdk = 33
    ndkVersion = "28.1.13356709"  // NDK version for native build (matches xtask ANDROID_NDK_VERSION)

    buildFeatures {
        buildConfig = true
    }

    defaultConfig {
        minSdk = 21

        manifestPlaceholders["godotPluginName"] = pluginName
        manifestPlaceholders["godotPluginPackageName"] = pluginPackageName
        buildConfigField("String", "GODOT_PLUGIN_NAME", "\"${pluginName}\"")
        setProperty("archivesBaseName", pluginName)

        // Native build configuration for HardwareBuffer JNI
        externalNativeBuild {
            cmake {
                cppFlags += "-std=c++17"
                // Target only arm64-v8a and x86_64 (most common for modern devices)
                abiFilters += listOf("arm64-v8a", "armeabi-v7a", "x86_64")
                // Use API 26 for AHardwareBuffer_fromHardwareBuffer
                // Runtime checks ensure GPU mode only activates on API 29+
                arguments += "-DANDROID_PLATFORM=android-26"
            }
        }

        ndk {
            // Match the ABI filters
            abiFilters += listOf("arm64-v8a", "armeabi-v7a", "x86_64")
        }
    }

    externalNativeBuild {
        cmake {
            path = file("src/main/cpp/CMakeLists.txt")
            version = "3.22.1"
        }
    }

    buildTypes {
        release {
            isMinifyEnabled = false
            proguardFiles(
                getDefaultProguardFile("proguard-android-optimize.txt"),
                "proguard-rules.pro"
            )
        }
    }

    compileOptions {
        sourceCompatibility = JavaVersion.VERSION_17
        targetCompatibility = JavaVersion.VERSION_17
    }
    kotlinOptions {
        jvmTarget = "17"
    }
}

dependencies {
    implementation("org.godotengine:godot:4.3.0.stable")
    implementation("androidx.browser:browser:1.5.0")

    // ExoPlayer dependencies for video playback
    // Using 'api' instead of 'implementation' to make dependencies transitive
    // This ensures ExoPlayer classes are included in the final APK
    api("androidx.media3:media3-exoplayer:1.4.1")
    api("androidx.media3:media3-exoplayer-dash:1.4.1")
    api("androidx.media3:media3-exoplayer-hls:1.4.1")
    api("androidx.media3:media3-ui:1.4.1")
}

// BUILD TASKS DEFINITION
val copyDebugAARToDemoAddons by tasks.registering(Copy::class) {
    description = "Copies the generated debug AAR binary to the plugin's addons directory"
    from("build/outputs/aar")
    include("$pluginName-debug.aar")
    into("demo/addons/$pluginName/bin/debug")
}

val copyReleaseAARToDemoAddons by tasks.registering(Copy::class) {
    description = "Copies the generated release AAR binary to the plugin's addons directory"
    from("build/outputs/aar")
    include("$pluginName-release.aar")
    into("demo/addons/$pluginName/bin/release")
}

val cleanDemoAddons by tasks.registering(Delete::class) {
    delete("demo/addons/$pluginName", "addons/$pluginName")
}

val copyAddonsToDemo by tasks.registering(Copy::class) {
    description = "Copies the export scripts templates to the plugin's addons directory"

    dependsOn(cleanDemoAddons)
    finalizedBy(copyDebugAARToDemoAddons, copyReleaseAARToDemoAddons)

    from("export_scripts_template")
    into("demo/addons/$pluginName")
}

tasks.named("assemble").configure {
    finalizedBy(copyAddonsToDemo)
}

tasks.named<Delete>("clean").apply {
    dependsOn(cleanDemoAddons)
}