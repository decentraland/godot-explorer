// Boots the Godot engine inside the RN app and builds the window's root view
// controller: Godot's view controller at the bottom, the React Native view on top.
#import <UIKit/UIKit.h>

NS_ASSUME_NONNULL_BEGIN

@interface DclGodotHost : NSObject

/// Starts the engine as an ephemeral guest in Genesis Plaza with the Godot UI hidden, and returns
/// a root view controller that stacks `reactView` over Godot. Touches go to RN only where a real
/// RN view is under them; everything else reaches Godot (joystick, gamepad, camera look).
+ (UIViewController *)rootViewControllerWithReactView:(UIView *)reactView;

// The DclGodotiOS plugin listens on GDTApplicationDelegate, which is not the app delegate here:
// AppDelegate forwards deep links, universal links and APNs callbacks through these.
+ (void)forwardLaunchOptions:(nullable NSDictionary<UIApplicationLaunchOptionsKey, id> *)launchOptions NS_SWIFT_NAME(forwardLaunchOptions(_:));
+ (void)forwardOpenURL:(NSURL *)url options:(NSDictionary<UIApplicationOpenURLOptionsKey, id> *)options NS_SWIFT_NAME(forwardOpenURL(_:options:));
+ (void)forwardUserActivity:(NSUserActivity *)userActivity NS_SWIFT_NAME(forwardUserActivity(_:));
+ (void)forwardDeviceToken:(nullable NSData *)deviceToken error:(nullable NSError *)error NS_SWIFT_NAME(forwardDeviceToken(_:error:));

@end

NS_ASSUME_NONNULL_END
