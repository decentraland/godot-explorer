/// Event taxonomy, sent as the `source` tag.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "ffi", derive(uniffi::Enum))]
pub enum ReportSource {
    Crash,
    Capture,
    Scene,
    RustApp,
    RustDep,
    Engine,
    SceneCrash,
    ExitReason,
    Feedback,
    FreedInstance,
}

impl ReportSource {
    /// Value of the `source` tag.
    pub fn tag(self) -> &'static str {
        match self {
            Self::Crash => "crash",
            Self::Capture => "capture",
            Self::Scene => "scene",
            Self::RustApp => "rust_app",
            Self::RustDep => "rust_dep",
            Self::Engine => "engine",
            Self::SceneCrash => "scene_crash",
            Self::ExitReason => "exit_reason",
            Self::Feedback => "feedback",
            Self::FreedInstance => "freed_instance",
        }
    }

    /// Fraction kept once the remote rate has passed.
    pub fn keep_rate(self) -> f64 {
        match self {
            Self::RustDep => 0.05,
            Self::Scene | Self::Engine => 0.01,
            _ => 1.0,
        }
    }

    /// Skips the remote sample-rate roll (rate 0 still drops it).
    pub fn remote_rate_exempt(self) -> bool {
        matches!(
            self,
            Self::Crash
                | Self::SceneCrash
                | Self::ExitReason
                | Self::Feedback
                | Self::FreedInstance
        )
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "ffi", derive(uniffi::Enum))]
pub enum ReportLevel {
    Debug,
    Info,
    Warning,
    Error,
    Fatal,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "ffi", derive(uniffi::Enum))]
pub enum Environment {
    Production,
    Staging,
    Development,
}

/// An error or panic to forward to the crash reporter, with what the keep/drop decision reads.
#[derive(Clone, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "ffi", derive(uniffi::Record))]
pub struct ReportEvent {
    /// Set by whoever knows it (a core panic is `Crash`); `None` derives it from the fields below.
    pub source: Option<ReportSource>,
    pub level: ReportLevel,
    pub message: String,
    /// A native crash record.
    pub is_crash: bool,
    /// Value of the `event_kind` tag.
    pub event_kind: Option<String>,
    /// Value of the `category` tag.
    pub category: Option<String>,
    /// Value of the first exception; `None` when the event has no exception.
    pub exception: Option<String>,
}

impl ReportEvent {
    /// An error from the Rust or engine log, carried as an exception.
    pub fn logged_error(text: &str) -> Self {
        Self {
            source: None,
            level: ReportLevel::Error,
            message: String::new(),
            is_crash: false,
            event_kind: None,
            category: None,
            exception: Some(text.to_string()),
        }
    }

    /// A panic of the core.
    pub fn panic(message: &str) -> Self {
        Self {
            source: Some(ReportSource::Crash),
            level: ReportLevel::Fatal,
            message: message.to_string(),
            is_crash: false,
            event_kind: None,
            category: None,
            exception: None,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "ffi", derive(uniffi::Enum))]
pub enum Decision {
    /// Send it, tagged `source`; `attach_log` is the per-session log-upload sample.
    Keep {
        source: ReportSource,
        attach_log: bool,
    },
    Drop,
}
