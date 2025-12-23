#ifndef AVPLAYER_WRAPPER_H
#define AVPLAYER_WRAPPER_H

#import <Foundation/Foundation.h>
#import <AVFoundation/AVFoundation.h>
#import <CoreVideo/CoreVideo.h>

/**
 * AVPlayerWrapper - Objective-C wrapper for AVPlayer with zero-copy GPU texture support.
 *
 * This class manages AVPlayer instances for video playback and provides access to
 * video frames via IOSurface for zero-copy GPU texture sharing with Godot.
 *
 * The key features are:
 * - Hardware-accelerated video decoding via VideoToolbox
 * - Zero-copy frame access via CVPixelBuffer/IOSurface
 * - Triple buffering to ensure safe GPU access to frames
 */
@interface AVPlayerWrapper : NSObject

// Player ID for GDScript reference
@property (nonatomic, readonly) int playerId;

// Video dimensions (updated when video loads)
@property (nonatomic, readonly) int videoWidth;
@property (nonatomic, readonly) int videoHeight;

// Texture dimensions (may differ from video dimensions)
@property (nonatomic, readonly) int textureWidth;
@property (nonatomic, readonly) int textureHeight;

// Playback state
@property (nonatomic, readonly) BOOL isPlaying;
@property (nonatomic) BOOL isLooping;
@property (nonatomic) float volume;

// Flag indicating video size has changed since last check
@property (nonatomic, readonly) BOOL videoSizeChanged;

// Initialization
- (instancetype)initWithId:(int)playerId;

// Surface initialization (called before playback to set initial texture size)
- (int)initializeSurfaceWithWidth:(int)width height:(int)height;

// Source management
- (BOOL)setSourceURL:(NSString *)urlString;
- (BOOL)setSourceLocal:(NSString *)filePath;

// Playback control
- (void)play;
- (void)pause;
- (void)stop;

// Position and duration (in seconds)
- (void)setPosition:(float)positionSec;
- (float)getPosition;
- (float)getDuration;

// Playback rate (1.0 = normal speed)
- (void)setPlaybackRate:(float)rate;

// State queries
- (BOOL)hasVideoSizeChanged;
- (void)clearVideoSizeChangedFlag;

// GPU texture methods (key for zero-copy)
- (BOOL)hasNewPixelBuffer;
- (uint64_t)acquireIOSurfacePtr;

// Cleanup
- (void)releasePlayer;

// Debug
- (NSString *)getPlayerInfo;

@end

#endif // AVPLAYER_WRAPPER_H
