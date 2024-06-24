use godot::{engine::EngineDebugger, prelude::*};

#[repr(i32)]
pub enum ResourceTrackerState {
    Started = 0,
    Downloading = 1,
    Downloaded = 2,
    Loading = 3,
    Failed = 4,
    Finished = 5,
}

fn send_resource_tracker_message(
    state: ResourceTrackerState,
    hash_id: &String,
    progress: &str,
    size: &str,
    metadata: &str,
) {
    if !EngineDebugger::singleton().is_active() {
        return;
    }
    let mut array = Array::new();
    array.push(hash_id.to_variant());
    array.push((state as i32).to_variant());
    array.push(progress.to_variant());
    array.push(size.to_variant());
    array.push(metadata.to_variant());
    EngineDebugger::singleton().send_message("resource_tracker:report".into_godot(), array);
}

pub fn report_download_speed(speed: f64) {
    if !EngineDebugger::singleton().is_active() {
        return;
    }
    let speed_str = format!("{:.2}mb/s", speed / 1024.0 / 1024.0);
    let mut array = Array::new();
    array.push(speed_str.to_variant());
    EngineDebugger::singleton().send_message("resource_tracker:report_speed".into_godot(), array);
}

pub fn report_resource_start(hash_id: &String) {
    send_resource_tracker_message(ResourceTrackerState::Started, hash_id, "0%", "0mb", "");
}

pub fn report_resource_downloading(hash_id: &String, current_size: u64, speed: f64) {
    let size_str = format!("{:.2}mb", (current_size as f64) / 1024.0 / 1024.0);
    let speed_str = format!("{:.2}kb/s", speed / 1024.0);
    send_resource_tracker_message(
        ResourceTrackerState::Downloading,
        hash_id,
        "",
        &size_str,
        &speed_str,
    );
}

pub fn report_resource_download_done(hash_id: &String, current_size: u64) {
    let size_str = format!("{:.2}mb", (current_size as f64) / 1024.0 / 1024.0);
    send_resource_tracker_message(ResourceTrackerState::Downloaded, hash_id, "", &size_str, "");
}

pub fn report_resource_error(hash_id: &String, error: &String) {
    send_resource_tracker_message(ResourceTrackerState::Failed, hash_id, "Failed", "", error);
}

pub fn report_resource_loading(hash_id: &String, progress: &String, detail: &String) {
    send_resource_tracker_message(ResourceTrackerState::Loading, hash_id, progress, "", detail);
}

pub fn report_resource_loaded(hash_id: &String) {
    send_resource_tracker_message(ResourceTrackerState::Finished, hash_id, "Done", "", "");
}
