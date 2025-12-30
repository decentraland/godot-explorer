## Benchmark Flow Controller
##
## Orchestrates the full benchmark flow through all scenes without modifying production code.
## This controller manages scene transitions, metric collection, and report generation.
##
## Flow: Terms → Lobby → Menu → Explorer (Goerli x3 / Genesis x3 alternating)

extends Node

# Benchmark configuration - alternating between Goerli and Genesis to stress test scene loading/unloading
const GOERLI_PLAZA = {
	"name": "Goerli Plaza",
	"pos": Vector2i(72, -10),
	"realm": "https://sdk-team-cdn.decentraland.org/ipfs/goerli-plaza-main-latest"
}

const GENESIS_PLAZA = {
	"name": "Genesis Plaza",
	"pos": Vector2i(0, 0),
	"realm": "https://realm-provider-ea.decentraland.org/main"
}

var benchmark_locations = [
	# Round 1
	{"name": "Goerli Plaza", "pos": GOERLI_PLAZA.pos, "realm": GOERLI_PLAZA.realm},
	{"name": "Genesis Plaza", "pos": GENESIS_PLAZA.pos, "realm": GENESIS_PLAZA.realm},
	# Round 2
	{"name": "Goerli Plaza Second", "pos": GOERLI_PLAZA.pos, "realm": GOERLI_PLAZA.realm},
	{"name": "Genesis Plaza Second", "pos": GENESIS_PLAZA.pos, "realm": GENESIS_PLAZA.realm},
	# Round 3
	{"name": "Goerli Plaza Third", "pos": GOERLI_PLAZA.pos, "realm": GOERLI_PLAZA.realm},
	{"name": "Genesis Plaza Third", "pos": GENESIS_PLAZA.pos, "realm": GENESIS_PLAZA.realm},
]

var current_location_index = 0
var current_stage = ""
var is_handling_scene = false  # Prevent re-entry

# References
var benchmark_report = null


func _ready():
	if not Global.cli.benchmark_report:
		queue_free()
		return

	benchmark_report = Global.benchmark_report
	if not benchmark_report:
		push_error("BenchmarkReport not found in Global")
		queue_free()
		return

	log_message("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━")
	log_message("✓ Benchmark Flow Controller initialized")
	log_message("✓ Flow: Terms → Lobby → Menu → Explorer locations")
	log_message("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━")

	# Don't connect to tree_changed - use a timer instead to poll for scene changes
	# This avoids duplicate handling from multiple signal emissions
	var timer = Timer.new()
	timer.wait_time = 0.5
	timer.one_shot = false
	timer.timeout.connect(_check_current_scene)
	add_child(timer)
	timer.start()

	# No initial check needed - the Timer will handle it on the first tick


# gdlint:ignore = async-function-name
func _check_current_scene():
	# Prevent re-entry
	if is_handling_scene:
		return

	var current_scene = get_tree().current_scene
	if not current_scene:
		return

	var scene_name = current_scene.scene_file_path.get_file().get_basename()

	# Determine if we should handle this scene and set flags ATOMICALLY
	var should_handle = false
	var handler_name = ""

	match scene_name:
		"terms_and_conditions":
			if current_stage != "terms" and current_stage != "terms_handling":
				should_handle = true
				handler_name = "terms"
				current_stage = "terms_handling"  # Set immediately
		"lobby":
			if current_stage != "lobby" and current_stage != "lobby_handling":
				should_handle = true
				handler_name = "lobby"
				current_stage = "lobby_handling"  # Set immediately
		"menu":
			if current_stage != "menu" and current_stage != "menu_handling":
				should_handle = true
				handler_name = "menu"
				current_stage = "menu_handling"  # Set immediately
		"explorer":
			if current_stage != "explorer" and current_stage != "explorer_handling":
				should_handle = true
				handler_name = "explorer"
				current_stage = "explorer_handling"  # Set immediately

	# If we should handle, set the lock and call the handler
	if should_handle:
		is_handling_scene = true
		match handler_name:
			"terms":
				await handle_terms_scene(current_scene)
			"lobby":
				await handle_lobby_scene(current_scene)
			"menu":
				await handle_menu_scene(current_scene)
			"explorer":
				await handle_explorer_scene(current_scene)
		is_handling_scene = false


func _on_scene_changed():
	if not is_handling_scene:
		call_deferred("_check_current_scene")


## Terms and Conditions Scene
# gdlint:ignore = async-function-name
func handle_terms_scene(scene):
	log_message("✓ Terms and Conditions: Starting benchmark collection...")
	await get_tree().create_timer(2.0).timeout

	await collect_ui_scene_metrics("1_Terms_and_Conditions", "UI Scene")
	log_message("✓ Terms and Conditions benchmark collected")

	# Mark stage as complete
	current_stage = "terms"

	# Auto-accept and proceed
	log_message("✓ Auto-accepting Terms and Conditions...")
	await get_tree().create_timer(1.0).timeout
	scene._on_button_accept_pressed()


## Lobby Scene
# gdlint:ignore = async-function-name
func handle_lobby_scene(scene):
	log_message("✓ Lobby: Starting benchmark collection...")
	await get_tree().create_timer(2.0).timeout

	await collect_ui_scene_metrics("2_Lobby", "UI Scene")
	log_message("✓ Lobby benchmark collected")

	# Create guest account
	log_message("✓ Creating guest account...")
	scene.create_guest_account_if_needed()

	# Update metrics identity
	Global.metrics.update_identity(
		Global.player_identity.get_address_str(), Global.player_identity.is_guest
	)

	# Mark stage as complete
	current_stage = "lobby"

	# Proceed to menu (don't set deep_link_obj yet - that will skip the menu)
	log_message("✓ Auto-proceeding to Menu...")
	await get_tree().create_timer(1.0).timeout
	get_tree().call_deferred("change_scene_to_file", "res://src/ui/components/menu/menu.tscn")


## Menu Scene
# gdlint:ignore = async-function-name, unused-argument
func handle_menu_scene(_scene):
	log_message("✓ Menu: Starting benchmark collection...")
	await get_tree().create_timer(2.0).timeout

	await collect_ui_scene_metrics("3_Menu", "UI Scene")
	log_message("✓ Menu benchmark collected")

	# Now set up for Goerli Plaza (first location) before going to explorer
	log_message("✓ Configuring Goerli Plaza as first location...")
	Global.deep_link_obj.location = benchmark_locations[0].pos
	Global.deep_link_obj.realm = benchmark_locations[0].realm

	# Mark stage as complete
	current_stage = "menu"

	# Proceed to Explorer
	log_message("✓ Auto-proceeding to Explorer...")
	await get_tree().create_timer(1.0).timeout
	get_tree().call_deferred("change_scene_to_file", "res://src/ui/explorer.tscn")


## Explorer Scene
# gdlint:ignore = async-function-name, unused-argument
func handle_explorer_scene(_scene):
	# Wait for loading to complete
	await _wait_for_loading_complete()

	# Wait for scene to stabilize
	await get_tree().create_timer(5.0).timeout

	# Process all benchmark locations in a loop (don't rely on timer for re-detection)
	while current_location_index < benchmark_locations.size():
		# Collect metrics for current location
		var current_pos = Global.get_explorer().parcel_position
		var location_name = benchmark_locations[current_location_index].name

		log_message(
			"✓ Explorer: Collecting benchmark at %s (%s)..." % [str(current_pos), location_name]
		)

		await collect_explorer_metrics(current_pos, location_name)

		log_message("✓ Explorer benchmark collected at %s (%s)" % [str(current_pos), location_name])

		# Move to next location
		current_location_index += 1

		if current_location_index < benchmark_locations.size():
			# Teleport to next location
			var next_loc = benchmark_locations[current_location_index]
			log_message("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━")
			log_message("✓ Moving to next location: %s at %s" % [next_loc.name, next_loc.pos])
			log_message("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━")

			# Wait before teleport
			await get_tree().create_timer(4.0).timeout

			# Teleport and wait for loading to complete
			Global.teleport_to(next_loc.pos, next_loc.realm)
			await _wait_for_loading_complete()

			# Wait for scene to stabilize
			await get_tree().create_timer(5.0).timeout

	# Mark stage as complete (prevents re-entry from timer)
	current_stage = "explorer"

	# All locations tested - finalize
	finalize_benchmark()


## Wait for loading to complete (with smart detection)
## Handles cases where loading screen may not be shown
# gdlint:ignore = async-function-name
func _wait_for_loading_complete():
	log_message("✓ Waiting for Explorer to finish loading...")

	# Check if loading screen is currently visible
	var explorer = Global.get_explorer()
	if not is_instance_valid(explorer):
		return

	var loading_ui = explorer.get_node_or_null("CanvasLayer/LoadingScreen")
	if loading_ui != null and loading_ui.visible:
		# Loading screen is visible, wait for it to complete
		await Global.loading_finished
	else:
		# Loading screen not visible - wait a bit for scenes to stabilize
		# This handles cases where scenes were already loaded or loading was very fast
		await get_tree().create_timer(2.0).timeout


## Collect metrics for UI scenes (Terms, Lobby, Menu)
func collect_ui_scene_metrics(test_name: String, location: String):
	var resource_data = {
		"total_meshes": 0,
		"total_materials": 0,
		"mesh_rid_count": 0,
		"material_rid_count": 0,
		"mesh_hash_count": 0,
		"potential_dedup_count": 0,
		"mesh_savings_percent": 0.0
	}

	benchmark_report.collect_and_store_metrics(test_name, location, "", resource_data)


## Collect metrics for Explorer scenes
func collect_explorer_metrics(current_pos: Vector2i, location_name: String):
	# Count resources
	var counter = (
		load("res://addons/dcl_dev_tools/dev_tools/resource_counter/resource_counter.gd").new()
	)
	add_child(counter)
	counter.count(get_tree().get_root().get_node("scene_runner"))

	# Calculate resource data
	var resource_data = {
		"total_meshes": counter.meshes.size(),
		"total_materials": counter.materials.size(),
		"mesh_rid_count": 0,
		"material_rid_count": 0,
		"mesh_hash_count": 0,
		"potential_dedup_count": 0,
		"mesh_savings_percent": 0.0
	}

	# Calculate RID counts
	var mesh_rid_map = {}
	for mesh in counter.meshes:
		if mesh:
			var mesh_rid = mesh.get_rid()
			mesh_rid_map[mesh_rid] = true
	resource_data["mesh_rid_count"] = mesh_rid_map.size()

	var material_rid_map = {}
	for material in counter.materials:
		if material:
			var material_rid = material.get_rid()
			material_rid_map[material_rid] = true
	resource_data["material_rid_count"] = material_rid_map.size()

	# Collect metrics
	var test_name = "4_Explorer_" + location_name.replace(" ", "_")
	var location = str(current_pos)
	var realm = Global.realm.get_realm_string()

	benchmark_report.collect_and_store_metrics(test_name, location, realm, resource_data)

	counter.queue_free()


## Finalize benchmark and generate reports
# gdlint:ignore = async-function-name
func finalize_benchmark():
	log_message("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━")
	log_message("✓ All benchmark locations completed!")
	log_message("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━")

	log_message("Generating consolidated benchmark report...")
	benchmark_report.generate_consolidated_report()

	var user_dir = OS.get_user_data_dir()
	log_message("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━")
	log_message("✅ BENCHMARK COMPLETE!")
	log_message("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━")
	log_message("CSV report saved to: " + user_dir + "/output/benchmark_report.csv")
	log_message("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━")

	await get_tree().create_timer(3.0).timeout
	get_tree().quit()


## Logging helper
func log_message(message: String):
	print(message)
