use crate::analytics::data_definition::{SegmentEvent, SegmentEventPerformanceMetrics};
use crate::godot_classes::{
    dcl_android_plugin::DclGodotAndroidPlugin,
    dcl_global::DclGlobal,
    dcl_ios_plugin::{DclIosPlugin, DclMobileDeviceInfo},
};

const HICCUP_THRESHOLD_MS: f32 = 50.0;
const FRAME_AMOUNT_TO_MEASURE: usize = 1000;

#[derive(Clone, Copy)]
pub enum MobilePlatform {
    Ios,
    Android,
}

pub struct Frame {
    dt_ms_vec: Vec<f32>,
    hiccups_count: u32,
    hiccups_time_ms: f32,
    sum_dt: f32,
    mobile_platform: Option<MobilePlatform>,
    device_info: Option<DclMobileDeviceInfo>,
}

impl Default for Frame {
    fn default() -> Self {
        Self::new(None, None)
    }
}

impl Frame {
    pub fn new(
        mobile_platform: Option<MobilePlatform>,
        device_info: Option<DclMobileDeviceInfo>,
    ) -> Self {
        Self {
            dt_ms_vec: Vec::new(),
            hiccups_count: 0,
            hiccups_time_ms: 0.0,
            sum_dt: 0.0,
            mobile_platform,
            device_info,
        }
    }

    pub fn set_mobile_info(
        &mut self,
        mobile_platform: Option<MobilePlatform>,
        device_info: Option<DclMobileDeviceInfo>,
    ) {
        self.mobile_platform = mobile_platform;
        self.device_info = device_info;
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

            // Get static device info (collected once at startup)
            let (device_brand, device_model, os_version, total_ram_mb) =
                if let Some(info) = &self.device_info {
                    (
                        if info.device_brand.is_empty() {
                            None
                        } else {
                            Some(info.device_brand.clone())
                        },
                        if info.device_model.is_empty() {
                            None
                        } else {
                            Some(info.device_model.clone())
                        },
                        if info.os_version.is_empty() {
                            None
                        } else {
                            Some(info.os_version.clone())
                        },
                        if info.total_ram_mb >= 0 {
                            Some(info.total_ram_mb as u32)
                        } else {
                            None
                        },
                    )
                } else {
                    (None, None, None, None)
                };

            // Get dynamic mobile metrics (ONLY when event is about to be sent)
            let mobile_metrics = match self.mobile_platform {
                Some(MobilePlatform::Ios) => DclIosPlugin::get_mobile_metrics_internal(),
                Some(MobilePlatform::Android) => DclGodotAndroidPlugin::get_mobile_metrics_internal(),
                None => None,
            };

            let (
                memory_usage,
                device_temperature_celsius,
                device_thermal_state,
                battery_percent,
                charging_state,
            ) = if let Some(metrics) = mobile_metrics {
                (
                    metrics.memory_usage,
                    Some(metrics.device_temperature_celsius),
                    if metrics.device_thermal_state.is_empty() {
                        None
                    } else {
                        Some(metrics.device_thermal_state.clone())
                    },
                    if metrics.battery_percent >= 0.0 {
                        Some(metrics.battery_percent)
                    } else {
                        None
                    },
                    if metrics.charging_state.is_empty() || metrics.charging_state == "unknown" {
                        None
                    } else {
                        Some(metrics.charging_state.clone())
                    },
                )
            } else {
                (-1, None, None, None, None)
            };

            // Get data from DclGlobal singleton (network speed and player count)
            let (network_speed_mbps, player_count) = DclGlobal::try_singleton()
                .map(|global| {
                    let global_bind = global.bind();

                    // Get download speed from content provider
                    let network_speed = {
                        let content_provider = global_bind.content_provider.clone();
                        let download_speed = content_provider.bind().get_download_speed_mbs();
                        if download_speed > 0.0 {
                            Some(download_speed as f32)
                        } else {
                            None
                        }
                    };

                    // Get player count from avatar scene
                    let avatars_count = global_bind.avatars.bind().get_avatars_count();

                    (network_speed, avatars_count)
                })
                .unwrap_or((None, -1));

            // Network type is not available yet
            let network_type: Option<String> = None;

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

                player_count,
                used_jsheap_size: -1,
                memory_usage,

                // Mobile device info (static)
                device_brand,
                device_model,
                os_version,
                total_ram_mb,

                // Mobile metrics (dynamic)
                device_temperature_celsius,
                device_thermal_state,
                battery_percent,
                charging_state,
                network_type,
                network_speed_mbps,
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
