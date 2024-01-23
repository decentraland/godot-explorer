extends Node
# This script is a timing approach for showing the loading screen and report the progress.
# Can be changed for a more robust, taking into account the frame rate of the scenes,
# and waiting for some frames and verifying the GLTF Container State and more...

@export var loading_screen : Control 

var current_pending_promises: int = 0
var scenes_metadata_loaded: bool = false
var wait_time = 2.0
var waiting_new_scene_load_report = true

# Called when the node enters the scene tree for the first time.
func _ready():
	Global.scene_fetcher.report_new_load.connect(_report_scene_new_load)

func _report_scene_new_load(done: bool):
	scenes_metadata_loaded = done
	if done == false: # start
		wait_time = 4.0
		enable_loading_screen()
	else:
		wait_time = 2.0

	waiting_new_scene_load_report = false
	
func enable_loading_screen():
	loading_screen.show()
	loading_screen.set_physics_process(true)
	scenes_metadata_loaded = false
	wait_time = 4.0
	loading_screen.set_progress(0.0)
	waiting_new_scene_load_report = true
	
func hide_loading_screen():
	loading_screen.hide()
	loading_screen.set_physics_process(false)

func _physics_process(delta):
	var pending_promises := Global.content_provider.get_pending_promises().size()

	current_pending_promises = maxi(pending_promises, current_pending_promises)
	
	if waiting_new_scene_load_report: return

	if wait_time > 0.0 or scenes_metadata_loaded == false or current_pending_promises == 0:
		# We wait 2 seconds, to wait to queue all the assets...
		wait_time = maxf(wait_time - delta, 0.0)
		
		# We fake 20% for the metadata loading in 2 seconds
		var new_progress = minf(loading_screen.progress + delta / 4.0 * 20.0, 20.0)
		loading_screen.set_progress(new_progress)
	else:
		# Other 80% are the resources downloaded
		var resolved_promises = current_pending_promises - pending_promises
				
		var current_progress: int = int(float(resolved_promises) / float(current_pending_promises) * 80.0) + 20
		loading_screen.set_progress(current_progress)
		
		if current_progress == 100:
			hide_loading_screen()
