use crate::analytics::data_definition::{SegmentEvent, SegmentEventPerformanceMetrics};

const HICCUP_THRESHOLD_MS: f32 = 50.0;
const FRAME_AMOUNT_TO_MEASURE: usize = 1000;

pub struct Frame {
    dt_ms_vec: Vec<f32>,
    hiccups_count: u32,
    hiccups_time_ms: f32,
    sum_dt: f32,
}

impl Default for Frame {
    fn default() -> Self {
        Self::new()
    }
}

impl Frame {
    pub fn new() -> Self {
        Self {
            dt_ms_vec: Vec::new(),
            hiccups_count: 0,
            hiccups_time_ms: 0.0,
            sum_dt: 0.0,
        }
    }

    pub fn process(&mut self, dt_ms: f32) -> Option<SegmentEvent> {
        self.sum_dt += dt_ms;
        self.dt_ms_vec.push(dt_ms);
        if dt_ms > HICCUP_THRESHOLD_MS {
            self.hiccups_count += 1;
            self.hiccups_time_ms += dt_ms;
        }

        if self.dt_ms_vec.len() >= FRAME_AMOUNT_TO_MEASURE {
            // All the dt >= 0
            self.dt_ms_vec.sort_by(|a, b| a.partial_cmp(b).unwrap());

            let n_samples = self.dt_ms_vec.len();
            let median_frame_time = self.dt_ms_vec[n_samples / 2];
            let p1_frame_time = self.dt_ms_vec[n_samples / 100];
            let p5_frame_time = self.dt_ms_vec[(n_samples * 5) / 100];
            let p10_frame_time = self.dt_ms_vec[(n_samples * 10) / 100];
            let p20_frame_time = self.dt_ms_vec[(n_samples * 20) / 100];
            let p50_frame_time = self.dt_ms_vec[(n_samples * 50) / 100];
            let p75_frame_time = self.dt_ms_vec[(n_samples * 75) / 100];
            let p80_frame_time = self.dt_ms_vec[(n_samples * 80) / 100];
            let p90_frame_time = self.dt_ms_vec[(n_samples * 90) / 100];
            let p95_frame_time = self.dt_ms_vec[(n_samples * 95) / 100];
            let p99_frame_time = self.dt_ms_vec[(n_samples * 99) / 100];

            let event = SegmentEvent::PerformanceMetrics(SegmentEventPerformanceMetrics {
                samples: n_samples as u32,
                total_time: self.sum_dt,
                hiccups_in_thousand_frames: self.hiccups_count, // TODO: if FRAME_AMOUNT_TO_MEASURE is != 1000, this be measured in a different way
                hiccups_time: self.hiccups_time_ms / 1000.0,
                min_frame_time: *self.dt_ms_vec.first().unwrap(),
                max_frame_time: *self.dt_ms_vec.last().unwrap(),
                mean_frame_time: self.sum_dt / n_samples as f32,
                median_frame_time,
                p1_frame_time,
                p5_frame_time,
                p10_frame_time,
                p20_frame_time,
                p50_frame_time,
                p75_frame_time,
                p80_frame_time,
                p90_frame_time,
                p95_frame_time,
                p99_frame_time,

                // These will be populated by metrics.rs
                player_count: -1,
                used_jsheap_size: -1,
                memory_usage: -1,
                device_brand: None,
                device_model: None,
                os_version: None,
                total_ram_mb: None,
                device_temperature_celsius: None,
                device_thermal_state: None,
                battery_percent: None,
                charging_state: None,
                network_type: None,
                network_speed_peak_mbps: None,
                network_used_last_minute_mb: None,
            });

            self.dt_ms_vec.resize(0, 0.0);
            self.hiccups_count = 0;
            self.hiccups_time_ms = 0.0;
            self.sum_dt = 0.0;

            Some(event)
        } else {
            None
        }
    }
}
