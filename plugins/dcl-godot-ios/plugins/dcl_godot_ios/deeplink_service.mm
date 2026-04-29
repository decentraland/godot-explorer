//
// © 2024-present https://github.com/cengiz-pz
//

#import "drivers/apple_embedded/godot_app_delegate.h"
#import "deeplink_service.h"
#import "dcl_godot_ios.h"
#import <objc/runtime.h>

static bool deeplink_service_initialized = false;

// Forward declarations for injected methods
static void injected_scene_openURLContexts(id self, SEL _cmd, UIScene* scene, NSSet<UIOpenURLContext*>* URLContexts);
static void injected_scene_willConnectToSession(id self, SEL _cmd, UIScene* scene, UISceneSession* session, UISceneConnectionOptions* connectionOptions);

// Inject scene URL handling methods into GDTApplicationDelegate at runtime
// This is needed because Godot 4.6 uses scene-based lifecycle but doesn't forward
// scene:openURLContexts: to services, breaking deep link handling
static void inject_scene_url_methods() {
	Class delegateClass = [GDTApplicationDelegate class];

	// Inject scene:openURLContexts: method
	SEL openURLSel = @selector(scene:openURLContexts:);
	if (!class_respondsToSelector(delegateClass, openURLSel)) {
		NSLog(@"[DEEPLINK] Injecting scene:openURLContexts: into GDTApplicationDelegate");
		class_addMethod(delegateClass, openURLSel, (IMP)injected_scene_openURLContexts, "v@:@@");
	}

	// Inject scene:willConnectToSession:options: method
	SEL willConnectSel = @selector(scene:willConnectToSession:options:);
	if (!class_respondsToSelector(delegateClass, willConnectSel)) {
		NSLog(@"[DEEPLINK] Injecting scene:willConnectToSession:options: into GDTApplicationDelegate");
		class_addMethod(delegateClass, willConnectSel, (IMP)injected_scene_willConnectToSession, "v@:@@@");
	}
}

// Injected method: handles deep links when app is running (e.g., Safari tab returning)
static void injected_scene_openURLContexts(id self, SEL _cmd, UIScene* scene, NSSet<UIOpenURLContext*>* URLContexts) {
	NSLog(@"[DEEPLINK] scene:openURLContexts: called with %lu URL(s)", (unsigned long)URLContexts.count);
	for (UIOpenURLContext* context in URLContexts) {
		NSURL* url = context.URL;
		NSLog(@"[DEEPLINK] Scene URL received: %@", url.absoluteString);
		if (url) {
			DclGodotiOS::emit_deeplink_received(String(url.absoluteString.UTF8String));
		}
	}
}

// Injected method: handles deep links on cold start
static void injected_scene_willConnectToSession(id self, SEL _cmd, UIScene* scene, UISceneSession* session, UISceneConnectionOptions* connectionOptions) {
	NSLog(@"[DEEPLINK] scene:willConnectToSession: called");

	// Handle URL contexts passed at launch
	if (connectionOptions.URLContexts.count > 0) {
		NSLog(@"[DEEPLINK] Launch with %lu URL context(s)", (unsigned long)connectionOptions.URLContexts.count);
		for (UIOpenURLContext* context in connectionOptions.URLContexts) {
			NSURL* url = context.URL;
			NSLog(@"[DEEPLINK] Launch URL: %@", url.absoluteString);
			if (url) {
				DclGodotiOS::emit_deeplink_received(String(url.absoluteString.UTF8String));
			}
		}
	}

	// Handle user activities (universal links) passed at launch
	if (connectionOptions.userActivities.count > 0) {
		for (NSUserActivity* activity in connectionOptions.userActivities) {
			if ([activity.activityType isEqualToString:NSUserActivityTypeBrowsingWeb]) {
				NSURL* url = activity.webpageURL;
				NSLog(@"[DEEPLINK] Launch Universal Link: %@", url.absoluteString);
				if (url) {
					DclGodotiOS::emit_deeplink_received(String(url.absoluteString.UTF8String));
				}
			}
		}
	}
}

struct DeeplinkServiceInitializer {
	DeeplinkServiceInitializer() {
		if (!deeplink_service_initialized) {
			inject_scene_url_methods();
			[GDTApplicationDelegate addService:[DeeplinkService shared]];
			deeplink_service_initialized = true;
		}
	}
};
static DeeplinkServiceInitializer initializer;

// C function to force initialization
void force_deeplink_service_initialization() {
	if (!deeplink_service_initialized) {
		inject_scene_url_methods();
		[GDTApplicationDelegate addService:[DeeplinkService shared]];
		deeplink_service_initialized = true;
	}
}


@implementation DeeplinkService

- (instancetype) init {
	self = [super init];
	NSLog(@"[DEEPLINK] DeeplinkService initialized!");
	return self;
}

+ (instancetype) shared {
	static DeeplinkService* sharedInstance = nil;
	static dispatch_once_t onceToken;
	dispatch_once(&onceToken, ^{
		sharedInstance = [[DeeplinkService alloc] init];
	});
	return sharedInstance;
}

- (BOOL) application:(UIApplication*) app openURL:(NSURL*) url options:(NSDictionary<UIApplicationOpenURLOptionsKey, id>*) options {
	NSLog(@"[DEEPLINK] openURL called with URL: %@", url.absoluteString);
	NSLog(@"[DEEPLINK] Application state: %ld", (long)app.applicationState);
	if (url) {
		DclGodotiOS::emit_deeplink_received(String(url.absoluteString.UTF8String));
	}
	return YES;
}

- (BOOL) application:(UIApplication*) app continueUserActivity:(NSUserActivity*) userActivity restorationHandler:(void (^)(NSArray<id<UIUserActivityRestoring>>* restorableObjects)) restorationHandler {
	NSLog(@"[DEEPLINK] continueUserActivity called, activityType: %@", userActivity.activityType);
	if ([userActivity.activityType isEqualToString: NSUserActivityTypeBrowsingWeb]) {
		NSURL* url = userActivity.webpageURL;
		NSLog(@"[DEEPLINK] Universal Link URL: %@", url.absoluteString);
		DclGodotiOS::emit_deeplink_received(String(url.absoluteString.UTF8String));
	}

	return YES;
}

// Note: Scene-based deep link handling (scene:openURLContexts: and scene:willConnectToSession:options:)
// is handled via runtime method injection into GDTApplicationDelegate.
// See inject_scene_url_methods() above. This is necessary because Godot 4.6 uses scene-based lifecycle
// but doesn't forward these methods to services.

- (BOOL) application:(UIApplication*) app didFinishLaunchingWithOptions:(NSDictionary<UIApplicationLaunchOptionsKey, id>*) launchOptions {
	if (launchOptions) {
		NSLog(@"[DEEPLINK] Launch options available, keys: %@", [launchOptions allKeys]);

		NSURL *url = [launchOptions objectForKey:UIApplicationLaunchOptionsURLKey];
		if (url) {
			DclGodotiOS::emit_deeplink_received(String(url.absoluteString.UTF8String));
		} else {
			NSDictionary* userActivityDict = [launchOptions objectForKey:UIApplicationLaunchOptionsUserActivityDictionaryKey];
			if (userActivityDict) {
				url = [userActivityDict objectForKey:UIApplicationLaunchOptionsURLKey];
				if (url) {
					DclGodotiOS::emit_deeplink_received(String(url.absoluteString.UTF8String));
				} else {
					NSUserActivity* userActivity = [userActivityDict objectForKey:@"UIApplicationLaunchOptionsUserActivityKey"];
					if (userActivity) {
						if ([userActivity.activityType isEqualToString: NSUserActivityTypeBrowsingWeb]) {
							url = userActivity.webpageURL;
							DclGodotiOS::emit_deeplink_received(String(url.absoluteString.UTF8String));
						}
					}
				}
			}
		}
	}

	return YES;
}

@end
