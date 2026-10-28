//
// Firebase Analytics bridge for the DclGodotiOS plugin.
//
// Everything goes through the Obj-C runtime (NSClassFromString + performSelector) so this
// plugin builds and loads without the Firebase headers, and a build whose export did not
// bundle the SDK simply reports Firebase as unavailable. The linker flag `-ObjC` in the
// .gdip is what keeps the Firebase classes from being dead-stripped.
//

#import "firebase_service.h"
#import "core/os/os.h"
#import "core/string/print_string.h"

#define FIREBASE_LOG(fmt, ...) do { \
	NSLog(@"[FIREBASE] " fmt, ##__VA_ARGS__); \
	if (OS::get_singleton() != nullptr) { \
		NSString *_firebase_msg = [NSString stringWithFormat:@"[FIREBASE] " fmt, ##__VA_ARGS__]; \
		print_line(String::utf8([_firebase_msg UTF8String])); \
	} \
} while (0)

#pragma clang diagnostic ignored "-Warc-performSelector-leaks"

static bool firebase_available = false;

static Class firebase_analytics_class() {
	return NSClassFromString(@"FIRAnalytics");
}

// Mirrors the Android bridge: bools as 1/0, numbers as-is, anything else as its description.
static NSDictionary<NSString *, id> *firebase_params_from_json(NSString *params_json) {
	if (params_json.length == 0) {
		return nil;
	}
	NSData *data = [params_json dataUsingEncoding:NSUTF8StringEncoding];
	id parsed = [NSJSONSerialization JSONObjectWithData:data options:0 error:nil];
	if (![parsed isKindOfClass:[NSDictionary class]]) {
		FIREBASE_LOG(@"failed to parse params json");
		return nil;
	}
	NSMutableDictionary<NSString *, id> *params = [NSMutableDictionary dictionary];
	[(NSDictionary *)parsed enumerateKeysAndObjectsUsingBlock:^(id key, id value, BOOL *stop) {
		if ([value isKindOfClass:[NSString class]]) {
			params[key] = value;
		} else if ([value isKindOfClass:[NSNumber class]]) {
			bool is_bool = CFGetTypeID((__bridge CFTypeRef)value) == CFBooleanGetTypeID();
			params[key] = is_bool ? @([value boolValue] ? 1 : 0) : value;
		} else {
			params[key] = [value description];
		}
	}];
	return params;
}

void force_firebase_service_initialization() {
	if (firebase_available) {
		return;
	}
	Class app_class = NSClassFromString(@"FIRApp");
	if (app_class == nil || firebase_analytics_class() == nil) {
		FIREBASE_LOG(@"SDK not linked in this build, Firebase disabled");
		return;
	}
	// FIRApp configure raises an exception without the plist, so check for it first.
	if ([[NSBundle mainBundle] pathForResource:@"GoogleService-Info" ofType:@"plist"] == nil) {
		FIREBASE_LOG(@"GoogleService-Info.plist missing from the bundle, Firebase disabled");
		return;
	}
	if ([app_class performSelector:@selector(defaultApp)] == nil) {
		[app_class performSelector:@selector(configure)];
	}
	firebase_available = [app_class performSelector:@selector(defaultApp)] != nil;
	FIREBASE_LOG(@"configured (available=%d)", firebase_available);
}

bool firebase_is_available() {
	return firebase_available;
}

NSString *firebase_app_instance_id() {
	if (!firebase_available) {
		return @"";
	}
	NSString *instance_id = [firebase_analytics_class() performSelector:@selector(appInstanceID)];
	return instance_id ?: @"";
}

bool firebase_set_user_id(NSString *user_id) {
	if (!firebase_available) {
		return false;
	}
	[firebase_analytics_class() performSelector:@selector(setUserID:) withObject:(user_id.length > 0 ? user_id : nil)];
	return true;
}

bool firebase_set_user_property(NSString *name, NSString *value) {
	if (!firebase_available) {
		return false;
	}
	[firebase_analytics_class() performSelector:@selector(setUserPropertyString:forName:)
									  withObject:(value.length > 0 ? value : nil)
									  withObject:name];
	return true;
}

bool firebase_log_event(NSString *event_name, NSString *params_json) {
	if (!firebase_available) {
		return false;
	}
	[firebase_analytics_class() performSelector:@selector(logEventWithName:parameters:)
									  withObject:event_name
									  withObject:firebase_params_from_json(params_json)];
	return true;
}
