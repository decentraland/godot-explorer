#import "GodotHost.h"

#import "GodotBridge.h"

// C API from the engine's drivers/apple_embedded/godot_host.h (built with embedded_host=yes).
extern "C" {
int godot_host_init(int argc, const char *const *argv);
void godot_host_focus_in(void);
void godot_host_focus_out(void);
void godot_host_enter_background(void);
void godot_host_exit_background(void);
void godot_host_memory_warning(void);
}

static NSString *const kMainRealm = @"https://realm-provider-ea.decentraland.org/main";

@implementation DclGodotHost

+ (void)attachToWindow:(UIWindow *)window {
	UIViewController *root = window.rootViewController;
	Class godotViewControllerClass = NSClassFromString(@"GDTViewController");
	if (!root || !godotViewControllerClass) {
		NSLog(@"[DclGodotHost] missing root view controller or GDTViewController (libgodot not linked with -ObjC?)");
		return;
	}

	NSString *executable = NSProcessInfo.processInfo.arguments.firstObject;
	const char *argv[] = {
		executable.UTF8String,
		"--rn-host", "--skip-lobby", "--guest-profile", "--force-mobile",
		"--realm", kMainRealm.UTF8String,
		"--location", "0,0",
	};
	// The engine finds <executable>.pck in the app bundle root (--main-pack is compiled out).
	if (godot_host_init((int)(sizeof(argv) / sizeof(argv[0])), argv) != 0) {
		NSLog(@"[DclGodotHost] godot_host_init failed");
		return;
	}
	[GodotBridge bindEngineEvents];

	UIViewController *godot = [[godotViewControllerClass alloc] init];
	[root addChildViewController:godot];
	godot.view.frame = root.view.bounds;
	godot.view.autoresizingMask = UIViewAutoresizingFlexibleWidth | UIViewAutoresizingFlexibleHeight;
	[root.view insertSubview:godot.view atIndex:0];
	[godot didMoveToParentViewController:root];
	root.view.backgroundColor = UIColor.clearColor;

	NSNotificationCenter *center = NSNotificationCenter.defaultCenter;
	[center addObserverForName:UIApplicationDidBecomeActiveNotification object:nil queue:nil usingBlock:^(NSNotification *) { godot_host_focus_in(); }];
	[center addObserverForName:UIApplicationWillResignActiveNotification object:nil queue:nil usingBlock:^(NSNotification *) { godot_host_focus_out(); }];
	[center addObserverForName:UIApplicationDidEnterBackgroundNotification object:nil queue:nil usingBlock:^(NSNotification *) { godot_host_enter_background(); }];
	[center addObserverForName:UIApplicationWillEnterForegroundNotification object:nil queue:nil usingBlock:^(NSNotification *) { godot_host_exit_background(); }];
	[center addObserverForName:UIApplicationDidReceiveMemoryWarningNotification object:nil queue:nil usingBlock:^(NSNotification *) { godot_host_memory_warning(); }];
}

@end
