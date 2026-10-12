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

// RN's own container views: a touch that only hits these landed on a transparent area.
static BOOL isReactBackground(UIView *view, UIView *reactRoot) {
	if (view == reactRoot) {
		return YES;
	}
	NSString *name = NSStringFromClass(view.class);
	return [name hasPrefix:@"RCTSurface"] || [name isEqualToString:@"RCTRootComponentView"];
}

@interface DclHostView : UIView
@property(nonatomic, weak) UIView *godotView;
@property(nonatomic, weak) UIView *reactView;
@end

@implementation DclHostView

- (void)layoutSubviews {
	[super layoutSubviews];
	self.godotView.frame = self.bounds;
	self.reactView.frame = self.bounds;
}

- (UIView *)hitTest:(CGPoint)point withEvent:(UIEvent *)event {
	UIView *reactHit = [self.reactView hitTest:[self convertPoint:point toView:self.reactView] withEvent:event];
	if (reactHit && !isReactBackground(reactHit, self.reactView)) {
		return reactHit;
	}
	return [self.godotView hitTest:[self convertPoint:point toView:self.godotView] withEvent:event];
}

@end

@interface DclHostViewController : UIViewController
@property(nonatomic, strong) UIView *reactView;
@end

@implementation DclHostViewController

- (void)loadView {
	self.view = [DclHostView new];
	self.view.backgroundColor = UIColor.blackColor;
}

- (void)viewDidLoad {
	[super viewDidLoad];
	DclHostView *host = (DclHostView *)self.view;

	Class godotViewControllerClass = NSClassFromString(@"GDTViewController");
	if (godotViewControllerClass) {
		UIViewController *godot = [[godotViewControllerClass alloc] init];
		[self addChildViewController:godot];
		[host addSubview:godot.view];
		[godot didMoveToParentViewController:self];
		host.godotView = godot.view;
	} else {
		NSLog(@"[DclGodotHost] GDTViewController missing (libgodot not linked with -ObjC?)");
	}

	self.reactView.backgroundColor = UIColor.clearColor;
	[host addSubview:self.reactView];
	host.reactView = self.reactView;
}

- (UIInterfaceOrientationMask)supportedInterfaceOrientations {
	return UIInterfaceOrientationMaskLandscape;
}

- (BOOL)prefersStatusBarHidden {
	return YES;
}

- (BOOL)prefersHomeIndicatorAutoHidden {
	return YES;
}

@end

@implementation DclGodotHost

+ (UIViewController *)rootViewControllerWithReactView:(UIView *)reactView {
	DclHostViewController *controller = [DclHostViewController new];
	controller.reactView = reactView;

	NSString *executable = NSProcessInfo.processInfo.arguments.firstObject;
	const char *argv[] = {
		executable.UTF8String,
		"--rn-host", "--skip-lobby", "--guest-profile",
		"--realm", kMainRealm.UTF8String,
		"--location", "0,0",
	};
	// The engine finds <executable>.pck in the app bundle root (--main-pack is compiled out).
	if (godot_host_init((int)(sizeof(argv) / sizeof(argv[0])), argv) != 0) {
		NSLog(@"[DclGodotHost] godot_host_init failed");
		return controller;
	}
	[GodotBridge bindEngineEvents];

	NSNotificationCenter *center = NSNotificationCenter.defaultCenter;
	[center addObserverForName:UIApplicationDidBecomeActiveNotification object:nil queue:nil usingBlock:^(NSNotification *) { godot_host_focus_in(); }];
	[center addObserverForName:UIApplicationWillResignActiveNotification object:nil queue:nil usingBlock:^(NSNotification *) { godot_host_focus_out(); }];
	[center addObserverForName:UIApplicationDidEnterBackgroundNotification object:nil queue:nil usingBlock:^(NSNotification *) { godot_host_enter_background(); }];
	[center addObserverForName:UIApplicationWillEnterForegroundNotification object:nil queue:nil usingBlock:^(NSNotification *) { godot_host_exit_background(); }];
	[center addObserverForName:UIApplicationDidReceiveMemoryWarningNotification object:nil queue:nil usingBlock:^(NSNotification *) { godot_host_memory_warning(); }];
	return controller;
}

@end
