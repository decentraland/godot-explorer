extends RefCounted

# Freed-instance errors must reach Sentry whether or not the `sentry-error-events`
# firehose flag is on: on upstream's release template the same access is a SIGSEGV,
# so every logged hit is a use-after-free in our GDScript that we want to see.
# Exercises the text-only decision in ProjectMainLoop._firehose_source; the event
# shape test in _before_send cannot be driven from GDScript (SentryEvent has no
# way to add an exception), so this covers everything below it.

# One message per engine site that reports a freed object (fork's gdscript_vm.cpp,
# variant_setget.cpp). A new engine string that is not caught here is a silent
# regression, so keep this list in step with _is_freed_instance_error.
const FREED_MESSAGES := [
	"Cannot call method 'show' on a previously freed instance.",
	"Left operand of 'is' is a previously freed instance.",
	"Trying to assign invalid previously freed instance.",
	"Trying to return a previously freed instance.",
	"Trying to iterate on a previously freed object.",
	"Trying to cast a freed object.",
	"Trying to await on a freed object.",
	"Invalid assignment of property or key 'name_claimed' with value of type 'bool' on a base object of type 'previously freed'.",
	"Invalid access to property or key 'visible' on a base object of type 'previously freed'.",
]

# OTHER_MESSAGES[0..3] are indexed by the flag-on classification checks below.
const OTHER_MESSAGES := [
	'Parameter "p_node" is null.',
	'Node not found: "%PanelContainer_NewBadge" (relative to "/root/explorer").',
	"[Rust:dclgodot::comms] connection closed (src/comms/mod.rs:10)",
	"[Rust:dclgodot::dcl::js] [scene SceneId(2)] script error onUpdate: Error: channel closed",
	'Condition "!is_inside_tree()" is true. Returning: false',
	# Scene text is not ours: freed-object wording in it must not pick the exempt source.
	"[Rust:dclgodot::dcl::js] [scene SceneId(2)] script error onUpdate: Error: previously freed",
	"[Rust:dclgodot::dcl::js] [scene SceneId(2)] script error: Trying to cast a freed object.",
	# Printed by object.cpp before the fork's release check existed; stays flag-gated.
	"Object 'Control' was freed or unreferenced while a signal is being emitted from it. Try connecting to the signal using 'CONNECT_DEFERRED' flag, or use queue_free() to free the object (if this object is a Node) to avoid this error and potential crashes.",
]

var suite_name := "sentry_before_send"
var method_name := "test_freed_instance_errors_bypass_the_firehose_flag"
var errors: Array[String] = []
var execution_time_seconds := 0.0


func run() -> bool:
	var start := Time.get_ticks_usec()
	var ok := true

	for text in FREED_MESSAGES:
		ok = (
			_expect(ProjectMainLoop._is_freed_instance_error(text), true, "matches: " + text) and ok
		)
		# Flag off (the default): kept, and as the dedicated source.
		ok = (
			_expect_source(
				ProjectMainLoop._firehose_source(text, false),
				ProjectMainLoop.SOURCE_FREED_INSTANCE,
				"flag off keeps: " + text
			)
			and ok
		)
		# Flag on: still the dedicated source, never re-bucketed as engine noise.
		ok = (
			_expect_source(
				ProjectMainLoop._firehose_source(text, true),
				ProjectMainLoop.SOURCE_FREED_INSTANCE,
				"flag on keeps: " + text
			)
			and ok
		)

	for text in OTHER_MESSAGES:
		ok = (
			_expect(ProjectMainLoop._is_freed_instance_error(text), false, "no match: " + text)
			and ok
		)
		# Flag off: everything else is dropped, exactly as before.
		ok = (
			_expect_source(
				ProjectMainLoop._firehose_source(text, false), "", "flag off drops: " + text
			)
			and ok
		)

	# Flag on: the classifier is untouched by the new branch.
	ok = (
		_expect_source(
			ProjectMainLoop._firehose_source(OTHER_MESSAGES[5], true),
			ProjectMainLoop.SOURCE_SCENE,
			"scene freed wording stays scene"
		)
		and ok
	)
	ok = (
		_expect_source(
			ProjectMainLoop._firehose_source(OTHER_MESSAGES[2], true),
			ProjectMainLoop.SOURCE_RUST_APP,
			"rust app still classified"
		)
		and ok
	)
	ok = (
		_expect_source(
			ProjectMainLoop._firehose_source(OTHER_MESSAGES[3], true),
			ProjectMainLoop.SOURCE_SCENE,
			"scene still classified"
		)
		and ok
	)
	ok = (
		_expect_source(
			ProjectMainLoop._firehose_source(OTHER_MESSAGES[0], true),
			ProjectMainLoop.SOURCE_ENGINE,
			"engine still classified"
		)
		and ok
	)

	# The new source is sampled like a crash: kept in full and exempt from the remote rate.
	ok = (
		_expect(
			ProjectMainLoop.SOURCE_KEEP_RATE.get(ProjectMainLoop.SOURCE_FREED_INSTANCE, 0.0) == 1.0,
			true,
			"keep rate is 1.0"
		)
		and ok
	)
	ok = (
		_expect(
			ProjectMainLoop.SOURCE_FREED_INSTANCE in ProjectMainLoop.REMOTE_RATE_EXEMPT,
			true,
			"exempt from the remote sample rate"
		)
		and ok
	)

	execution_time_seconds = (Time.get_ticks_usec() - start) / 1_000_000.0
	return ok


func _expect(actual: bool, expected: bool, label: String) -> bool:
	if actual != expected:
		errors.append("%s: expected %s, got %s" % [label, expected, actual])
		return false
	return true


func _expect_source(actual: String, expected: String, label: String) -> bool:
	if actual != expected:
		errors.append("%s: expected source '%s', got '%s'" % [label, expected, actual])
		return false
	return true
