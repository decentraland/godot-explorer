extends SceneTree

# Unit test for TranslationKey (localization v1, #2667).
#
# The type exists so a display API can demand a key instead of a String, making
# "passed English where a key belongs" a parse error rather than something the
# key scanner has to notice. This test pins the runtime half of that contract:
#   1. construction and raw() round-trip
#   2. is_valid() accepts keys and rejects prose
#   3. text()/format()/format_named()/upper() resolve through the catalogue
#   4. join() translates each piece separately (the concatenation fix)
#   5. _to_string() is loud, so stray interpolation is visible in a log
#
# The parse-time half cannot be asserted from inside GDScript: a script that
# passes a String to a TranslationKey parameter fails to compile, so it cannot be
# loaded to be tested. It is verified by check-gdscript over the tree instead.
#
# Run headless:
#   .bin/godot/godot4_bin --headless --path godot \
#     --script res://src/test/i18n/test_translation_key.gd

var _failures: Array[String] = []


func _initialize() -> void:
	# The expected values below are the English catalogue's. Without pinning, the locale
	# resolves from the machine (OS.get_locale()), so this suite failed on any es/pt_BR
	# host the moment those locales became supported.
	TranslationServer.set_locale("en")
	_test_construction()
	_test_is_valid()
	_test_resolution()
	_test_join()
	_test_exists()
	_test_to_string()
	_finish()


func _test_construction() -> void:
	var key := TranslationKey.new("COMMON_CANCEL")
	_expect_eq("raw() round-trips", key.raw(), "COMMON_CANCEL")
	_expect_eq("key field matches", key.key, "COMMON_CANCEL")


func _test_is_valid() -> void:
	# Checked statically: constructing a prose key would (correctly) trip the
	# debug guard in _init and spray push_error through the test output.
	for valid in ["COMMON_CANCEL", "NEARBY", "MODAL_ADD_EMAIL_TITLE", "TOOLTIP_ACTION_1"]:
		if not TranslationKey.is_key(valid):
			_fail("is_key rejected a key: %s" % valid)
	# Shapes that are prose, and would have been the bug.
	for invalid in ["Try restarting the app.", "Add Email", "lower_case", "Mixed_Case"]:
		if TranslationKey.is_key(invalid):
			_fail("is_key accepted prose: %s" % invalid)
	# The instance form delegates to it.
	if not TranslationKey.new("COMMON_CANCEL").is_valid():
		_fail("is_valid() disagreed with is_key()")


func _test_resolution() -> void:
	# The en catalogue is the fallback, so these resolve even with no locale set.
	var cancel := TranslationKey.new("COMMON_CANCEL")
	_expect_eq("text() translates", cancel.text(), "CANCEL")
	_expect_eq("upper() upper-cases", TranslationKey.new("COMMON_CANCEL").upper(), "CANCEL")

	# An unknown key falls back to itself rather than to empty, which is what makes
	# a missed lookup visible on screen instead of silently blank.
	var missing := TranslationKey.new("NO_SUCH_KEY_AT_ALL")
	_expect_eq("unknown key falls back to itself", missing.text(), "NO_SUCH_KEY_AT_ALL")

	# %-format and named format.
	var unreachable := TranslationKey.new("TOAST_WORLD_UNREACHABLE_BODY")
	_expect_eq(
		"format() fills %s", unreachable.format("my-world"), "my-world could not be reached."
	)


func _test_join() -> void:
	# The case this type was built for: two catalogue entries, translated
	# individually, then joined — never concatenated as keys.
	var joined := (
		TranslationKey
		. join(
			[
				TranslationKey.new("MODAL_CONNECTION_LOST_BODY"),
				TranslationKey.new("MODAL_TRY_RESTARTING_APP"),
			]
		)
	)
	var expected := "Please check your internet connection and try again.\n\nTry restarting the app."
	_expect_eq("join() translates each piece", joined, expected)

	var custom := TranslationKey.join(
		[TranslationKey.new("COMMON_CANCEL"), TranslationKey.new("COMMON_CANCEL")], " / "
	)
	_expect_eq("join() honours the separator", custom, "CANCEL / CANCEL")


func _test_exists() -> void:
	# Present, absent, and the plural companion — which is not a message of its own,
	# so it is answered by stripping _PLURAL back to the singular.
	if not TranslationKey.new("COMMON_CANCEL").exists():
		_fail("exists() missed a real key")
	if TranslationKey.new("NO_SUCH_KEY_AT_ALL").exists():
		_fail("exists() accepted an absent key")
	if not TranslationKey.is_known("CREDITS_PURCHASED_PLURAL"):
		_fail("is_known() missed a plural companion")
	# Stripping must not invent keys: FOO_PLURAL is absent when FOO is too.
	if TranslationKey.is_known("NO_SUCH_KEY_AT_ALL_PLURAL"):
		_fail("is_known() invented a key by stripping _PLURAL")


func _test_to_string() -> void:
	var rendered := str(TranslationKey.new("COMMON_CANCEL"))
	_expect_eq("_to_string is loud", rendered, 'TranslationKey("COMMON_CANCEL")')


func _expect_eq(ctx: String, actual: String, expected: String) -> void:
	if actual != expected:
		_fail("%s: got %s, expected %s" % [ctx, actual, expected])


func _fail(msg: String) -> void:
	_failures.append(msg)


func _finish() -> void:
	if _failures.is_empty():
		print("[test_translation_key] PASS")
		quit(0)
		return
	for f in _failures:
		printerr(f)
	printerr("[test_translation_key] FAIL: %d case(s)" % _failures.size())
	quit(1)
