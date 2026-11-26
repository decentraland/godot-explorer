#import "GodotHost.h"

#if DCL_WITH_GODOT
// C API from the engine's drivers/apple_embedded/godot_host.h (built with embedded_host=yes).
extern "C" {
int godot_host_init(int argc, const char *const *argv);
void godot_host_focus_in(void);
void godot_host_focus_out(void);
void godot_host_enter_background(void);
void godot_host_exit_background(void);
void godot_host_memory_warning(void);
}

#endif

// RN's own container views: a touch that only hits these landed on a transparent area.
static BOOL isReactBackground(UIView *view, UIView *reactRoot) {
	if (view == reactRoot) {
		return YES;
	}
	NSString *name = NSStringFromClass(view.class);
	// The safe-area provider ignores pointerEvents, so it is a full-screen target; look through it.
	return [name hasPrefix:@"RCTSurface"] || [name isEqualToString:@"RCTRootComponentView"] ||
			[name hasPrefix:@"RNCSafeAreaProvider"];
}

@interface DclHostView : UIView
@property(nonatomic, weak) UIView *godotView;
@property(nonatomic, weak) UIView *reactView;
@end

@implementation DclHostView

- (void)setGodotOnTop:(BOOL)onTop {
	if (!self.godotView) {
		return;
	}
	if (onTop) {
		[self bringSubviewToFront:self.godotView];
	} else {
		[self sendSubviewToBack:self.godotView];
	}
}

- (void)layoutSubviews {
	[super layoutSubviews];
	self.godotView.frame = self.bounds;
	self.reactView.frame = self.bounds;
}

- (UIView *)hitTest:(CGPoint)point withEvent:(UIEvent *)event {
	UIView *reactHit = [self.reactView hitTest:[self convertPoint:point toView:self.reactView] withEvent:event];
	if (!self.godotView || (reactHit && !isReactBackground(reactHit, self.reactView))) {
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
	self.reactView.backgroundColor = UIColor.clearColor;
	[host addSubview:self.reactView];
	host.reactView = self.reactView;
}

#if DCL_WITH_GODOT
/// Adds Godot's view controller under the React Native view. Only after `godot_host_init`.
- (void)attachGodotView {
	DclHostView *host = (DclHostView *)self.view;
	Class godotViewControllerClass = NSClassFromString(@"GDTViewController");
	if (!godotViewControllerClass) {
		NSLog(@"[DclGodotHost] GDTViewController missing (libgodot not linked with -ObjC?)");
		return;
	}
	UIViewController *godot = [[godotViewControllerClass alloc] init];
	[self addChildViewController:godot];
	[host insertSubview:godot.view belowSubview:self.reactView];
	[godot didMoveToParentViewController:self];
	host.godotView = godot.view;
	[host setNeedsLayout];
}
#endif

// RN owns the orientation: the AppDelegate narrows this to react-native-orientation-locker's mask.
- (UIInterfaceOrientationMask)supportedInterfaceOrientations {
	return UIInterfaceOrientationMaskAll;
}

- (BOOL)prefersStatusBarHidden {
	return YES;
}

- (BOOL)prefersHomeIndicatorAutoHidden {
	return YES;
}

@end

// The host view controller, for starting the engine's view and reordering the surfaces later.
static __weak DclHostViewController *sController = nil;

#if DCL_WITH_GODOT
// Engine arguments: the host's defaults, then the exporter's debug command line (scene inspector,
// remote debug) shipped as godot_cmdline.txt, then the arguments the process was launched with.
static NSArray<NSString *> *engineArguments(void) {
	NSArray<NSString *> *launch = NSProcessInfo.processInfo.arguments;
	NSMutableArray<NSString *> *args = [NSMutableArray arrayWithArray:@[
		launch.firstObject,
		@"--force-mobile", @"--no-pulse",
	]];
	NSString *cmdlinePath = [NSBundle.mainBundle pathForResource:@"godot_cmdline" ofType:@"txt"];
	NSString *cmdline = cmdlinePath ? [NSString stringWithContentsOfFile:cmdlinePath encoding:NSUTF8StringEncoding error:nil] : nil;
	for (NSString *line in [cmdline componentsSeparatedByCharactersInSet:NSCharacterSet.newlineCharacterSet]) {
		if (line.length > 0) {
			[args addObject:line];
		}
	}
	[args addObjectsFromArray:[launch subarrayWithRange:NSMakeRange(1, launch.count - 1)]];
	return args;
}

static BOOL sEngineStarted = NO;

static void startEngineNow(void) {
	if (sEngineStarted) {
		return;
	}
	sEngineStarted = YES;
	NSArray<NSString *> *args = engineArguments();
	const char **argv = (const char **)calloc(args.count, sizeof(char *));
	for (NSUInteger i = 0; i < args.count; i++) {
		argv[i] = args[i].UTF8String;
	}
	// The engine finds <executable>.pck in the app bundle root (--main-pack is compiled out).
	int err = godot_host_init((int)args.count, argv);
	free(argv);
	if (err != 0) {
		NSLog(@"[DclGodotHost] godot_host_init failed: %d", err);
		return;
	}
	[sController attachGodotView];

	NSNotificationCenter *center = NSNotificationCenter.defaultCenter;
	[center addObserverForName:UIApplicationDidBecomeActiveNotification object:nil queue:nil usingBlock:^(NSNotification *) { godot_host_focus_in(); }];
	[center addObserverForName:UIApplicationWillResignActiveNotification object:nil queue:nil usingBlock:^(NSNotification *) { godot_host_focus_out(); }];
	[center addObserverForName:UIApplicationDidEnterBackgroundNotification object:nil queue:nil usingBlock:^(NSNotification *) { godot_host_enter_background(); }];
	[center addObserverForName:UIApplicationWillEnterForegroundNotification object:nil queue:nil usingBlock:^(NSNotification *) { godot_host_exit_background(); }];
	[center addObserverForName:UIApplicationDidReceiveMemoryWarningNotification object:nil queue:nil usingBlock:^(NSNotification *) { godot_host_memory_warning(); }];
}
#endif

@implementation DclGodotHost

+ (UIViewController *)rootViewControllerWithReactView:(UIView *)reactView {
	DclHostViewController *controller = [DclHostViewController new];
	controller.reactView = reactView;
#if DCL_WITH_GODOT
	sController = controller;
#endif
	return controller;
}

+ (void)startEngine {
#if DCL_WITH_GODOT
	startEngineNow();
#endif
}

+ (void)setGodotOnTop:(BOOL)onTop {
	DclHostViewController *controller = sController;
	[(DclHostView *)controller.view setGodotOnTop:onTop];
}

@end
