class_name CaptureResult
extends RefCounted

## Typed result of a deterministic viewport capture (GameLoopScenario.async_capture_viewport)
## — replaces an untyped {ok, path, size, distinct_colors, detail} Dictionary.

var succeeded: bool
## Absolute filesystem path of the saved PNG (empty on failure).
var path: String
## Frame dimensions as "WxH".
var size: String
## Smoke metric: distinct colors on the aspect-preserved downscale (MAX over sampled frames).
var distinct_colors: int
## Failure reason, set only when succeeded == false.
var detail: String


## Failed-capture constructor (succeeded stays false).
static func failure(p_detail: String) -> CaptureResult:
	var result := CaptureResult.new()
	result.detail = p_detail
	return result
