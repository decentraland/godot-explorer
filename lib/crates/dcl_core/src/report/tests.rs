use std::collections::VecDeque;
use std::sync::{Arc, Mutex};

use super::*;

/// Returns the queued draws in order, then 0.0 forever; counts draws taken.
struct Scripted {
    draws: Mutex<VecDeque<f64>>,
    taken: Mutex<usize>,
}

impl Scripted {
    fn new(draws: &[f64]) -> Arc<Self> {
        Arc::new(Self {
            draws: Mutex::new(draws.iter().copied().collect()),
            taken: Mutex::new(0),
        })
    }

    fn taken(&self) -> usize {
        *self.taken.lock().unwrap()
    }
}

impl RandomSource for Scripted {
    fn next_f64(&self) -> f64 {
        *self.taken.lock().unwrap() += 1;
        self.draws.lock().unwrap().pop_front().unwrap_or(0.0)
    }
}

const NO_LOG: f64 = 0.5; // first draw: above 0.01, so no log upload

fn policy(env: Environment, draws: &[f64]) -> (ReportPolicy, Arc<Scripted>) {
    let mut all = vec![NO_LOG];
    all.extend_from_slice(draws);
    let rng = Scripted::new(&all);
    (ReportPolicy::new(env, rng.clone()), rng)
}

fn prod(draws: &[f64]) -> (ReportPolicy, Arc<Scripted>) {
    policy(Environment::Production, draws)
}

fn keep(source: ReportSource) -> Decision {
    Decision::Keep {
        source,
        attach_log: false,
    }
}

fn logged(text: &str) -> ReportEvent {
    ReportEvent::logged_error(text)
}

fn with_source(source: ReportSource) -> ReportEvent {
    ReportEvent {
        source: Some(source),
        ..ReportEvent::logged_error("x")
    }
}

#[test]
fn source_tags_and_rates() {
    use ReportSource::*;
    let table = [
        (Crash, "crash", 1.0, true),
        (Capture, "capture", 1.0, false),
        (SceneCrash, "scene_crash", 1.0, true),
        (ExitReason, "exit_reason", 1.0, true),
        (Feedback, "feedback", 1.0, true),
        (FreedInstance, "freed_instance", 1.0, true),
        (RustApp, "rust_app", 1.0, false),
        (RustDep, "rust_dep", 0.05, false),
        (Scene, "scene", 0.01, false),
        (Engine, "engine", 0.01, false),
    ];
    for (source, tag, rate, exempt) in table {
        assert_eq!(source.tag(), tag);
        assert_eq!(source.keep_rate(), rate, "{tag}");
        assert_eq!(source.remote_rate_exempt(), exempt, "{tag}");
    }
}

#[test]
fn development_drops_everything() {
    let (p, _) = policy(Environment::Development, &[]);
    assert_eq!(p.decide(&ReportEvent::panic("boom")), Decision::Drop);
    assert_eq!(
        p.decide(&with_source(ReportSource::Feedback)),
        Decision::Drop
    );
}

#[test]
fn staging_reports_like_production() {
    let (p, _) = policy(Environment::Staging, &[]);
    assert_eq!(
        p.decide(&ReportEvent::panic("boom")),
        keep(ReportSource::Crash)
    );
}

#[test]
fn attach_log_is_a_per_session_roll() {
    for (draw, expected) in [(0.0, true), (0.0099, true), (0.01, false), (0.9, false)] {
        let p = ReportPolicy::new(Environment::Production, Scripted::new(&[draw]));
        assert_eq!(p.attach_log(), expected, "draw {draw}");
        assert_eq!(
            p.decide(&ReportEvent::panic("x")),
            Decision::Keep {
                source: ReportSource::Crash,
                attach_log: expected
            }
        );
    }
}

#[test]
fn derives_crash_from_native_crash_or_fatal_level() {
    let (p, _) = prod(&[]);
    let native = ReportEvent {
        is_crash: true,
        ..logged("x")
    };
    let fatal = ReportEvent {
        level: ReportLevel::Fatal,
        ..logged("x")
    };
    assert_eq!(p.decide(&native), keep(ReportSource::Crash));
    assert_eq!(p.decide(&fatal), keep(ReportSource::Crash));
}

#[test]
fn derives_structured_sources_before_the_shape_test() {
    let (p, _) = prod(&[]);
    let structured = |kind: Option<&str>, category: Option<&str>| ReportEvent {
        event_kind: kind.map(String::from),
        category: category.map(String::from),
        exception: None,
        ..logged("x")
    };
    assert_eq!(
        p.decide(&structured(Some("scene_crash"), None)),
        keep(ReportSource::SceneCrash)
    );
    assert_eq!(
        p.decide(&structured(Some("exit_reason"), None)),
        keep(ReportSource::ExitReason)
    );
    assert_eq!(
        p.decide(&structured(None, Some("FEEDBACK"))),
        keep(ReportSource::Feedback)
    );
    // Unknown kind or category falls through to the shape test: no exception means capture.
    assert_eq!(
        p.decide(&structured(Some("other"), Some("feedback"))),
        keep(ReportSource::Capture)
    );
}

#[test]
fn event_without_exception_is_a_capture() {
    let (p, _) = prod(&[0.0]);
    let e = ReportEvent {
        exception: None,
        message: "NodeGuard".into(),
        ..logged("")
    };
    assert_eq!(p.decide(&e), keep(ReportSource::Capture));
}

#[test]
fn exception_events_are_dropped_unless_error_events_enabled() {
    let (p, rng) = prod(&[]);
    for text in [
        "ERROR: boom",
        "[Rust:dclgodot::comms] x",
        "[Rust:tokio] x",
        "[Rust:dclgodot::dcl::js] x",
    ] {
        assert_eq!(p.decide(&logged(text)), Decision::Drop, "{text}");
    }
    assert_eq!(rng.taken(), 1, "only the session roll; drops draw nothing");
}

#[test]
fn freed_instance_is_kept_with_the_flag_off() {
    let (p, _) = prod(&[]);
    for text in [
        "Invalid call on a previously freed instance.",
        "Trying to cast a freed object.",
        "Trying to await on a freed object.",
        "Invalid get index on a base object of type 'previously freed'",
    ] {
        assert_eq!(
            p.decide(&logged(text)),
            keep(ReportSource::FreedInstance),
            "{text}"
        );
    }
}

#[test]
fn freed_instance_text_with_rust_prefix_is_not_exempt() {
    let (mut p, _) = prod(&[0.0, 0.0]);
    assert_eq!(
        p.decide(&logged("[Rust:dclgodot::dcl::js] a freed object")),
        Decision::Drop
    );
    p.set_error_events_enabled(true);
    assert_eq!(
        p.decide(&logged("[Rust:dclgodot::dcl::js] a freed object")),
        keep(ReportSource::Scene)
    );
}

#[test]
fn signal_emission_message_stays_an_engine_error() {
    let (mut p, _) = prod(&[]);
    let e = logged("Object was freed or unreferenced while a signal is being emitted");
    assert_eq!(p.decide(&e), Decision::Drop);
    p.set_error_events_enabled(true);
    assert_eq!(classified(&p, &e), Some(ReportSource::Engine));
}

fn classified(p: &ReportPolicy, e: &ReportEvent) -> Option<ReportSource> {
    // Keep rate 0.01 and a 0.0 draw: the event survives, so the source shows in the decision.
    match p.decide(e) {
        Decision::Keep { source, .. } => Some(source),
        Decision::Drop => None,
    }
}

#[test]
fn classification_with_error_events_enabled() {
    use ReportSource::*;
    let table = [
        ("[Rust:dclgodot::dcl::js] scene threw", Scene),
        ("[Rust:dclgodot::comms] lost", RustApp),
        ("[Rust:dclgodot] root", RustApp),
        ("[Rust:tokio::runtime] x", RustDep),
        ("[Rust:] x", RustDep),
        ("ERROR: Condition failed", Engine),
        ("", Engine),
        ("rust:dclgodot lower case", Engine),
    ];
    for (text, expected) in table {
        let (mut p, _) = prod(&[0.0, 0.0]);
        p.set_error_events_enabled(true);
        assert_eq!(classified(&p, &logged(text)), Some(expected), "{text:?}");
    }
}

#[test]
fn exception_text_falls_back_to_message_when_empty() {
    let (mut p, _) = prod(&[0.0, 0.0]);
    let e = ReportEvent {
        message: "[Rust:tokio] x".into(),
        exception: Some(String::new()),
        ..logged("")
    };
    assert_eq!(p.decide(&e), Decision::Drop);
    p.set_error_events_enabled(true);
    assert_eq!(classified(&p, &e), Some(ReportSource::RustDep));
}

#[test]
fn per_source_keep_rate_boundaries() {
    use ReportSource::*;
    // (source, draw for the source roll, kept). The remote roll draws 0.0 and always passes.
    let table = [
        (RustDep, 0.0499, true),
        (RustDep, 0.05, false),
        (Scene, 0.0099, true),
        (Scene, 0.01, false),
        (Engine, 0.0099, true),
        (Engine, 0.01, false),
        (RustApp, 0.9999, true),
        (Capture, 0.9999, true),
    ];
    for (source, draw, kept) in table {
        let (p, _) = prod(&[0.0, draw]);
        let expected = if kept { keep(source) } else { Decision::Drop };
        assert_eq!(
            p.decide(&with_source(source)),
            expected,
            "{source:?} {draw}"
        );
    }
}

#[test]
fn rate_one_sources_do_not_draw_for_the_keep_rate() {
    let (p, rng) = prod(&[0.0]);
    p.decide(&with_source(ReportSource::RustApp));
    assert_eq!(rng.taken(), 2, "session roll + remote roll");
    let (p, rng) = prod(&[]);
    p.decide(&with_source(ReportSource::Crash));
    assert_eq!(rng.taken(), 1, "exempt and rate 1.0: no draws");
}

#[test]
fn remote_rate_rolls_before_the_source_rate() {
    // rate 0.5: remote draw 0.49 passes, 0.5 drops.
    let table = [(0.5, 0.49, true), (0.5, 0.5, false)];
    for (rate, draw, kept) in table {
        let (mut p, _) = prod(&[draw]);
        p.set_remote_rate(rate);
        let expected = if kept {
            keep(ReportSource::RustApp)
        } else {
            Decision::Drop
        };
        assert_eq!(p.decide(&with_source(ReportSource::RustApp)), expected);
    }
    // A remote drop does not spend the source roll.
    let (mut p, rng) = prod(&[0.9]);
    p.set_remote_rate(0.5);
    assert_eq!(p.decide(&with_source(ReportSource::Engine)), Decision::Drop);
    assert_eq!(rng.taken(), 2);
}

#[test]
fn remote_rate_zero_is_a_total_kill_switch() {
    let (mut p, rng) = prod(&[]);
    p.set_remote_rate(0.0);
    for source in [
        ReportSource::Crash,
        ReportSource::SceneCrash,
        ReportSource::ExitReason,
        ReportSource::Feedback,
        ReportSource::FreedInstance,
        ReportSource::RustApp,
        ReportSource::Capture,
    ] {
        assert_eq!(p.decide(&with_source(source)), Decision::Drop, "{source:?}");
    }
    assert_eq!(rng.taken(), 1);
}

#[test]
fn remote_rate_one_keeps_non_exempt_sources_without_losing_to_a_draw() {
    let (mut p, _) = prod(&[0.999_999]);
    p.set_remote_rate(1.0);
    assert_eq!(
        p.decide(&with_source(ReportSource::RustApp)),
        keep(ReportSource::RustApp)
    );
}

#[test]
fn exempt_sources_ignore_a_low_remote_rate() {
    let (mut p, _) = prod(&[]);
    p.set_remote_rate(0.000_1);
    assert_eq!(
        p.decide(&with_source(ReportSource::Crash)),
        keep(ReportSource::Crash)
    );
}

#[test]
fn remote_rate_is_clamped_and_defaults_to_one() {
    let (mut p, _) = prod(&[]);
    assert_eq!(p.remote_rate(), 1.0);
    for (input, expected) in [
        (0.0, 0.0),
        (0.25, 0.25),
        (1.0, 1.0),
        (-3.0, 0.0),
        (7.5, 1.0),
        (f64::INFINITY, 1.0),
        (f64::NEG_INFINITY, 0.0),
        (f64::NAN, 1.0),
    ] {
        p.set_remote_rate(input);
        assert_eq!(p.remote_rate(), expected, "{input}");
    }
}

#[test]
fn explicit_source_wins_over_the_fields() {
    let (p, _) = prod(&[]);
    let e = ReportEvent {
        is_crash: false,
        level: ReportLevel::Debug,
        ..with_source(ReportSource::Crash)
    };
    assert_eq!(p.decide(&e), keep(ReportSource::Crash));
}

#[test]
fn panic_event_is_fatal_crash() {
    let e = ReportEvent::panic("boom");
    assert_eq!(e.source, Some(ReportSource::Crash));
    assert_eq!(e.level, ReportLevel::Fatal);
    assert_eq!(e.message, "boom");
}

#[test]
fn thread_random_policy_keeps_a_crash() {
    let p = ReportPolicy::with_thread_random(Environment::Production);
    assert!(matches!(
        p.decide(&ReportEvent::panic("x")),
        Decision::Keep { .. }
    ));
}

fn context() -> ReportContext {
    ReportContext {
        platform: "Android".into(),
        debug_build: false,
        gpu_name: "Mali-G57".into(),
        gpu_vendor: "ARM".into(),
        gpu_api_version: "Vulkan 1.1".into(),
        locale: "es_AR".into(),
        commit_hash: "abc123".into(),
        physical_ram_bytes: Some(6 * 1_073_741_824),
        attach_branch_info: false,
        branch_name: "feat/x".into(),
        commit_message: "msg".into(),
    }
}

#[test]
fn tags_for_a_production_device() {
    let t = ReportTags::from_context(&context(), true);
    assert_eq!(t.tag("attach_log_sampled"), Some("true"));
    assert_eq!(t.tag("platform"), Some("Android"));
    assert_eq!(t.tag("build_type"), Some("release"));
    assert_eq!(t.tag("gpu_vendor"), Some("ARM"));
    assert_eq!(t.tag("locale"), Some("es_AR"));
    assert_eq!(t.tag("commit_hash"), Some("abc123"));
    assert_eq!(t.tag("total_ram_mb"), Some("6144"));
    assert_eq!(t.tag("branch_name"), None);
    assert_eq!(t.tag("commit_message"), None);
    let graphics: Vec<_> = t
        .graphics
        .iter()
        .map(|g| (g.key.as_str(), g.value.as_str()))
        .collect();
    assert_eq!(
        graphics,
        [
            ("name", "Mali-G57"),
            ("vendor", "ARM"),
            ("version", "Vulkan 1.1")
        ]
    );
}

#[test]
fn tags_for_a_debug_staging_build_carry_branch_info() {
    let ctx = ReportContext {
        debug_build: true,
        attach_branch_info: true,
        ..context()
    };
    let t = ReportTags::from_context(&ctx, false);
    assert_eq!(t.tag("build_type"), Some("debug"));
    assert_eq!(t.tag("attach_log_sampled"), Some("false"));
    assert_eq!(t.tag("branch_name"), Some("feat/x"));
    assert_eq!(t.tag("commit_message"), Some("msg"));
}

#[test]
fn empty_and_unknown_values_are_omitted() {
    for ram in [None, Some(-1), Some(0)] {
        let ctx = ReportContext {
            commit_hash: String::new(),
            physical_ram_bytes: ram,
            attach_branch_info: true,
            branch_name: String::new(),
            commit_message: String::new(),
            ..context()
        };
        let t = ReportTags::from_context(&ctx, false);
        for key in [
            "commit_hash",
            "total_ram_mb",
            "branch_name",
            "commit_message",
        ] {
            assert_eq!(t.tag(key), None, "{key} with ram {ram:?}");
        }
    }
}

#[test]
fn loading_resource_tag_name() {
    assert_eq!(LOADING_RESOURCE_TAG, "loading_resource");
}
