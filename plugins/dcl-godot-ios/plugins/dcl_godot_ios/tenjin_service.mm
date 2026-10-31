//
// Tenjin MMP bridge for the DclGodotiOS plugin.
//
// Everything goes through the Obj-C runtime (NSClassFromString + objc_msgSend) so this plugin
// builds without the Tenjin headers, and a build that did not bundle the SDK simply reports
// Tenjin as unavailable. The `-ObjC` linker flag in the .gdip keeps the class from being
// dead-stripped.
//

#import "tenjin_service.h"
#import "core/os/os.h"
#import "core/string/print_string.h"
#import <objc/message.h>

#define TENJIN_LOG(fmt, ...) do { \
	NSLog(@"[TENJIN] " fmt, ##__VA_ARGS__); \
	if (OS::get_singleton() != nullptr) { \
		NSString *_tenjin_msg = [NSString stringWithFormat:@"[TENJIN] " fmt, ##__VA_ARGS__]; \
		print_line(String::utf8([_tenjin_msg UTF8String])); \
	} \
} while (0)

#pragma clang diagnostic ignored "-Warc-performSelector-leaks"

static bool tenjin_started = false;
static bool attribution_requested = false;
static NSDictionary *attribution_result = nil;
static bool attribution_failed = false;

static Class tenjin_class() {
	return NSClassFromString(@"TenjinSDK");
}

bool tenjin_start(NSString *sdk_key, NSString *user_id) {
	if (tenjin_started) {
		return true;
	}
	Class cls = tenjin_class();
	if (cls == nil) {
		TENJIN_LOG(@"SDK not linked in this build, Tenjin disabled");
		return false;
	}
	[cls performSelector:@selector(initialize:) withObject:sdk_key];
	if (user_id.length > 0) {
		[cls performSelector:@selector(setCustomerUserId:) withObject:user_id];
	}
	[cls performSelector:@selector(connect)];
	tenjin_started = true;
	TENJIN_LOG(@"started");
	return true;
}

bool tenjin_event(NSString *name, NSInteger value) {
	if (!tenjin_started) {
		return false;
	}
	Class cls = tenjin_class();
	if (value == 0) {
		[cls performSelector:@selector(sendEventWithName:) withObject:name];
	} else {
		((void (*)(Class, SEL, NSString *, NSInteger))objc_msgSend)(cls, @selector(sendEventWithName:andValue:), name, value);
	}
	return true;
}

void tenjin_handle_open_url(NSString *url) {
	Class cls = tenjin_class();
	if (cls == nil || url.length == 0) {
		return;
	}
	[cls performSelector:@selector(handleOpenURLString:) withObject:url];
}

NSDictionary *tenjin_attribution_if_ready(bool *failed) {
	*failed = false;
	if (!tenjin_started) {
		*failed = true;
		return nil;
	}
	@synchronized([NSBundle mainBundle]) {
		if (attribution_result != nil) {
			return attribution_result;
		}
		if (attribution_failed) {
			*failed = true;
			return nil;
		}
	}
	if (!attribution_requested) {
		attribution_requested = true;
		Class cls = tenjin_class();
		id shared = [cls performSelector:@selector(sharedInstance)];
		void (^handler)(NSDictionary *, NSError *) = ^(NSDictionary *info, NSError *error) {
			@synchronized([NSBundle mainBundle]) {
				if (error != nil || info == nil) {
					attribution_failed = true;
				} else {
					attribution_result = [info copy];
				}
			}
		};
		[shared performSelector:@selector(getAttributionInfo:) withObject:handler];
	}
	return nil;
}
