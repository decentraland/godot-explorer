//! The statistics a benchmark run reports for each sampled metric.

use serde_json::{json, Map, Value};

/// The metrics summarized, as they are named in a sample.
pub const SUMMARIZED: [&str; 21] = [
    "fps",
    "frame_time_process_ms",
    "frame_time_physics_ms",
    "render_cpu_ms",
    "render_gpu_ms",
    "memory_static_mb",
    "memory_rss_mb",
    "memory_peak_mb",
    "video_mem_mb",
    "texture_mem_mb",
    "buffer_mem_mb",
    "process_rss_mb",
    "resource_count",
    "render_objects_in_frame",
    "physics_active_objects",
    "physics_collision_pairs",
    "physics_island_count",
    "loaded_scene_count",
    "node_count",
    "draw_calls",
    "primitives",
];

#[derive(Debug, PartialEq)]
pub struct Stat {
    pub mean: f64,
    pub min: f64,
    pub p50: f64,
    pub p95: f64,
    pub max: f64,
}

impl Stat {
    /// `None` for no values.
    pub fn of(values: &[f64]) -> Option<Self> {
        if values.is_empty() {
            return None;
        }
        let mut sorted = values.to_vec();
        sorted.sort_by(|a, b| a.total_cmp(b));
        let n = sorted.len();
        Some(Self {
            mean: sorted.iter().sum::<f64>() / n as f64,
            min: sorted[0],
            p50: sorted[n / 2],
            p95: sorted[((n as f64 * 0.95) as usize).min(n - 1)],
            max: sorted[n - 1],
        })
    }

    pub fn to_json(&self) -> Value {
        json!({"mean": self.mean, "min": self.min, "p50": self.p50, "p95": self.p95, "max": self.max})
    }
}

/// Per-metric statistics over all the samples; empty without samples.
pub fn summarize(samples: &[Map<String, Value>]) -> Map<String, Value> {
    let mut out = Map::new();
    if samples.is_empty() {
        return out;
    }
    for key in SUMMARIZED {
        let values: Vec<f64> = samples
            .iter()
            .map(|sample| sample.get(key).and_then(Value::as_f64).unwrap_or(0.0))
            .collect();
        if let Some(stat) = Stat::of(&values) {
            out.insert(key.to_string(), stat.to_json());
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn statistics_are_taken_over_the_sorted_values() {
        let stat = Stat::of(&[4.0, 1.0, 3.0, 2.0, 10.0]).unwrap();
        assert_eq!(stat.min, 1.0);
        assert_eq!(stat.max, 10.0);
        assert_eq!(stat.p50, 3.0);
        assert_eq!(stat.p95, 10.0);
        assert!((stat.mean - 4.0).abs() < 1e-12);
    }

    #[test]
    fn no_values_have_no_statistics() {
        assert!(Stat::of(&[]).is_none());
        assert!(summarize(&[]).is_empty());
    }

    #[test]
    fn a_missing_metric_counts_as_zero() {
        let mut sample = Map::new();
        sample.insert("fps".into(), json!(60.0));
        let summary = summarize(&[sample]);
        assert_eq!(summary["fps"]["mean"], json!(60.0));
        assert_eq!(summary["node_count"]["max"], json!(0.0));
    }
}
