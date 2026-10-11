#import "GodotBridge.h"

extern "C" {
typedef void (*godot_host_event_callback)(const char *json);
void godot_host_set_event_callback(godot_host_event_callback callback);
void godot_host_send_command(const char *json);
}

static __weak GodotBridge *sInstance = nil;

// Runs on the engine (main) thread.
static void onGodotEvent(const char *json) {
	GodotBridge *bridge = sInstance;
	if (bridge) {
		[bridge sendEventWithName:@"GodotEvent" body:[NSString stringWithUTF8String:json]];
	}
}

@implementation GodotBridge

RCT_EXPORT_MODULE()

+ (BOOL)requiresMainQueueSetup {
	return NO;
}

+ (void)bindEngineEvents {
	godot_host_set_event_callback(onGodotEvent);
}

- (NSArray<NSString *> *)supportedEvents {
	return @[ @"GodotEvent" ];
}

- (void)startObserving {
	sInstance = self;
}

- (void)stopObserving {
	sInstance = nil;
}

RCT_EXPORT_METHOD(sendCommand : (NSString *)json) {
	dispatch_async(dispatch_get_main_queue(), ^{
		godot_host_send_command(json.UTF8String);
	});
}

@end
