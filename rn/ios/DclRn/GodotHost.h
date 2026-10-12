// VIBE-CODED SPIKE: boots the Godot engine inside the RN app and builds the window's root view
// controller: Godot's view controller at the bottom, the React Native view on top.
#import <UIKit/UIKit.h>

NS_ASSUME_NONNULL_BEGIN

@interface DclGodotHost : NSObject

/// Starts the engine as an ephemeral guest in Genesis Plaza with the Godot UI hidden, and returns
/// a root view controller that stacks `reactView` over Godot. Touches go to RN only where a real
/// RN view is under them; everything else reaches Godot (joystick, gamepad, camera look).
+ (UIViewController *)rootViewControllerWithReactView:(UIView *)reactView;

@end

NS_ASSUME_NONNULL_END
