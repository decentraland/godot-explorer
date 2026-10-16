use std::time::{Duration, Instant};

use crate::engine::local_task::defer;
use godot::prelude::*;

use crate::framework::{RustTestCase, TestContext};

#[derive(GodotClass)]
#[class(init, base=Node)]
pub struct TestRunnerSuite {
    base: Base<Node>,
    total: i64,
    passed: i64,
    skipped: i64,
    focus_run: bool,
}

#[godot_api]
impl INode for TestRunnerSuite {
    fn ready(&mut self) {
        let mut this = self.to_gd();
        defer(move || this.bind_mut().start());
    }
}

impl TestRunnerSuite {
    /// Runs every `#[itest]` and quits with its verdict as the exit code.
    fn start(&mut self) {
        let success = self.run_all_tests(true, self.to_gd().upcast());
        let exit_code = if success { 0 } else { 1 };
        println!("test-exiting with code {exit_code}");
        if let Some(mut tree) = self.base().get_tree_or_null() {
            tree.quit_ex().exit_code(exit_code).done();
        }
    }

    fn run_all_tests(&mut self, allow_focus: bool, scene_tree: Gd<Node>) -> bool {
        println!("{}Run{} Godot integration tests...", FMT_CYAN_BOLD, FMT_END);

        let (rust_tests, rust_file_count, focus_run) = crate::framework::collect_rust_tests();
        self.focus_run = focus_run;
        if focus_run {
            println!("  {FMT_CYAN}Focused run{FMT_END} -- execute only selected Rust tests.")
        }
        println!(
            "  Rust: found {} tests in {} files.",
            rust_tests.len(),
            rust_file_count
        );

        let clock = Instant::now();
        self.run_rust_tests(rust_tests, scene_tree);
        self.conclude(clock.elapsed(), allow_focus)
    }

    fn run_rust_tests(&mut self, tests: Vec<RustTestCase>, scene_tree: Gd<Node>) {
        let ctx = TestContext { scene_tree };

        let mut last_file = None;
        for test in tests {
            print_test_pre(test.name, test.file.to_string(), &mut last_file, false);
            let outcome = run_rust_test(&test, &ctx);

            self.update_stats(&outcome);
            print_test_post(test.name, outcome);
        }
    }

    fn conclude(&self, time: Duration, allow_focus: bool) -> bool {
        let (total, passed, skipped, focus_run) =
            (self.total, self.passed, self.skipped, self.focus_run);

        // Consider 0 tests run as a failure too, because it's probably a problem with the run itself.
        let failed = total - passed - skipped;
        let all_passed = failed == 0 && total != 0;
        let outcome = TestOutcome::from_bool(all_passed);

        let extra = if skipped > 0 {
            format!(", {skipped} skipped")
        } else if focus_run {
            " (focused run)".to_string()
        } else {
            "".to_string()
        };

        println!("\nTest result: {outcome}. {passed} passed; {failed} failed{extra}.");
        println!("  Time: {:.2}s.", time.as_secs_f32());

        if focus_run && !allow_focus {
            println!("  {FMT_YELLOW}Focus run disallowed; return failure.{FMT_END}");
            false
        } else {
            all_passed
        }
    }

    fn update_stats(&mut self, outcome: &TestOutcome) {
        self.total += 1;
        match outcome {
            TestOutcome::Passed => self.passed += 1,
            TestOutcome::Failed => {}
            TestOutcome::Skipped => self.skipped += 1,
        }
    }
}

// For more colors, see https://stackoverflow.com/a/54062826
// To experiment with colors, add `rand` dependency and add following code above.
//     use rand::seq::SliceRandom;
//     let outcome = [TestOutcome::Passed, TestOutcome::Failed, TestOutcome::Skipped];
//     let outcome = outcome.choose(&mut rand::thread_rng()).unwrap();
const FMT_CYAN_BOLD: &str = "\x1b[36;1;1m";
const FMT_CYAN: &str = "\x1b[36m";
const FMT_GREEN: &str = "\x1b[32m";
const FMT_YELLOW: &str = "\x1b[33m";
const FMT_RED: &str = "\x1b[31m";
const FMT_END: &str = "\x1b[0m";

fn run_rust_test(test: &RustTestCase, ctx: &TestContext) -> TestOutcome {
    if test.skipped {
        return TestOutcome::Skipped;
    }

    // Explicit type to prevent tests from returning a value
    let err_context = || format!("itest `{}` failed", test.name);
    let success: Result<(), _> = godot::private::handle_panic(err_context, || (test.function)(ctx));

    TestOutcome::from_bool(success.is_ok())
}

fn print_test_pre(test_case: &str, test_file: String, last_file: &mut Option<String>, flush: bool) {
    // Check if we need to open a new category for a file
    let print_file = last_file.as_ref() != Some(&test_file);

    if print_file {
        let file_subtitle = if let Some(sep_pos) = test_file.rfind(['/', '\\']) {
            &test_file[sep_pos + 1..]
        } else {
            test_file.as_str()
        };

        println!("\n   {file_subtitle}:");
    }

    print!("   -- {test_case} ... ");
    if flush {
        // Flush stdout
        use std::io::Write;
        let _ = std::io::stdout().flush();
    }

    // State update for file-category-print
    *last_file = Some(test_file);
}

/// Prints a test name and its outcome.
///
/// Note that this is run after a test run, so stdout/stderr output during the test will be printed before.
/// It would be possible to print the test name before and the outcome after, but that would split or duplicate the line.
fn print_test_post(test_case: &str, outcome: TestOutcome) {
    // If test failed, something was printed (e.g. assertion), so we can print the entire line again; otherwise just outcome on same line.
    if matches!(outcome, TestOutcome::Failed) {
        println!("   -- {test_case} ... {outcome}");
    } else {
        println!("{outcome}");
    }
}

enum TestOutcome {
    Passed,
    Failed,
    Skipped,
}

impl TestOutcome {
    fn from_bool(success: bool) -> Self {
        if success {
            Self::Passed
        } else {
            Self::Failed
        }
    }
}

impl std::fmt::Display for TestOutcome {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // Do not use print_rich() from Godot, because it's very slow and significantly delays test execution.
        let end = FMT_END;
        let (col, outcome) = match self {
            TestOutcome::Passed => (FMT_GREEN, "ok"),
            TestOutcome::Failed => (FMT_RED, "FAILED"),
            TestOutcome::Skipped => (FMT_YELLOW, "skipped"),
        };

        write!(f, "{col}{outcome}{end}")
    }
}
