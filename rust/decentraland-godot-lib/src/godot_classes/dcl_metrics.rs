use godot::prelude::*;

struct FixedCircularBuffer<T, const N: usize> {
    buffer: [T; N],
    index: usize,
}

impl<T, const N: usize> FixedCircularBuffer<T, N> {
    pub fn push(&mut self, value: T) {
        self.buffer[self.index] = value;
        self.index = (self.index + 1) % N;
    }

    pub fn tail(&self) -> &T {
        &self.buffer[self.index]
    }

    pub fn is_next_restart(&self) -> bool {
        self.index == 0
    }
}

#[derive(GodotClass)]
#[class(base=Node)]
pub struct DclMetrics {
    dt_1000_vec: FixedCircularBuffer<f32, 1000>,
    hiccups_1000: u32,
    sum_dt_1000: f32,

    hiccups_total: u32,
    frames_total: u32,
    sum_dt_total: f32,
    #[base]
    _base: Base<Node>,
}

const HICCUP_THRESHOLD_MS: f32 = 0.05;

#[godot_api]
impl INode for DclMetrics {
    fn init(base: Base<Node>) -> Self {
        DclMetrics {
            dt_1000_vec: FixedCircularBuffer {
                buffer: [0.0; 1000],
                index: 0,
            },
            hiccups_1000: 0,
            sum_dt_1000: 0.0,

            hiccups_total: 0,
            sum_dt_total: 0.0,
            frames_total: 0,
            _base: base,
        }
    }
    fn process(&mut self, dt: f64) {
        // pop tail frame
        if *self.dt_1000_vec.tail() > HICCUP_THRESHOLD_MS {
            self.hiccups_1000 -= 1;
        }

        // push frame
        let dt = dt as f32;
        self.frames_total += 1;
        self.sum_dt_1000 += dt;
        self.dt_1000_vec.push(dt);
        if dt > HICCUP_THRESHOLD_MS {
            self.hiccups_1000 += 1;
        }

        // when 1000 frames is just collected
        if self.dt_1000_vec.is_next_restart() {
            self.hiccups_total += self.hiccups_1000;
            self.sum_dt_total += self.sum_dt_1000;

            // Report
            let avg_dt_1000 = self.sum_dt_1000 / 1000.0;
            let avg_dt_total = self.sum_dt_total / self.frames_total as f32;
            let porc_hiccups_1000 = (self.hiccups_1000 as f32 / 1000.0) * 100.0;
            let porc_hiccups_total = (self.hiccups_total as f32 / self.frames_total as f32) * 100.0;

            self.sum_dt_1000 = 0.0;

            tracing::info!(
                "Metrics: avg_dt_1000: {:.2}ms, avg_dt_total: {:.2}ms, hiccups_1000: {:.2}%, hiccups_total: {:.2}%",
                avg_dt_1000 * 1000.0,
                avg_dt_total * 1000.0,
                porc_hiccups_1000,
                porc_hiccups_total
            );
        }
    }
}
