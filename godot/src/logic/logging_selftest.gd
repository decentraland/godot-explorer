class_name LoggingSelftest
extends RefCounted

## Exercises every logging stack (GDScript, Rust, Swift, Kotlin, ObjC) so each can be confirmed
## in the unified channel; grep `[LOGTEST]`.

var _done: bool = false


## Logging self-test, triggered by `--test-logging` / `?test-logging=true`.
## Exercises every logging form in every stack (GDScript / Rust / Swift / ObjC /
## Kotlin) so we can confirm each pipes into the unified channel. Grep `[LOGTEST]`
## to verify. The ERROR/WARN forms (push_error/push_warning, tracing error/warn,
## NSLog/os_log faults, Log.e) also exercise the Sentry pipeline.
## Run the logging self-test if requested (via `--test-logging` cli flag or a
## `?test-logging=true` deeplink), at most once. Called at _ready (covers the cli
## flag, e.g. iOS's baked arg) and on `deep_link_received` (Android delivers the
## deeplink async, after _ready — and its launcher strips command-line args).
func maybe_run() -> void:
	if _done:
		return
	# Never run in production: the self-test deliberately fires push_error /
	# tracing::error! / ERR_PRINT / Log.e, all of which ship to Sentry — otherwise
	# reachable in prod via the `?test-logging=true` deeplink. Staging/TestFlight
	# still run it so the Sentry path can be verified on a real build.
	if Global.is_production():
		return
	var requested := Global.cli.test_logging
	if not requested and Global.deep_link_obj != null:
		requested = str(Global.deep_link_obj.params.get("test-logging", "")).to_lower() == "true"
	if requested:
		_done = true
		run.call_deferred()


func run() -> void:
	print("[LOGTEST] ===== logging self-test start =====")

	# --- GDScript stack: every logging form ---
	print("[LOGTEST][gdscript] info via print()")
	prints("[LOGTEST][gdscript]", "info", "via", "prints()")
	print_rich("[LOGTEST][gdscript] info via print_rich() [color=yellow]rich[/color]")
	printraw("[LOGTEST][gdscript] raw via printraw() (no newline)\n")
	printerr("[LOGTEST][gdscript] error via printerr() (stderr)")
	push_warning("[LOGTEST][gdscript] warn via push_warning() (expect Sentry)")
	push_error("[LOGTEST][gdscript] error via push_error() (expect Sentry)")

	# --- Rust stack: all tracing levels + raw println!/eprintln! (DclGlobal) ---
	Global.test_logging()

	# --- Native stacks: gated by per-platform availability ---
	if DclSwiftLibPlugin.is_available():
		print("[LOGTEST][swift] DclSwiftLib.test_logging() -> ", DclSwiftLibPlugin.test_logging())
	if DclAndroidPlugin.is_available():
		DclAndroidPlugin.test_logging()
		print("[LOGTEST][kotlin] invoked DclAndroidPlugin.test_logging()")
	# has_singleton first: get_singleton logs an ERROR (→ Sentry) when absent.
	if Engine.has_singleton("DclGodotiOS"):
		var dcl_ios = Engine.get_singleton("DclGodotiOS")
		if dcl_ios != null and dcl_ios.has_method("test_logging"):
			dcl_ios.test_logging()
			print("[LOGTEST][objc] invoked DclGodotiOS.test_logging()")

	print("[LOGTEST] ===== logging self-test end =====")
