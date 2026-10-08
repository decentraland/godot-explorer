//
// Remote push (APNs) registration for the DclGodotiOS plugin.
//

#import "drivers/apple_embedded/godot_app_delegate.h"
#import "push_service.h"
#import "dcl_godot_ios.h"
#import "core/os/os.h"
#import "core/string/print_string.h"
#import <UIKit/UIKit.h>
#import <objc/runtime.h>

// Same shape as DEEPLINK_LOG: NSLog for Console.app, print_line for the Godot log stream,
// the latter only once OS_IOS exists (+load runs long before it does).
#define PUSH_LOG(fmt, ...) do { \
	NSLog(@"[PUSH] " fmt, ##__VA_ARGS__); \
	if (OS::get_singleton() != nullptr) { \
		NSString *_push_msg = [NSString stringWithFormat:@"[PUSH] " fmt, ##__VA_ARGS__]; \
		print_line(String::utf8([_push_msg UTF8String])); \
	} \
} while (0)

static bool push_methods_injected = false;
static bool push_registration_requested = false;

static void injected_didRegisterForRemoteNotifications(id self, SEL _cmd, UIApplication *application, NSData *deviceToken) {
	// Hex from the bytes, not -description: since iOS 13 that prints "{length = 32, bytes = 0x…}".
	const unsigned char *bytes = (const unsigned char *)deviceToken.bytes;
	NSMutableString *hex = [NSMutableString stringWithCapacity:deviceToken.length * 2];
	for (NSUInteger i = 0; i < deviceToken.length; i++) {
		[hex appendFormat:@"%02x", bytes[i]];
	}
	PUSH_LOG(@"APNs device token ready (len=%lu)", (unsigned long)hex.length);
	DclGodotiOS::emit_apns_token_ready(String::utf8([hex UTF8String]));
}

static void injected_didFailToRegisterForRemoteNotifications(id self, SEL _cmd, UIApplication *application, NSError *error) {
	// Expected on the simulator and with no network; the empty token still goes out so the
	// device is recorded as unreachable rather than pending.
	PUSH_LOG(@"APNs registration failed: %@", error.localizedDescription);
	DclGodotiOS::emit_apns_token_ready(String());
}

// GDTApplicationDelegate forwards ~40 UIApplicationDelegate selectors to its services and
// none of the remote-notification ones. UIKit only ever calls what respondsToSelector: says
// is there, so addService: alone would never see the token; add the methods to the class
// itself instead, the way deeplink_service.mm does for the scene URL callbacks.
static void inject_push_methods() {
	Class delegateClass = [GDTApplicationDelegate class];

	SEL registeredSel = @selector(application:didRegisterForRemoteNotificationsWithDeviceToken:);
	if (!class_respondsToSelector(delegateClass, registeredSel)) {
		class_addMethod(delegateClass, registeredSel, (IMP)injected_didRegisterForRemoteNotifications, "v@:@@");
	}

	SEL failedSel = @selector(application:didFailToRegisterForRemoteNotificationsWithError:);
	if (!class_respondsToSelector(delegateClass, failedSel)) {
		class_addMethod(delegateClass, failedSel, (IMP)injected_didFailToRegisterForRemoteNotifications, "v@:@@");
	}

	push_methods_injected = true;
}

// Injected at image load: UIApplication snapshots which delegate methods exist when the
// delegate is set, inside UIApplicationMain, which is before any Godot code runs.
@interface PushServiceLoader : NSObject
@end

@implementation PushServiceLoader
+ (void)load {
	if (!push_methods_injected) {
		inject_push_methods();
	}
}
@end

void force_push_service_initialization() {
	// Referenced so -dead_strip keeps the loader (and its +load); see deeplink_service.mm.
	(void)[PushServiceLoader class];
	if (!push_methods_injected) {
		PUSH_LOG(@"force_init: injecting APNs methods (loader +load did not fire first)");
		inject_push_methods();
	}
	if (push_registration_requested) {
		return;
	}
	push_registration_requested = true;

	// Deferred because this runs inside didFinishLaunchingWithOptions (apple_embedded_main is
	// called from there). Registration is silent — only requestAuthorization prompts — so the
	// token is requested on every launch whatever the alert permission says.
	dispatch_async(dispatch_get_main_queue(), ^{
		PUSH_LOG(@"registerForRemoteNotifications");
		[[UIApplication sharedApplication] registerForRemoteNotifications];
	});
}
