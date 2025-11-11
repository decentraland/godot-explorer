//
// © 2024-present https://github.com/cengiz-pz
//

#import "drivers/apple_embedded/godot_app_delegate.h"
#import "deeplink_service.h"
#import "dcl_godot_ios.h"



struct DeeplinkServiceInitializer {
	DeeplinkServiceInitializer() {
		[GDTApplicationDelegate addService:[DeeplinkService shared]];
	}
};
static DeeplinkServiceInitializer initializer;


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
	DclGodotiOS::receivedUrl = String(url.absoluteString.UTF8String);

	if (url) {
		// Check if the URL is a custom scheme (not http or https)
		BOOL isCustomScheme = ![url.scheme isEqualToString:@"http"] && ![url.scheme isEqualToString:@"https"];
		NSLog(@"Deeplink plugin: %@ URL received: %@", isCustomScheme ? @"Custom scheme" : @"Universal Link", url.absoluteString);

		// DeeplinkPlugin* plugin = DeeplinkPlugin::get_singleton();
		// if (plugin) {
		// 	Dictionary urlData = [GDPConverter nsUrlToGodotDictionary:url];
		// 	plugin->emit_signal(DEEPLINK_RECEIVED_SIGNAL, urlData);
		// }
	}
	else {
		NSLog(@"Deeplink plugin: URL is empty!");
	}

	return YES;
}

- (BOOL) application:(UIApplication*) app continueUserActivity:(NSUserActivity*) userActivity restorationHandler:(void (^)(NSArray<id<UIUserActivityRestoring>>* restorableObjects)) restorationHandler {
	if ([userActivity.activityType isEqualToString: NSUserActivityTypeBrowsingWeb]) {
		NSURL* url = userActivity.webpageURL;
		DclGodotiOS::receivedUrl = String(url.absoluteString.UTF8String);
		
		NSLog(@"Deeplink plugin: Universal Link received at app resumption: %@", url.absoluteString);

		// DeeplinkPlugin* plugin = DeeplinkPlugin::get_singleton();
		// if (plugin) {
		// 	Dictionary urlData = [GDPConverter nsUrlToGodotDictionary:url];
		// 	plugin->emit_signal(DEEPLINK_RECEIVED_SIGNAL, urlData);
		// }
	}

	return YES;
}

- (BOOL) application:(UIApplication*) app didFinishLaunchingWithOptions:(NSDictionary<UIApplicationLaunchOptionsKey, id>*) launchOptions {
	if (launchOptions) {
		NSURL *url = [launchOptions objectForKey:UIApplicationLaunchOptionsURLKey];
		if (url) {
			BOOL isCustomScheme = ![url.scheme isEqualToString:@"http"] && ![url.scheme isEqualToString:@"https"];
			NSLog(@"Deeplink plugin: %@ received at startup: %@", isCustomScheme ? @"Custom scheme URL" : @"Universal Link", url.absoluteString);
			DclGodotiOS::receivedUrl = String(url.absoluteString.UTF8String);

			// DeeplinkPlugin* plugin = DeeplinkPlugin::get_singleton();
			// if (plugin) {
			// 	Dictionary urlData = [GDPConverter nsUrlToGodotDictionary:url];
			// 	plugin->emit_signal(DEEPLINK_RECEIVED_SIGNAL, urlData);
			// }
		}
		else {
			NSLog(@"Deeplink plugin: UIApplicationLaunchOptionsURLKey is empty!");

			NSDictionary* userActivityDict = [launchOptions objectForKey:UIApplicationLaunchOptionsUserActivityDictionaryKey];
			if (userActivityDict) {
				url = [userActivityDict objectForKey:UIApplicationLaunchOptionsURLKey];
				if (url) {
					BOOL isCustomScheme = ![url.scheme isEqualToString:@"http"] && ![url.scheme isEqualToString:@"https"];
					NSLog(@"Deeplink plugin: %@ received at startup from user activity dictionary: %@", isCustomScheme ? @"Custom scheme URL" : @"Universal Link", url.absoluteString);
					DclGodotiOS::receivedUrl = String(url.absoluteString.UTF8String);

					// DeeplinkPlugin* plugin = DeeplinkPlugin::get_singleton();
					// if (plugin) {
					// 	Dictionary urlData = [GDPConverter nsUrlToGodotDictionary:url];
					// 	plugin->emit_signal(DEEPLINK_RECEIVED_SIGNAL, urlData);
					// }
				}
				else {
					NSLog(@"Deeplink plugin: UIApplicationLaunchOptionsURLKey is empty in user activity dictionary!");
					
					NSUserActivity* userActivity = [userActivityDict objectForKey:@"UIApplicationLaunchOptionsUserActivityKey"];
					if (userActivity) {
						if ([userActivity.activityType isEqualToString: NSUserActivityTypeBrowsingWeb]) {
							url = userActivity.webpageURL;
							DclGodotiOS::receivedUrl = String(url.absoluteString.UTF8String);
							
							NSLog(@"Deeplink plugin: Universal Link received at app startup from user activity: %@", url.absoluteString);

							// DeeplinkPlugin* plugin = DeeplinkPlugin::get_singleton();
							// if (plugin) {
							// 	Dictionary urlData = [GDPConverter nsUrlToGodotDictionary:url];
							// 	plugin->emit_signal(DEEPLINK_RECEIVED_SIGNAL, urlData);
							// }
						}
						else {
							NSLog(@"Deeplink plugin: activity type is %@", userActivity.activityType);
						}
					}
					else {
						NSLog(@"Deeplink plugin: No user activity in user activity dictionary!");
					}
				}
			}
			else {
				NSLog(@"Deeplink plugin: No user activity dictionary either!");
			}
		}
	}
	else {
		NSLog(@"Deeplink plugin: launch options is empty!");
	}

	return YES;
}

@end
