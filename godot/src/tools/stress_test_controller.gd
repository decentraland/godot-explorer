## Stress Test Controller
##
## Tests scene loading/unloading stability by rapidly teleporting around Goerli Plaza.
## Teleports between the 4 corners of the area (72,-10) to (80,10) multiple times.

extends Node

const GOERLI_REALM = "https://sdk-team-cdn.decentraland.org/ipfs/goerli-plaza-main-latest"

# Teleport positions - corners of Goerli Plaza area
const TELEPORT_POSITIONS = [
	Vector2i(72, -10),  # Bottom-left
	Vector2i(80, -10),  # Bottom-right
	Vector2i(80, 10),  # Top-right
	Vector2i(72, 10),  # Top-left
]

# How many complete rounds to do (each round visits all 4 corners)
const STRESS_TEST_ROUNDS = 5

var current_position_index = 0
var current_round = 0
var is_explorer_ready = false
var teleport_count = 0
var successful_loads = 0
var failed_loads = 0
var start_time = 0.0


func _ready():
	if not Global.cli.stress_test:
		queue_free()
		return

	log_message("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━")
	log_message("✓ Stress Test Controller initialized")
	log_message(
		(
			"✓ Will teleport %d times (%d rounds x %d positions)"
			% [
				STRESS_TEST_ROUNDS * TELEPORT_POSITIONS.size(),
				STRESS_TEST_ROUNDS,
				TELEPORT_POSITIONS.size()
			]
		)
	)
	log_message("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━")

	# Start polling for explorer scene
	var timer = Timer.new()
	timer.wait_time = 0.5
	timer.one_shot = false
	timer.timeout.connect(_check_for_explorer)
	add_child(timer)
	timer.start()


# gdlint:ignore = async-function-name
func _check_for_explorer():
	if is_explorer_ready:
		return

	var current_scene = get_tree().current_scene
	if not current_scene:
		return

	var scene_name = current_scene.scene_file_path.get_file().get_basename()
	if scene_name == "explorer":
		is_explorer_ready = true
		await _start_stress_test()


# gdlint:ignore = async-function-name
func _start_stress_test():
	log_message("✓ Explorer detected - starting stress test...")

	# Wait for initial loading to complete
	await _wait_for_loading_complete()
	await get_tree().create_timer(3.0).timeout

	start_time = Time.get_ticks_msec() / 1000.0

	# Run the stress test loop
	while current_round < STRESS_TEST_ROUNDS:
		for i in range(TELEPORT_POSITIONS.size()):
			current_position_index = i
			var pos = TELEPORT_POSITIONS[i]

			teleport_count += 1
			log_message("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━")
			log_message(
				(
					"[Round %d/%d] Teleport #%d to %s"
					% [current_round + 1, STRESS_TEST_ROUNDS, teleport_count, str(pos)]
				)
			)

			# Teleport
			var load_start = Time.get_ticks_msec()
			Global.teleport_to(pos, GOERLI_REALM)

			# Wait for loading
			var load_success = await _wait_for_loading_with_timeout(30.0)
			var load_time = (Time.get_ticks_msec() - load_start) / 1000.0

			if load_success:
				successful_loads += 1
				log_message(
					(
						"✓ Load complete in %.2fs (success: %d, failed: %d)"
						% [load_time, successful_loads, failed_loads]
					)
				)
			else:
				failed_loads += 1
				log_message(
					(
						"✗ Load TIMEOUT after %.2fs (success: %d, failed: %d)"
						% [load_time, successful_loads, failed_loads]
					)
				)

			# Brief pause between teleports
			await get_tree().create_timer(2.0).timeout

		current_round += 1

	# Finalize
	_finalize_stress_test()


## Wait for loading to complete with timeout
# gdlint:ignore = async-function-name
func _wait_for_loading_with_timeout(timeout_seconds: float) -> bool:
	var start = Time.get_ticks_msec()
	var timeout_ms = timeout_seconds * 1000.0

	# First check if loading screen is visible
	var explorer = Global.get_explorer()
	if not is_instance_valid(explorer):
		return false

	# Loading screen is at UI/Loading in the explorer scene
	var loading_ui = explorer.get_node_or_null("UI/Loading")

	# Wait for loading screen to appear (might take a frame)
	await get_tree().create_timer(0.1).timeout

	# Now wait for it to complete
	while Time.get_ticks_msec() - start < timeout_ms:
		loading_ui = explorer.get_node_or_null("UI/Loading")
		if loading_ui == null or not loading_ui.visible:
			return true
		await get_tree().create_timer(0.2).timeout

	return false


## Wait for loading to complete (with smart detection)
# gdlint:ignore = async-function-name
func _wait_for_loading_complete():
	log_message("✓ Waiting for loading to complete...")

	var explorer = Global.get_explorer()
	if not is_instance_valid(explorer):
		return

	# Loading screen is at UI/Loading in the explorer scene
	var loading_ui = explorer.get_node_or_null("UI/Loading")
	if loading_ui != null and loading_ui.visible:
		await Global.loading_finished
	else:
		await get_tree().create_timer(2.0).timeout


# gdlint:ignore = async-function-name
func _finalize_stress_test():
	var total_time = (Time.get_ticks_msec() / 1000.0) - start_time

	log_message("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━")
	log_message("✅ STRESS TEST COMPLETE!")
	log_message("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━")
	log_message("Total teleports: %d" % teleport_count)
	log_message("Successful loads: %d" % successful_loads)
	log_message("Failed loads: %d" % failed_loads)
	log_message("Success rate: %.1f%%" % (100.0 * successful_loads / max(teleport_count, 1)))
	log_message("Total time: %.1fs" % total_time)
	log_message("Avg time per teleport: %.2fs" % (total_time / max(teleport_count, 1)))
	log_message("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━")

	if failed_loads > 0:
		log_message("⚠️  WARNING: %d loads failed!" % failed_loads)
	else:
		log_message("✓ All loads successful!")

	await get_tree().create_timer(3.0).timeout
	get_tree().quit()


func log_message(message: String):
	print(message)
