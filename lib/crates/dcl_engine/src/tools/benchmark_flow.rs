//! The CI benchmark run (`--benchmark-report`): visits the benchmark locations in turn, lets the
//! memory settle at each, records its metrics and writes the consolidated CSV before quitting.

use std::sync::atomic::{AtomicU32, Ordering};

use godot::prelude::*;

use crate::engine::local_task::{await_promise, next_frame, sleep};
use crate::fetcher::api::DclNavigator;
use crate::fetcher::globals;
use crate::godot_classes::dcl_global::DclGlobal;
use crate::world::state;

const GOERLI_REALM: &str =
    "https://benchmark-scenes.dclexplorer.com/sdk7-goerli-plaza/power-cube/power-cube/";
const GENESIS_REALM: &str =
    "https://benchmark-scenes.dclexplorer.com/Genesis-Plaza-2025/central-plaza/central-plaza/";

/// Alternating Goerli and Genesis, three rounds, to stress scene loading and unloading.
const LOCATIONS: [(&str, Vector2i, &str); 6] = [
    ("Goerli Plaza", Vector2i::new(72, -10), GOERLI_REALM),
    ("Genesis Plaza", Vector2i::new(0, 0), GENESIS_REALM),
    ("Goerli Plaza Second", Vector2i::new(72, -10), GOERLI_REALM),
    ("Genesis Plaza Second", Vector2i::new(0, 0), GENESIS_REALM),
    ("Goerli Plaza Third", Vector2i::new(72, -10), GOERLI_REALM),
    ("Genesis Plaza Third", Vector2i::new(0, 0), GENESIS_REALM),
];

/// Lets GC and resource cleanup finish before the memory is read.
const SETTLE_SECONDS: f64 = 20.0;
const LOAD_TIMEOUT_SECONDS: f64 = 300.0;

static LOADS_COMPLETED: AtomicU32 = AtomicU32::new(0);

pub async fn run() {
    while state::current().is_none() {
        next_frame().await;
    }
    globals::scene_runner()
        .signals()
        .loading_complete()
        .connect(|_| {
            LOADS_COMPLETED.fetch_add(1, Ordering::Relaxed);
        });
    for (name, parcel, realm) in LOCATIONS {
        let before = LOADS_COMPLETED.load(Ordering::Relaxed);
        godot_print!("Benchmark: going to {name} at {parcel}");
        await_promise(DclNavigator::teleport_to(parcel, realm.into())).await;
        if !wait_for_load(before).await {
            godot_error!("Benchmark: {name} did not finish loading in {LOAD_TIMEOUT_SECONDS}s");
            quit(2).await;
            return;
        }
        sleep(SETTLE_SECONDS).await;
        collect(name).await;
    }
    let report = DclGlobal::singleton().bind().benchmark_report.clone();
    report.bind().generate_consolidated_report();
    godot_print!("Benchmark complete");
    quit(0).await;
}

async fn wait_for_load(completed_before: u32) -> bool {
    let started = godot::classes::Time::singleton().get_ticks_msec();
    while LOADS_COMPLETED.load(Ordering::Relaxed) == completed_before {
        let waited = godot::classes::Time::singleton().get_ticks_msec() - started;
        if waited as f64 > LOAD_TIMEOUT_SECONDS * 1000.0 {
            return false;
        }
        next_frame().await;
    }
    true
}

/// The mesh and material counts are left at zero: nothing counts them since the resource counter
/// addon went away.
async fn collect(name: &str) {
    let parcel = state::parcel_position();
    let realm = globals::realm().bind().get_realm_string();
    let report = DclGlobal::singleton().bind().benchmark_report.clone();
    report.clone().bind_mut().collect_and_store_metrics(
        format!("4_Explorer_{}", name.replace(' ', "_"))
            .as_str()
            .into(),
        format!("{parcel}").as_str().into(),
        realm,
        VarDictionary::new(),
    );
    godot_print!("Benchmark: collected {name} at {parcel}");
}

async fn quit(code: i32) {
    sleep(3.0).await;
    if let Some(mut tree) = DclGlobal::singleton().get_tree_or_null() {
        tree.quit_ex().exit_code(code).done();
    }
}
