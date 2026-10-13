// JS-facing end of the bridge. Commands and events are JSON strings.
#import <React/RCTBridgeModule.h>
#import <React/RCTEventEmitter.h>

@interface GodotBridge : RCTEventEmitter <RCTBridgeModule>

/// Routes the engine's RnHostPlugin.rn_event calls to JS. Call after godot_host_init.
+ (void)bindEngineEvents;

@end
