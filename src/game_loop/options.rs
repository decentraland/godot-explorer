//! Configuration for a `cargo run -- game-loop` invocation.

use std::path::PathBuf;
use std::time::Duration;

pub struct GameLoopOptions {
    /// Scenario numbers to run in sequence (each matches the `SCENARIOS` registry in
    /// `game_loop_runner.gd`). `--scenario all` expands to [`super::ALL_SCENARIOS`].
    pub scenarios: Vec<u32>,
    /// Optional golden-mode seed — forces a deterministic guest via a seed-derived
    /// anchor (standard Play-as-Guest flow). Reachable ONLY through this harness.
    pub guest_seed: Option<String>,
    /// Explicit adb device serial; when None the single connected device is used.
    pub device: Option<String>,
    /// Install (`-r`) the exported debug APK before launching (done once, up front).
    pub install: bool,
    /// `pm clear` the app before each scenario (fresh guest / clean state).
    pub clear: bool,
    /// Max time to wait for each scenario's RESULT line before giving up.
    pub timeout: Duration,
    /// Override the pulled-screenshot dir. Honored only for a single scenario; a
    /// multi-scenario run always derives `target/gameloop/scenario_<N>` per scenario.
    pub out_override: Option<PathBuf>,
    /// Override the golden dir. Same single-scenario-only rule as `out_override`.
    pub golden_override: Option<PathBuf>,
    /// Copy each scenario's screenshots into its golden dir as the new baseline.
    pub bless: bool,
    /// Per-image similarity floor (0..1) for a golden to pass.
    pub threshold: f64,
}
