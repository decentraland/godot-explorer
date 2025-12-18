#import "AVPlayerWrapper.h"
#import <UIKit/UIKit.h>

/**
 * AVPlayerWrapper Implementation
 *
 * This class provides zero-copy video frame access for Godot by:
 * 1. Using AVPlayerItemVideoOutput to get CVPixelBuffer frames
 * 2. Configuring pixel buffers with IOSurface backing
 * 3. Using triple buffering to manage frame lifecycle
 *
 * The IOSurface can be imported directly into Metal/Vulkan as a texture,
 * avoiding any CPU-side pixel copying.
 */

@interface AVPlayerWrapper ()

// AVPlayer components
@property (nonatomic, strong) AVPlayer *player;
@property (nonatomic, strong) AVPlayerItem *playerItem;
@property (nonatomic, strong) AVPlayerItemVideoOutput *videoOutput;
@property (nonatomic, strong) id playerEndObserver;
@property (nonatomic, strong) id playerItemObserver;

// Triple buffering for CVPixelBuffer management
// We keep 3 frames: latest (N), previous (N-1), oldest (N-2)
// The oldest frame is safe to release because GPU has finished with it
@property (nonatomic) CVPixelBufferRef latestPixelBuffer;
@property (nonatomic) CVPixelBufferRef previousPixelBuffer;
@property (nonatomic) CVPixelBufferRef oldestPixelBuffer;

// Internal state
@property (nonatomic) int playerId;
@property (nonatomic) int videoWidth;
@property (nonatomic) int videoHeight;
@property (nonatomic) int textureWidth;
@property (nonatomic) int textureHeight;
@property (nonatomic) BOOL isPlaying;
@property (nonatomic) BOOL videoSizeChanged;
@property (nonatomic) BOOL hasReceivedFirstFrame;
@property (nonatomic) CMTime lastFrameTime;
@property (nonatomic) float volumeLevel;

@end

@implementation AVPlayerWrapper

#define AVPLAYER_WRAPPER_VERSION "1.0.1"

- (instancetype)initWithId:(int)playerId {
    self = [super init];
    if (self) {
        _playerId = playerId;
        _videoWidth = 0;
        _videoHeight = 0;
        _textureWidth = 640;
        _textureHeight = 360;
        _isPlaying = NO;
        _isLooping = NO;
        _volumeLevel = 1.0f;
        _videoSizeChanged = NO;
        _hasReceivedFirstFrame = NO;
        _lastFrameTime = kCMTimeZero;

        _latestPixelBuffer = NULL;
        _previousPixelBuffer = NULL;
        _oldestPixelBuffer = NULL;

        NSLog(@"[AVPlayerWrapper v%s] Created player with ID: %d", AVPLAYER_WRAPPER_VERSION, playerId);
    }
    return self;
}

- (void)dealloc {
    [self cleanup];
    NSLog(@"[AVPlayerWrapper] Deallocated player with ID: %d", _playerId);
}

- (void)cleanup {
    // Stop observing
    if (_playerEndObserver) {
        [[NSNotificationCenter defaultCenter] removeObserver:_playerEndObserver];
        _playerEndObserver = nil;
    }

    if (_playerItemObserver) {
        [_playerItem removeObserver:self forKeyPath:@"status"];
        _playerItemObserver = nil;
    }

    // Stop playback
    [_player pause];
    _player = nil;

    // Remove video output
    if (_playerItem && _videoOutput) {
        [_playerItem removeOutput:_videoOutput];
    }
    _videoOutput = nil;
    _playerItem = nil;

    // Release pixel buffers
    @synchronized(self) {
        if (_latestPixelBuffer) {
            CVPixelBufferRelease(_latestPixelBuffer);
            _latestPixelBuffer = NULL;
        }
        if (_previousPixelBuffer) {
            CVPixelBufferRelease(_previousPixelBuffer);
            _previousPixelBuffer = NULL;
        }
        if (_oldestPixelBuffer) {
            CVPixelBufferRelease(_oldestPixelBuffer);
            _oldestPixelBuffer = NULL;
        }
    }

    _isPlaying = NO;
}

- (int)initializeSurfaceWithWidth:(int)width height:(int)height {
    _textureWidth = width;
    _textureHeight = height;
    NSLog(@"[AVPlayerWrapper] Initialized surface with size: %dx%d", width, height);
    return 1; // Success
}

#pragma mark - Video Output Setup

- (void)setupVideoOutput {
    if (_videoOutput) {
        if (_playerItem) {
            [_playerItem removeOutput:_videoOutput];
        }
        _videoOutput = nil;
    }

    // Configure pixel buffer attributes for optimal GPU performance
    // Use YCbCr biplanar format (420YpCbCr8BiPlanarVideoRange) for video
    // Enable IOSurface backing for zero-copy GPU access
    // Enable Metal compatibility
    NSDictionary *attributes = @{
        (NSString *)kCVPixelBufferPixelFormatTypeKey: @(kCVPixelFormatType_32BGRA),
        (NSString *)kCVPixelBufferIOSurfacePropertiesKey: @{},  // Enable IOSurface backing
        (NSString *)kCVPixelBufferMetalCompatibilityKey: @YES
    };

    _videoOutput = [[AVPlayerItemVideoOutput alloc] initWithPixelBufferAttributes:attributes];

    if (_playerItem) {
        [_playerItem addOutput:_videoOutput];
        NSLog(@"[AVPlayerWrapper] Video output configured with BGRA format and IOSurface backing");
    }
}

#pragma mark - Source Management

- (BOOL)setSourceURL:(NSString *)urlString {
    [self cleanup];

    NSURL *url = [NSURL URLWithString:urlString];
    if (!url) {
        NSLog(@"[AVPlayerWrapper] Invalid URL: %@", urlString);
        return NO;
    }

    NSLog(@"[AVPlayerWrapper] Loading URL: %@", urlString);

    // Create player item and player
    _playerItem = [AVPlayerItem playerItemWithURL:url];
    _player = [AVPlayer playerWithPlayerItem:_playerItem];
    _player.volume = _volumeLevel;

    // Setup video output
    [self setupVideoOutput];

    // Observe player item status
    [_playerItem addObserver:self
                  forKeyPath:@"status"
                     options:NSKeyValueObservingOptionNew
                     context:nil];
    _playerItemObserver = @YES;

    // Observe end of playback for looping
    __weak AVPlayerWrapper *weakSelf = self;
    _playerEndObserver = [[NSNotificationCenter defaultCenter]
        addObserverForName:AVPlayerItemDidPlayToEndTimeNotification
        object:_playerItem
        queue:[NSOperationQueue mainQueue]
        usingBlock:^(NSNotification *note) {
            AVPlayerWrapper *strongSelf = weakSelf;
            if (strongSelf && strongSelf.isLooping) {
                [strongSelf.player seekToTime:kCMTimeZero];
                [strongSelf.player play];
                NSLog(@"[AVPlayerWrapper] Looping video");
            } else if (strongSelf) {
                strongSelf.isPlaying = NO;
                NSLog(@"[AVPlayerWrapper] Playback ended");
            }
        }];

    return YES;
}

- (BOOL)setSourceLocal:(NSString *)filePath {
    [self cleanup];

    NSURL *url = [NSURL fileURLWithPath:filePath];
    if (!url || ![[NSFileManager defaultManager] fileExistsAtPath:filePath]) {
        NSLog(@"[AVPlayerWrapper] File not found: %@", filePath);
        return NO;
    }

    NSLog(@"[AVPlayerWrapper] Loading local file: %@", filePath);

    // Create player item and player
    _playerItem = [AVPlayerItem playerItemWithURL:url];
    _player = [AVPlayer playerWithPlayerItem:_playerItem];
    _player.volume = _volumeLevel;

    // Setup video output
    [self setupVideoOutput];

    // Observe player item status
    [_playerItem addObserver:self
                  forKeyPath:@"status"
                     options:NSKeyValueObservingOptionNew
                     context:nil];
    _playerItemObserver = @YES;

    // Observe end of playback for looping
    __weak AVPlayerWrapper *weakSelf = self;
    _playerEndObserver = [[NSNotificationCenter defaultCenter]
        addObserverForName:AVPlayerItemDidPlayToEndTimeNotification
        object:_playerItem
        queue:[NSOperationQueue mainQueue]
        usingBlock:^(NSNotification *note) {
            AVPlayerWrapper *strongSelf = weakSelf;
            if (strongSelf && strongSelf.isLooping) {
                [strongSelf.player seekToTime:kCMTimeZero];
                [strongSelf.player play];
            } else if (strongSelf) {
                strongSelf.isPlaying = NO;
            }
        }];

    return YES;
}

#pragma mark - KVO Observer

- (void)observeValueForKeyPath:(NSString *)keyPath
                      ofObject:(id)object
                        change:(NSDictionary<NSKeyValueChangeKey,id> *)change
                       context:(void *)context {
    if ([keyPath isEqualToString:@"status"]) {
        AVPlayerItemStatus status = (AVPlayerItemStatus)[[change objectForKey:NSKeyValueChangeNewKey] integerValue];

        switch (status) {
            case AVPlayerItemStatusReadyToPlay: {
                NSLog(@"[AVPlayerWrapper] Player ready to play");

                // Get video dimensions from the video track
                NSArray *videoTracks = [_playerItem.asset tracksWithMediaType:AVMediaTypeVideo];
                if (videoTracks.count > 0) {
                    AVAssetTrack *videoTrack = videoTracks[0];
                    CGSize naturalSize = videoTrack.naturalSize;
                    CGAffineTransform transform = videoTrack.preferredTransform;

                    // Apply transform to get actual dimensions (handles rotation)
                    CGSize transformedSize = CGSizeApplyAffineTransform(naturalSize, transform);
                    int newWidth = (int)fabs(transformedSize.width);
                    int newHeight = (int)fabs(transformedSize.height);

                    if (newWidth != _videoWidth || newHeight != _videoHeight) {
                        _videoWidth = newWidth;
                        _videoHeight = newHeight;
                        _videoSizeChanged = YES;
                        NSLog(@"[AVPlayerWrapper] Video size: %dx%d", _videoWidth, _videoHeight);
                    }
                }
                break;
            }
            case AVPlayerItemStatusFailed:
                NSLog(@"[AVPlayerWrapper] Player failed: %@", _playerItem.error.localizedDescription);
                break;
            case AVPlayerItemStatusUnknown:
                NSLog(@"[AVPlayerWrapper] Player status unknown");
                break;
        }
    }
}

#pragma mark - Playback Control

- (void)play {
    if (_player) {
        [_player play];
        _isPlaying = YES;
        NSLog(@"[AVPlayerWrapper] Play");
    }
}

- (void)pause {
    if (_player) {
        [_player pause];
        _isPlaying = NO;
        NSLog(@"[AVPlayerWrapper] Pause");
    }
}

- (void)stop {
    if (_player) {
        [_player pause];
        [_player seekToTime:kCMTimeZero];
        _isPlaying = NO;
        NSLog(@"[AVPlayerWrapper] Stop");
    }
}

- (void)setPosition:(float)positionSec {
    if (_player) {
        CMTime time = CMTimeMakeWithSeconds(positionSec, NSEC_PER_SEC);
        [_player seekToTime:time toleranceBefore:kCMTimeZero toleranceAfter:kCMTimeZero];
    }
}

- (float)getPosition {
    if (_player) {
        CMTime time = _player.currentTime;
        if (CMTIME_IS_VALID(time)) {
            return (float)CMTimeGetSeconds(time);
        }
    }
    return 0.0f;
}

- (float)getDuration {
    if (_playerItem) {
        CMTime duration = _playerItem.duration;
        if (CMTIME_IS_VALID(duration) && !CMTIME_IS_INDEFINITE(duration)) {
            return (float)CMTimeGetSeconds(duration);
        }
    }
    return 0.0f;
}

- (void)setVolume:(float)volume {
    _volumeLevel = fmaxf(0.0f, fminf(1.0f, volume));
    if (_player) {
        _player.volume = _volumeLevel;
    }
}

- (float)volume {
    return _volumeLevel;
}

#pragma mark - State Queries

- (BOOL)hasVideoSizeChanged {
    return _videoSizeChanged;
}

- (void)clearVideoSizeChangedFlag {
    _videoSizeChanged = NO;
}

#pragma mark - GPU Texture Methods

- (BOOL)hasNewPixelBuffer {
    if (!_videoOutput || !_player) {
        return NO;
    }

    CMTime currentTime = _player.currentTime;
    if (!CMTIME_IS_VALID(currentTime)) {
        return NO;
    }

    return [_videoOutput hasNewPixelBufferForItemTime:currentTime];
}

- (uint64_t)acquireIOSurfacePtr {
    if (!_videoOutput || !_player) {
        return 0;
    }

    CMTime currentTime = _player.currentTime;
    if (!CMTIME_IS_VALID(currentTime)) {
        return 0;
    }

    // Get new pixel buffer from video output
    CVPixelBufferRef pixelBuffer = [_videoOutput copyPixelBufferForItemTime:currentTime
                                                         itemTimeForDisplay:NULL];

    if (!pixelBuffer) {
        return 0;
    }

    // Get IOSurface from pixel buffer (zero-copy)
    IOSurfaceRef surface = CVPixelBufferGetIOSurface(pixelBuffer);

    if (!surface) {
        NSLog(@"[AVPlayerWrapper] Warning: CVPixelBuffer does not have IOSurface backing");
        CVPixelBufferRelease(pixelBuffer);
        return 0;
    }

    // Update video dimensions if they changed
    size_t width = CVPixelBufferGetWidth(pixelBuffer);
    size_t height = CVPixelBufferGetHeight(pixelBuffer);

    if ((int)width != _videoWidth || (int)height != _videoHeight) {
        _videoWidth = (int)width;
        _videoHeight = (int)height;
        _videoSizeChanged = YES;

        // Log detailed IOSurface info for debugging
        OSType pixelFormat = CVPixelBufferGetPixelFormatType(pixelBuffer);
        size_t bytesPerRow = CVPixelBufferGetBytesPerRow(pixelBuffer);
        size_t ioWidth = IOSurfaceGetWidth(surface);
        size_t ioHeight = IOSurfaceGetHeight(surface);
        OSType ioFormat = IOSurfaceGetPixelFormat(surface);

        NSLog(@"[AVPlayerWrapper] Video size updated: %dx%d", _videoWidth, _videoHeight);
        NSLog(@"[AVPlayerWrapper] CVPixelBuffer format: %c%c%c%c, bytesPerRow: %zu",
              (char)(pixelFormat >> 24), (char)(pixelFormat >> 16),
              (char)(pixelFormat >> 8), (char)(pixelFormat), bytesPerRow);
        NSLog(@"[AVPlayerWrapper] IOSurface: %zux%zu, format: %c%c%c%c, ptr: %p",
              ioWidth, ioHeight,
              (char)(ioFormat >> 24), (char)(ioFormat >> 16),
              (char)(ioFormat >> 8), (char)(ioFormat), surface);
    }

    // Store pixel buffer with triple buffering
    // This ensures the GPU has finished with old frames before we release them
    [self storePixelBuffer:pixelBuffer];

    // Mark that we've received at least one frame
    if (!_hasReceivedFirstFrame) {
        _hasReceivedFirstFrame = YES;
        NSLog(@"[AVPlayerWrapper] First frame received, IOSurface ready, ptr: %p", surface);
    }

    // Return IOSurface pointer as uint64_t for passing to Godot
    return (uint64_t)surface;
}

- (void)storePixelBuffer:(CVPixelBufferRef)newBuffer {
    @synchronized(self) {
        // Release oldest (N-2 frame, safe to release as GPU has finished with it)
        if (_oldestPixelBuffer) {
            CVPixelBufferRelease(_oldestPixelBuffer);
        }

        // Shift: oldest <- previous <- latest <- new
        _oldestPixelBuffer = _previousPixelBuffer;
        _previousPixelBuffer = _latestPixelBuffer;
        _latestPixelBuffer = newBuffer;
    }
}

#pragma mark - Cleanup

- (void)releasePlayer {
    [self cleanup];
}

#pragma mark - Debug

- (NSString *)getPlayerInfo {
    NSMutableString *info = [NSMutableString string];

    [info appendFormat:@"Player ID: %d\n", _playerId];
    [info appendFormat:@"Video Size: %dx%d\n", _videoWidth, _videoHeight];
    [info appendFormat:@"Texture Size: %dx%d\n", _textureWidth, _textureHeight];
    [info appendFormat:@"Is Playing: %@\n", _isPlaying ? @"YES" : @"NO"];
    [info appendFormat:@"Is Looping: %@\n", _isLooping ? @"YES" : @"NO"];
    [info appendFormat:@"Volume: %.2f\n", _volumeLevel];
    [info appendFormat:@"Position: %.2f / %.2f\n", [self getPosition], [self getDuration]];
    [info appendFormat:@"Has First Frame: %@\n", _hasReceivedFirstFrame ? @"YES" : @"NO"];

    if (_playerItem) {
        [info appendFormat:@"Player Status: %ld\n", (long)_playerItem.status];
    }

    return info;
}

@end
