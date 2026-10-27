extends RefCounted

# Event text reaching Sentry must carry no LiveKit room token, JWT or full wallet.
# Exercises the static ProjectMainLoop._scrub_text that _before_send applies to
# `message`; the exception text of logger events cannot be rewritten from GDScript.

const WALLET := "0x1234567890abcdef1234567890abcdef12345678"
const JWT := "eyJhbGciOiJIUzI1NiJ9.eyJyb29tIjoid29ybGQifQ.sig_part-123"

var suite_name := "sentry_before_send"
var method_name := "test_scrub_text_redacts_tokens_and_wallets"
var errors: Array[String] = []
var execution_time_seconds := 0.0


func run() -> bool:
	var start := Time.get_ticks_usec()
	var ok := true
	ok = (
		_expect(
			ProjectMainLoop._scrub_text(
				"wss://livekit.decentraland.org?access_token=" + JWT + "&x=1"
			),
			"wss://livekit.decentraland.org?access_token=[redacted]&x=1",
			"access_token query value"
		)
		and ok
	)
	ok = (
		_expect(
			ProjectMainLoop._scrub_text("token " + JWT + " in text"),
			"token [jwt] in text",
			"bare jwt"
		)
		and ok
	)
	ok = (
		_expect(
			ProjectMainLoop._scrub_text("profile for " + WALLET + " failed"),
			"profile for 0x1234… failed",
			"wallet masked to a prefix"
		)
		and ok
	)
	ok = (
		_expect(
			ProjectMainLoop._scrub_text("0x1234 short hex stays"),
			"0x1234 short hex stays",
			"short hex untouched"
		)
		and ok
	)
	ok = _expect(ProjectMainLoop._scrub_text(""), "", "empty text") and ok
	execution_time_seconds = (Time.get_ticks_usec() - start) / 1_000_000.0
	return ok


func _expect(actual: String, expected: String, label: String) -> bool:
	if actual != expected:
		errors.append("%s: expected '%s', got '%s'" % [label, expected, actual])
		return false
	return true
