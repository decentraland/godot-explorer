/**
 * JNI native code for ExoPlayer HardwareBuffer integration.
 *
 * This file provides the native bridge to get the AHardwareBuffer* pointer
 * from a Java HardwareBuffer object. This pointer is used by Godot's Vulkan
 * renderer to import the video frame directly without CPU readback.
 */

#include <jni.h>
#include <android/hardware_buffer.h>
#include <android/hardware_buffer_jni.h>
#include <android/log.h>

#define LOG_TAG "ExoPlayerHWBuffer"
#define LOGI(...) __android_log_print(ANDROID_LOG_INFO, LOG_TAG, __VA_ARGS__)
#define LOGE(...) __android_log_print(ANDROID_LOG_ERROR, LOG_TAG, __VA_ARGS__)

extern "C" {

/**
 * Get the native AHardwareBuffer* pointer from a Java HardwareBuffer.
 *
 * This uses the NDK function AHardwareBuffer_fromHardwareBuffer() which
 * is available from API level 26+.
 *
 * @param env JNI environment
 * @param thiz The ExoPlayerWrapper instance (unused)
 * @param hardwareBuffer The Java HardwareBuffer object
 * @return The native AHardwareBuffer* pointer as a jlong, or 0 on failure
 */
JNIEXPORT jlong JNICALL
Java_org_decentraland_godotexplorer_ExoPlayerWrapper_nativeGetHardwareBufferPtr(
        JNIEnv *env,
        jobject /* thiz */,
        jobject hardwareBuffer) {

    if (hardwareBuffer == nullptr) {
        LOGE("nativeGetHardwareBufferPtr: hardwareBuffer is null");
        return 0;
    }

    // Convert Java HardwareBuffer to native AHardwareBuffer*
    // This function is available from NDK API level 26
    AHardwareBuffer *nativeBuffer = AHardwareBuffer_fromHardwareBuffer(env, hardwareBuffer);

    if (nativeBuffer == nullptr) {
        LOGE("nativeGetHardwareBufferPtr: AHardwareBuffer_fromHardwareBuffer failed");
        return 0;
    }

    // Note: We do NOT call AHardwareBuffer_acquire() here because the Java
    // HardwareBuffer already holds a reference. The caller (Godot) must use
    // this pointer immediately and not hold it past the lifetime of the
    // Java HardwareBuffer.

    return reinterpret_cast<jlong>(nativeBuffer);
}

} // extern "C"
