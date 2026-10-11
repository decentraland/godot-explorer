// VIBE-CODED SPIKE: boots the Godot engine inside the RN app and adds its view controller
// underneath the React Native root view.
#import <UIKit/UIKit.h>

NS_ASSUME_NONNULL_BEGIN

@interface DclGodotHost : NSObject

/// Call after React Native has created window.rootViewController. Starts the engine as an
/// ephemeral guest in Genesis Plaza with the Godot UI hidden, and adds Godot's view below RN's.
+ (void)attachToWindow:(UIWindow *)window;

@end

NS_ASSUME_NONNULL_END
