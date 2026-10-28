// Boots the Godot engine inside the RN app and builds the window's root view
// controller: Godot's view controller at the bottom, the React Native view on top.
#import <UIKit/UIKit.h>

NS_ASSUME_NONNULL_BEGIN

@interface DclGodotHost : NSObject

/// Returns a root view controller that stacks `reactView` over Godot once the engine starts. Touches
/// go to RN only where a real RN view is under them; everything else reaches Godot (joystick,
/// gamepad, camera look).
+ (UIViewController *)rootViewControllerWithReactView:(UIView *)reactView;

/// Starts the engine and adds its view under the React Native view. Idempotent. The core asks for
/// it with `engine_required`; the engine then attaches itself to the core. Main thread.
+ (void)startEngine NS_SWIFT_NAME(startEngine());

/// Puts the Godot view above the React Native view while the avatar preview shows, below otherwise.
+ (void)setGodotOnTop:(BOOL)onTop NS_SWIFT_NAME(setGodotOnTop(_:));

// The DclGodotiOS plugin listens on GDTApplicationDelegate, which is not the app delegate here:
// AppDelegate forwards deep links, universal links and APNs callbacks through these.
+ (void)forwardLaunchOptions:(nullable NSDictionary<UIApplicationLaunchOptionsKey, id> *)launchOptions NS_SWIFT_NAME(forwardLaunchOptions(_:));
+ (void)forwardOpenURL:(NSURL *)url options:(NSDictionary<UIApplicationOpenURLOptionsKey, id> *)options NS_SWIFT_NAME(forwardOpenURL(_:options:));
+ (void)forwardUserActivity:(NSUserActivity *)userActivity NS_SWIFT_NAME(forwardUserActivity(_:));
+ (void)forwardDeviceToken:(nullable NSData *)deviceToken error:(nullable NSError *)error NS_SWIFT_NAME(forwardDeviceToken(_:error:));

@end

NS_ASSUME_NONNULL_END
