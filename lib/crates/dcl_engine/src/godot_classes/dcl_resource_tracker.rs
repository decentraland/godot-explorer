use godot::{classes::EngineDebugger, prelude::*};
use std::collections::HashMap;
use std::sync::Mutex;
use std::time::{Duration, Instant};

#[repr(i32)]
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum ResourceTrackerState {
    Started = 0,
    Downloading = 1,
    Downloaded = 2,
    Loading = 3,
    Failed = 4,
    Finished = 5,
    Deleted = 6,
    Timeout = 7,
}

struct ResourceActivity {
    last_activity: Instant,
    last_state: ResourceTrackerState,
    is_timed_out: bool,
    resource_type: String,
}

static ACTIVITY_TRACKER: Mutex<Option<HashMap<String, ResourceActivity>>> = Mutex::new(None);

const TIMEOUT_DURATION: Duration = Duration::from_secs(20);

fn get_or_init_tracker() -> std::sync::MutexGuard<'static, Option<HashMap<String, ResourceActivity>>>
{
    let mut guard = ACTIVITY_TRACKER.lock().unwrap();
    if guard.is_none() {
        *guard = Some(HashMap::new());
    }
    guard
}

fn update_activity(hash_id: &String, state: ResourceTrackerState, resource_type: Option<&str>) {
    let mut tracker = get_or_init_tracker();
    if let Some(map) = tracker.as_mut() {
        if let Some(activity) = map.get_mut(hash_id) {
            activity.last_activity = Instant::now();
            activity.last_state = state;
            activity.is_timed_out = false;
            if let Some(rt) = resource_type {
                activity.resource_type = rt.to_string();
            }
        } else {
            map.insert(
                hash_id.clone(),
                ResourceActivity {
                    last_activity: Instant::now(),
                    last_state: state,
                    is_timed_out: false,
                    resource_type: resource_type.unwrap_or("").to_string(),
                },
            );
        }
    }
}

fn get_resource_type(hash_id: &String) -> String {
    let tracker = get_or_init_tracker();
    if let Some(map) = tracker.as_ref() {
        if let Some(activity) = map.get(hash_id) {
            return activity.resource_type.clone();
        }
    }
    String::new()
}

fn remove_activity(hash_id: &String) {
    let mut tracker = get_or_init_tracker();
    if let Some(map) = tracker.as_mut() {
        map.remove(hash_id);
    }
}

/// Check for timed out resources and report them.
/// Returns the list of resources that just timed out.
pub fn check_and_report_timeouts() -> Vec<String> {
    if !EngineDebugger::singleton().is_active() {
        return Vec::new();
    }

    // Collect timed out resources with their types while holding the lock
    let mut timed_out: Vec<(String, String)> = Vec::new();
    let now = Instant::now();

    {
        let mut tracker = get_or_init_tracker();
        if let Some(map) = tracker.as_mut() {
            for (hash_id, activity) in map.iter_mut() {
                // Only check resources that are in active states (not finished/failed/deleted)
                let is_active_state = matches!(
                    activity.last_state,
                    ResourceTrackerState::Started
                        | ResourceTrackerState::Downloading
                        | ResourceTrackerState::Downloaded
                        | ResourceTrackerState::Loading
                );

                if is_active_state && !activity.is_timed_out {
                    if now.duration_since(activity.last_activity) >= TIMEOUT_DURATION {
                        activity.is_timed_out = true;
                        timed_out.push((hash_id.clone(), activity.resource_type.clone()));
                    }
                }
            }
        }
    } // Lock is released here

    // Report timeouts after releasing the lock
    let hash_ids: Vec<String> = timed_out
        .iter()
        .map(|(hash_id, resource_type)| {
            send_resource_tracker_message(
                ResourceTrackerState::Timeout,
                hash_id,
                "Timeout",
                "",
                "",
                resource_type,
            );
            hash_id.clone()
        })
        .collect();

    hash_ids
}

fn send_resource_tracker_message(
    state: ResourceTrackerState,
    hash_id: &String,
    progress: &str,
    size: &str,
    metadata: &str,
    resource_type: &str,
) {
    if !EngineDebugger::singleton().is_active() {
        return;
    }
    let mut array = Array::new();
    array.push(&hash_id.to_variant());
    array.push(&(state as i32).to_variant());
    array.push(&progress.to_variant());
    array.push(&size.to_variant());
    array.push(&metadata.to_variant());
    array.push(&resource_type.to_variant());
    EngineDebugger::singleton().send_message("resource_tracker:report", &array);
}

pub fn report_download_speed(speed: f64) {
    if !EngineDebugger::singleton().is_active() {
        return;
    }
    let speed_str = format!("{:.2}mb/s", speed / 1024.0 / 1024.0);
    let mut array = Array::new();
    array.push(&speed_str.to_variant());
    EngineDebugger::singleton().send_message("resource_tracker:report_speed", &array);
}

pub fn report_resource_start(hash_id: &String, resource_type: &str) {
    update_activity(hash_id, ResourceTrackerState::Started, Some(resource_type));
    send_resource_tracker_message(
        ResourceTrackerState::Started,
        hash_id,
        "0%",
        "0mb",
        "",
        resource_type,
    );
}

pub fn report_resource_downloading(hash_id: &String, current_size: u64, speed: f64) {
    update_activity(hash_id, ResourceTrackerState::Downloading, None);
    let size_str = format!("{:.2}mb", (current_size as f64) / 1024.0 / 1024.0);
    let speed_str = format!("{:.2}kb/s", speed / 1024.0);
    let resource_type = get_resource_type(hash_id);
    send_resource_tracker_message(
        ResourceTrackerState::Downloading,
        hash_id,
        "",
        &size_str,
        &speed_str,
        &resource_type,
    );
}

pub fn report_resource_download_done(hash_id: &String, current_size: u64) {
    update_activity(hash_id, ResourceTrackerState::Downloaded, None);
    let size_str = format!("{:.2}mb", (current_size as f64) / 1024.0 / 1024.0);
    let resource_type = get_resource_type(hash_id);
    send_resource_tracker_message(
        ResourceTrackerState::Downloaded,
        hash_id,
        "",
        &size_str,
        "",
        &resource_type,
    );
}

pub fn report_resource_error(hash_id: &String, error: &String) {
    let resource_type = get_resource_type(hash_id);
    remove_activity(hash_id);
    send_resource_tracker_message(
        ResourceTrackerState::Failed,
        hash_id,
        "Failed",
        "",
        error,
        &resource_type,
    );
}

pub fn report_resource_loading(hash_id: &String, progress: &String, detail: &String) {
    update_activity(hash_id, ResourceTrackerState::Loading, None);
    let resource_type = get_resource_type(hash_id);
    send_resource_tracker_message(
        ResourceTrackerState::Loading,
        hash_id,
        progress,
        "",
        detail,
        &resource_type,
    );
}

pub fn report_resource_loaded(hash_id: &String) {
    let resource_type = get_resource_type(hash_id);
    remove_activity(hash_id);
    send_resource_tracker_message(
        ResourceTrackerState::Finished,
        hash_id,
        "Done",
        "",
        "",
        &resource_type,
    );
}

pub fn report_resource_deleted(hash_id: &String) {
    let resource_type = get_resource_type(hash_id);
    remove_activity(hash_id);
    send_resource_tracker_message(
        ResourceTrackerState::Deleted,
        hash_id,
        "",
        "",
        "",
        &resource_type,
    );
}
