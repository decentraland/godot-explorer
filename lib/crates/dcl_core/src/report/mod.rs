//! Crash-report policy: which events are kept and at what rate (port of project_main_loop.gd).
//! Pure decisions and plain data; the shell owns the Sentry SDK and applies both.

mod coalesce;
mod event;
mod policy;
mod tags;

#[cfg(test)]
mod tests;

pub use coalesce::{Push, TagChange, TagCoalescer};
pub use event::{BuildEnvironment, Decision, ReportEvent, ReportLevel, ReportSource};
pub use policy::{RandomSource, ReportPolicy, ThreadRandom};
pub use tags::{ReportContext, ReportTag, ReportTags, LOADING_RESOURCE_TAG};
