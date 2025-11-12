extends Control

signal accepted

@onready var v_box_container_terms: VBoxContainer = %VBoxContainer_Terms
@onready var control_separator: Control = %Control_Separator
@onready var button_accept: Button = %Button_Accept
@onready var timer: Timer = $Timer
@onready var spinner: TextureProgressBar = %Spinner


func _ready():
	if Global.cli.benchmark_report:
		print("✓ Terms and Conditions: Starting benchmark collection...")
		_collect_benchmark()


func _collect_benchmark():
	# Wait for scene to stabilize
	await get_tree().create_timer(2.0).timeout

	var benchmark_report = Global.benchmark_report
	if not benchmark_report:
		push_warning("BenchmarkReport not found in Global")
		# Auto-proceed anyway
		_auto_accept_for_benchmark()
		return

	var resource_data = {
		"total_meshes": 0,
		"total_materials": 0,
		"mesh_rid_count": 0,
		"material_rid_count": 0,
		"mesh_hash_count": 0,
		"potential_dedup_count": 0,
		"mesh_savings_percent": 0.0
	}

	benchmark_report.collect_and_store_metrics(
		"1_Terms_and_Conditions",
		"UI Scene",
		"",
		resource_data
	)
	benchmark_report.generate_individual_report()

	print("✓ Terms and Conditions benchmark collected")

	# Auto-proceed to Lobby
	await get_tree().create_timer(1.0).timeout
	_auto_accept_for_benchmark()


func _auto_accept_for_benchmark():
	print("✓ Auto-accepting Terms and Conditions for benchmark flow...")
	_on_button_accept_pressed()


func _on_check_box_terms_and_privacy_toggled(toggled_on: bool) -> void:
	%Button_Accept.disabled = !toggled_on


func _on_rich_text_label_meta_clicked(meta: Variant) -> void:
	Global.open_webview_url(meta)


func _on_button_accept_pressed() -> void:
	Global.metrics.track_screen_viewed("ACCEPT_EULA", "")
	Global.metrics.track_click_button("accept", "ACCEPT_EULA", "")
	Global.metrics.flush()
	spinner.show()
	control_separator.hide()
	button_accept.hide()
	timer.start()


func _on_button_reject_pressed() -> void:
	v_box_container_terms.hide()
	get_tree().quit()


func _on_control_gui_input(event: InputEvent) -> void:
	if event is InputEventScreenTouch:
		if event.pressed:
			%CheckBox_TermsAndPrivacy.button_pressed = !%CheckBox_TermsAndPrivacy.button_pressed


func _on_timer_timeout() -> void:
	Global.get_config().terms_and_conditions_version = Global.TERMS_AND_CONDITIONS_VERSION
	Global.get_config().save_to_settings_file()
	accepted.emit()
	if !Global.is_xr():
		get_tree().change_scene_to_file("res://src/ui/components/auth/lobby.tscn")
