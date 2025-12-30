//
// © 2024-present https://github.com/cengiz-pz
//
// Extracted from https://github.com/godot-sdk-integrations/godot-deeplink/blob/35f2bcee4a859ae644cf2adf401b9537b66b671d/ios/DeeplinkPlugin/deeplink_service.h

#ifndef deeplink_plugin_application_delegate_h
#define deeplink_plugin_application_delegate_h

#ifdef __OBJC__
#import <UIKit/UIKit.h>

@interface DeeplinkService : UIResponder<UIApplicationDelegate>

+ (instancetype) shared;

- (BOOL) application:(UIApplication*) app openURL:(NSURL*) url options:(NSDictionary<UIApplicationOpenURLOptionsKey, id>*) options;

- (BOOL) application:(UIApplication*) app continueUserActivity:(NSUserActivity*) userActivity restorationHandler:(void (^)(NSArray<id<UIUserActivityRestoring>>* restorableObjects)) restorationHandler;

- (BOOL) application:(UIApplication*) app didFinishLaunchingWithOptions:(NSDictionary<NSString*,id> *) launchOptions;

@end
#endif

// C++ function to ensure the DeeplinkService initializer runs
#ifdef __cplusplus
extern "C" {
#endif

void force_deeplink_service_initialization();

#ifdef __cplusplus
}
#endif

#endif /* deeplink_plugin_application_delegate_h */
