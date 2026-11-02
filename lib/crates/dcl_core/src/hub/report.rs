//! Crash reports: panics and errors of the core go through the keep/drop policy, then to the
//! shell, which owns the Sentry SDK and sends them with the tags the core builds.

use std::{thread, time::Instant};

use crate::{
    app::{AppEvents, HostEvents},
    flags,
    report::{
        BuildEnvironment, Decision, Push, ReportContext, ReportEvent, ReportSource, ReportTags,
        TagChange,
    },
};

use super::Hub;

/// Flag names as the mobile-bff serves them.
const FLAG_SAMPLE_RATE: &str = "sentry-sample-rate";
const FLAG_ERROR_EVENTS: &str = "sentry-error-events";

const MAX_SCENE_CRASH_REPORTS: usize = 10;
const SCENE_CRASH_REASON_MAX: usize = 160;

/// A report the policy kept, with everything the SDK needs to send it.
#[derive(Clone, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "ffi", derive(uniffi::Record))]
pub struct ReportPayload {
    pub event: ReportEvent,
    /// Also in `tags` as the `source` tag.
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
            policy.set_remote_rate(flags::get_number(FLAG_SAMPLE_RATE, 1.0));
            policy.set_error_events_enabled(flags::is_enabled(FLAG_ERROR_EVENTS, false));
            match policy.decide(&event) {
                Decision::Keep { source, attach_log } => ReportPayload {
                    tags: tags.for_event(source, &event),
                    event,
                    source,
                    attach_log,
                },
                Decision::Drop => return,
            }
        };
        self.events.report(&payload);
    }

    /// The one report a crashed scene sends: once per entity, and for the first ten scenes only.
    pub fn report_scene_crash(
        &self,
        entity_id: &str,
        title: &str,
        parcel: (i32, i32),
        reason: &str,
    ) {
        {
            let mut seen = self.scene_crashes.lock().unwrap_or_else(|e| e.into_inner());
            if seen.len() >= MAX_SCENE_CRASH_REPORTS || !seen.insert(entity_id.to_string()) {
                return;
            }
        }
        let reason: String = reason.chars().take(SCENE_CRASH_REASON_MAX).collect();
        self.report(ReportEvent::scene_crash(&format!(
            "Scene crashed: {title} ({},{}): {reason}",
            parcel.0, parcel.1
        )));
    }

    /// Adds a tag to every later report; cheap enough for a per-resource call. The shell hears
    /// the latest value per key, at most every 250 ms.
    pub fn set_report_tag(&self, key: &str, value: &str) {
        self.push_report_tag(key, Some(value));
    }

    pub fn clear_report_tag(&self, key: &str) {
        self.push_report_tag(key, None);
    }

    fn push_report_tag(&self, key: &str, value: Option<&str>) {
        match self.report_tags.push(key, value, Instant::now()) {
            Push::Deliver(changes) => deliver_tags(&self.events, changes),
            Push::Queued => {}
            Push::FlushIn(wait) => {
                let (events, tags) = (self.events.clone(), self.report_tags.clone());
                thread::spawn(move || {
                    thread::sleep(wait);
                    deliver_tags(&events, tags.flush(Instant::now()));
                });
            }
        }
    }
}

fn deliver_tags(events: &HostEvents, changes: Vec<TagChange>) {
    for (key, value) in changes {
        events.report_tag(&key, value.as_deref());
    }
}
