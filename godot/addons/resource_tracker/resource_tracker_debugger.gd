class_name ResourceTrackerDebugger

enum ResourceTrackerState {
	STARTED = 0,
	DOWNLOADING = 1,
	DOWNLOADED = 2,
	LOADING = 3,
	FAILED = 4,
	FINISHED = 5,
}


static func report_resource(
	hash_id: String, state: ResourceTrackerState, progress: String, size: String, metadata: String
):
	EngineDebugger.send_message(
		"resource_tracker:report", [hash_id, state, progress, size, metadata]
	)
	return true


static func get_resource_state_string(state: ResourceTrackerState) -> String:
	match state:
		ResourceTrackerState.STARTED:
			return "Started"
		ResourceTrackerState.DOWNLOADING:
			return "Downloading"
		ResourceTrackerState.DOWNLOADED:
			return "Downloaded"
		ResourceTrackerState.LOADING:
			return "Loading"
		ResourceTrackerState.FAILED:
			return "Failed"
		ResourceTrackerState.FINISHED:
			return "Finished"
		_:
			return "Unknown State"
