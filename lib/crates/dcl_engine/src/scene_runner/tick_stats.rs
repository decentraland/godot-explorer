//! Per-scene tick timing kept for creators (scene stats panel, console
//! warning), the debug hub and the GP benchmark. Cheap to update every frame;
//! percentiles are only computed when a getter or the warning check runs.

use std::collections::VecDeque;
use std::time::Instant;

/// Samples kept per scene (ticks and rendered frames).
pub const WINDOW: usize = 120;

#[derive(Clone, Copy, Default, Debug)]
pub struct TickSample {
    /// Pure scene-thread time of the tick (systems + CRDT serialization), µs.
    pub js_tick_us: u32,
    /// Kick -> output received on the main thread, µs (includes thread wake-ups).
    pub round_trip_us: u32,
    /// Main-thread time applying the output to Godot nodes, µs.
    pub apply_us: u32,
    /// Time the collect phase blocked for this scene in its frame, µs.
    pub wait_us: u32,
}

#[derive(Default)]
pub struct SceneTickStats {
    ticks: VecDeque<TickSample>,
    /// Per rendered frame: bit 0 = missed the deadline, bit 1 = skipped (throttled).
    frames: VecDeque<u8>,
    pub ticks_total: u64,
    pub missed_frames: u64,
    pub skipped_ticks: u64,
    pub apply_overruns: u64,
    pub last_warning_at: Option<Instant>,
    pub frames_since_check: u32,
}

impl SceneTickStats {
    pub fn on_output_received(&mut self, js_tick_us: u32, round_trip_us: u32) {
        if self.ticks.len() == WINDOW {
            self.ticks.pop_front();
        }
        self.ticks.push_back(TickSample {
            js_tick_us,
            round_trip_us,
            apply_us: 0,
            wait_us: 0,
        });
        self.ticks_total += 1;
    }

    pub fn on_apply_done(&mut self, apply_us: u32, overrun: bool) {
        if let Some(last) = self.ticks.back_mut() {
            last.apply_us = apply_us;
        }
        if overrun {
            self.apply_overruns += 1;
        }
    }

    /// Once per rendered frame for a foreground scene.
    pub fn on_frame(&mut self, wait_us: u32, missed: bool, skipped: bool) {
        if wait_us > 0 && !missed {
            if let Some(last) = self.ticks.back_mut() {
                last.wait_us = last.wait_us.max(wait_us);
            }
        }
        if self.frames.len() == WINDOW {
            self.frames.pop_front();
        }
        self.frames
            .push_back((missed as u8) | ((skipped as u8) << 1));
        if missed {
            self.missed_frames += 1;
        }
        if skipped {
            self.skipped_ticks += 1;
        }
        self.frames_since_check += 1;
    }

    pub fn window_ticks(&self) -> usize {
        self.ticks.len()
    }

    pub fn window_frames(&self) -> usize {
        self.frames.len()
    }

    pub fn last(&self) -> Option<&TickSample> {
        self.ticks.back()
    }

    /// `q` in 0..=1 over the tick window; 0 when empty.
    pub fn percentile(&self, pick: impl Fn(&TickSample) -> u32, q: f32) -> u32 {
        if self.ticks.is_empty() {
            return 0;
        }
        let mut values: Vec<u32> = self.ticks.iter().map(pick).collect();
        let idx = (((values.len() - 1) as f32) * q.clamp(0.0, 1.0)).round() as usize;
        let (_, value, _) = values.select_nth_unstable(idx);
        *value
    }

    /// Percent of the frame window drawn without a fresh update from this scene.
    pub fn missed_pct(&self) -> f32 {
        if self.frames.is_empty() {
            return 0.0;
        }
        let missed = self.frames.iter().filter(|f| *f & 1 != 0).count();
        missed as f32 * 100.0 / self.frames.len() as f32
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn percentile_and_missed_pct_over_the_window() {
        let mut stats = SceneTickStats::default();
        assert_eq!(stats.percentile(|s| s.js_tick_us, 0.95), 0);
        assert_eq!(stats.missed_pct(), 0.0);

        // 200 ticks: only the last WINDOW are kept, so the p50 sits in the
        // upper range and the p95 near the top of the retained samples.
        for i in 0..200u32 {
            stats.on_output_received(i * 100, i);
            stats.on_apply_done(i, false);
        }
        assert_eq!(stats.window_ticks(), WINDOW);
        assert_eq!(stats.ticks_total, 200);
        let p50 = stats.percentile(|s| s.js_tick_us, 0.5);
        let p95 = stats.percentile(|s| s.js_tick_us, 0.95);
        assert!((13_000..=14_500).contains(&p50), "p50={p50}");
        assert!((19_000..=19_900).contains(&p95), "p95={p95}");
        assert_eq!(stats.percentile(|s| s.apply_us, 1.0), 199);

        // 150 frames, every 5th one missed: the window holds the last 120.
        for i in 0..150u32 {
            stats.on_frame(500, i % 5 == 0, false);
        }
        assert_eq!(stats.window_frames(), WINDOW);
        assert_eq!(stats.missed_frames, 30);
        assert!(
            (stats.missed_pct() - 20.0).abs() < 0.01,
            "{}",
            stats.missed_pct()
        );
        assert_eq!(stats.frames_since_check, 150);
        // The wait is attached to the newest tick sample, only on frames that hit.
        assert_eq!(stats.last().map(|s| s.wait_us), Some(500));
    }
}
