# Add project specific ProGuard rules here.

# Keep ExoPlayer classes
-keep class androidx.media3.** { *; }
-dontwarn androidx.media3.**

# Keep ExoPlayer interfaces and their implementations
-keepclassmembers class * implements androidx.media3.common.Player {
    *;
}

# Keep ExoPlayer listener interfaces
-keep interface androidx.media3.common.Player$Listener { *; }
-keep class * implements androidx.media3.common.Player$Listener { *; }

# Keep our ExoPlayer wrapper
-keep class org.decentraland.godotexplorer.ExoPlayerWrapper { *; }
-keepclassmembers class org.decentraland.godotexplorer.ExoPlayerWrapper { *; }
