//! The list of scenes a scene-test run visits, and how each one is doing.

use std::time::{Duration, Instant};

use godot::prelude::*;

/// How long a scene gets to load and to finish its tests before the run gives up on it.
pub const SCENE_TIMEOUT: Duration = Duration::from_secs(15);

/// How long a scene that is loaded gets to report its tests: a scene with several snapshot tests
/// needs about ten seconds each.
pub const SCENE_TESTS_TIMEOUT: Duration = Duration::from_secs(180);

/// What a finished scene reported.
#[derive(Clone, Debug, Default)]
pub struct SceneReport {
    pub text: String,
    pub detail_failed: String,
    pub failed: i64,
}

impl SceneReport {
    pub fn from_dictionary(dict: &VarDictionary) -> Self {
        let text = |key: &str| dict.get(key).map(|v| v.to_string()).unwrap_or_default();
        Self {
            text: text("text"),
            detail_failed: text("text_detail_failed"),
            failed: dict
                .get("fail")
                .and_then(|v| v.try_to::<i64>().ok())
                .unwrap_or(0),
        }
    }
}

pub struct SceneTest {
    pub parcel: Vector2i,
    pub finished: bool,
    pub teleported: bool,
    /// The scene was found; its own clock to report its tests started then.
    pub seen: bool,
    pub report: Option<SceneReport>,
    started: Instant,
}

impl SceneTest {
    pub fn new(parcel: Vector2i) -> Self {
        Self {
            parcel,
            finished: false,
            teleported: false,
            seen: false,
            report: None,
            started: Instant::now(),
        }
    }

    pub fn reset_timeout(&mut self) {
        self.started = Instant::now();
    }

    pub fn timed_out(&self) -> bool {
        self.started.elapsed() > SCENE_TIMEOUT
    }

    pub fn tests_timed_out(&self) -> bool {
        self.started.elapsed() > SCENE_TESTS_TIMEOUT
    }

    pub fn elapsed_secs(&self) -> f32 {
        self.started.elapsed().as_secs_f32()
    }
}

/// `[[52,-52],[52,-54]]` (single quotes allowed: the shell's quoting) as parcels. Entries that
/// are not a pair of integers are skipped and returned as the second value.
pub fn parse_parcels(text: &str) -> Result<(Vec<Vector2i>, Vec<String>), String> {
    let value: serde_json::Value =
        serde_json::from_str(&text.replace('\'', "\"")).map_err(|e| e.to_string())?;
    let entries = value
        .as_array()
        .ok_or_else(|| "the scenes to test have to be an array".to_string())?;
    let mut parcels = Vec::new();
    let mut skipped = Vec::new();
    for entry in entries {
        let pair = entry.as_array().filter(|pair| pair.len() == 2);
        let coordinates = pair.and_then(|pair| Some((pair[0].as_f64()?, pair[1].as_f64()?)));
        match coordinates {
            Some((x, y)) => parcels.push(Vector2i::new(x as i32, y as i32)),
            None => skipped.push(entry.to_string()),
        }
    }
    Ok((parcels, skipped))
}

/// The summary line a run ends with: true when every scene reported and none failed.
pub fn summarize(tests: &[SceneTest]) -> (bool, Vec<String>) {
    let mut lines = Vec::new();
    let mut ok = true;
    let mut failed = 0;
    for test in tests {
        match &test.report {
            None => {
                ok = false;
                lines.push(format!(
                    "🔴 test result is empty in the scene {}",
                    parcel_text(test.parcel)
                ));
            }
            Some(report) => {
                lines.push(report.text.clone());
                lines.push(report.detail_failed.clone());
                failed += report.failed;
            }
        }
    }
    if failed > 0 || !ok {
        lines.push("Some tests fail or some scenes couldn't be tested".to_string());
        return (false, lines);
    }
    lines.push("All test of all scene passed".to_string());
    (true, lines)
}

fn parcel_text(parcel: Vector2i) -> String {
    format!("({}, {})", parcel.x, parcel.y)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parcels_come_from_pairs() {
        let (parcels, skipped) = parse_parcels("[[52,-52],[54, -56]]").unwrap();
        assert_eq!(
            parcels,
            vec![Vector2i::new(52, -52), Vector2i::new(54, -56)]
        );
        assert!(skipped.is_empty());
    }

    #[test]
    fn single_quotes_and_bad_entries_are_tolerated() {
        let (parcels, skipped) = parse_parcels("[[1,2],['a'],[1,2,3]]").unwrap();
        assert_eq!(parcels, vec![Vector2i::new(1, 2)]);
        assert_eq!(skipped.len(), 2);
    }

    #[test]
    fn something_that_is_not_a_list_is_an_error() {
        assert!(parse_parcels("{}").is_err());
        assert!(parse_parcels("not json").is_err());
    }

    fn done(failed: i64) -> SceneTest {
        let mut test = SceneTest::new(Vector2i::new(1, 1));
        test.report = Some(SceneReport {
            text: "ok".into(),
            detail_failed: String::new(),
            failed,
        });
        test
    }

    #[test]
    fn a_run_passes_when_every_scene_reported_clean() {
        let (ok, lines) = summarize(&[done(0), done(0)]);
        assert!(ok);
        assert_eq!(lines.last().unwrap(), "All test of all scene passed");
    }

    #[test]
    fn a_failure_or_a_missing_report_fails_the_run() {
        assert!(!summarize(&[done(0), done(2)]).0);
        let (ok, lines) = summarize(&[SceneTest::new(Vector2i::new(3, 4))]);
        assert!(!ok);
        assert!(lines[0].contains("(3, 4)"));
        assert_eq!(
            lines.last().unwrap(),
            "Some tests fail or some scenes couldn't be tested"
        );
    }
}
