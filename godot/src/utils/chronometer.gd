class_name Chronometer
extends RefCounted

const SECONDS_CONVERSION = 1000000

var started_at: int = 0


func restart(message: String):
	started_at = Time.get_ticks_usec()
	if message:
		print(float(started_at) / SECONDS_CONVERSION, " :: ", message)


func lap(message: String):
	var time = Time.get_ticks_usec()
	var time_elapsed = float(time - started_at) / SECONDS_CONVERSION
	if message:
		print(float(time) / SECONDS_CONVERSION, "[+%f s]" % time_elapsed, " :: ", message)
	started_at = time


func elapsed(message: String):
	var time = Time.get_ticks_usec()
	var time_elapsed = float(time - started_at) / SECONDS_CONVERSION
	print(float(time) / SECONDS_CONVERSION, "[+%f s]" % time_elapsed, " :: ", message)
