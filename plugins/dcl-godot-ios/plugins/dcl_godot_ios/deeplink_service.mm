//
// © 2024-present https://github.com/cengiz-pz
//

#import "drivers/apple_embedded/godot_app_delegate.h"
#import "deeplink_service.h"
#import "dcl_godot_ios.h"

static bool deeplink_service_initialized = false;

struct DeeplinkServiceInitializer {
	DeeplinkServiceInitializer() {
		if (!deeplink_service_initialized) {
			[GDTApplicationDelegate addService:[DeeplinkService shared]];
			deeplink_service_initialized = true;
		}
	}
};
static DeeplinkServiceInitializer initializer;

// C function to force initialization
void force_deeplink_service_initialization() {
	if (!deeplink_service_initialized) {
		[GDTApplicationDelegate addService:[DeeplinkService shared]];
		deeplink_service_initialized = true;
	}
}


@implementation DeeplinkService

- (instancetype) init {
	self = [super init];
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
	if (url) {
		DclGodotiOS::receivedUrl = String(url.absoluteString.UTF8String);
	}
	return YES;
}

- (BOOL) application:(UIApplication*) app continueUserActivity:(NSUserActivity*) userActivity restorationHandler:(void (^)(NSArray<id<UIUserActivityRestoring>>* restorableObjects)) restorationHandler {
	if ([userActivity.activityType isEqualToString: NSUserActivityTypeBrowsingWeb]) {
		NSURL* url = userActivity.webpageURL;
		DclGodotiOS::receivedUrl = String(url.absoluteString.UTF8String);
	}

	return YES;
}

- (BOOL) application:(UIApplication*) app didFinishLaunchingWithOptions:(NSDictionary<UIApplicationLaunchOptionsKey, id>*) launchOptions {
	if (launchOptions) {
		NSLog(@"[DEEPLINK] Launch options available, keys: %@", [launchOptions allKeys]);

		NSURL *url = [launchOptions objectForKey:UIApplicationLaunchOptionsURLKey];
		if (url) {
			DclGodotiOS::receivedUrl = String(url.absoluteString.UTF8String);
		} else {
			NSDictionary* userActivityDict = [launchOptions objectForKey:UIApplicationLaunchOptionsUserActivityDictionaryKey];
			if (userActivityDict) {
				url = [userActivityDict objectForKey:UIApplicationLaunchOptionsURLKey];
				if (url) {
					DclGodotiOS::receivedUrl = String(url.absoluteString.UTF8String);
				} else {
					NSUserActivity* userActivity = [userActivityDict objectForKey:@"UIApplicationLaunchOptionsUserActivityKey"];
					if (userActivity) {
						if ([userActivity.activityType isEqualToString: NSUserActivityTypeBrowsingWeb]) {
							url = userActivity.webpageURL;
							DclGodotiOS::receivedUrl = String(url.absoluteString.UTF8String);
						}
					}
				}
			}
		}
	}

	return YES;
}

@end
