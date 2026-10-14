//! Crash reports: panics and errors of the core go through the keep/drop policy, then to the
//! shell, which owns the Sentry SDK and sends them with the tags the core builds.

use crate::{
    app::AppEvents,
    flags,
    report::{BuildEnvironment, Decision, ReportContext, ReportEvent, ReportSource, ReportTags},
};

use super::Hub;

/// A report the policy kept, with everything the SDK needs to send it.
#[derive(Clone, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "ffi", derive(uniffi::Record))]
pub struct ReportPayload {
    pub event: ReportEvent,
    /// The `source` tag.
    pub source: ReportSource,
    pub attach_log: bool,
    pub tags: ReportTags,
}

impl Hub {
    /// What the tags are made of: the device and build facts only the host and engine know.
    pub fn set_report_context(&self, context: ReportContext, environment: BuildEnvironment) {
        let mut policy = self.report.lock().unwrap_or_else(|e| e.into_inner());
        policy.0 = crate::report::ReportPolicy::with_thread_random(environment);
        let attach_log = policy.0.attach_log();
        policy.1 = ReportTags::from_context(&context, attach_log);
    }

    /// Applies the policy and tells the listeners about a kept event. Safe from a panic hook: it
    /// takes no lock a panicking caller could hold, and gives up instead of waiting for one.
    pub fn report(&self, event: ReportEvent) {
        let payload = {
            let Ok(mut state) = self.report.try_lock() else {
                return;
            };
            let (policy, tags) = &mut *state;
            policy.set_remote_rate(flags::get_number("sentry_sample_rate", 1.0));
            policy.set_error_events_enabled(flags::is_enabled("sentry-error-events", false));
            match policy.decide(&event) {
                Decision::Keep { source, attach_log } => ReportPayload {
                    event,
                    source,
                    attach_log,
                    tags: tags.clone(),
                },
                Decision::Drop => return,
            }
        };
        self.events.report(&payload);
    }
}
