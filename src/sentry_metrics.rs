// TEMPORARY: This module is temporary and will be replaced by a proper monitoring solution.
// It fetches Sentry metrics for godot-explorer and auth-mobile projects and optionally
// pushes a summary to Slack.

use crate::ui::{print_divider, print_message, print_section, MessageType};
use chrono::{NaiveDate, Timelike, Utc};
use serde_json::Value;
use std::env;

const SENTRY_BASE_URL: &str = "https://sentry.io/api/0";

struct SentryConfig {
    token: String,
    org: String,
}

/// How the "healthy rate" is calculated for each project.
enum RateMode {
    /// Rate = (sessions - crashes) / sessions (ignores errors)
    CrashFree,
    /// Rate = (sessions - errors - crashes) / sessions
    ErrorFree,
}

struct ProjectMetrics {
    name: String,
    rate_label: String,
    track_users: bool,
    days: Vec<DayMetrics>,
    total_sessions: u64,
    total_errors: u64,
    total_crashes: u64,
    total_users: u64,
    healthy_rate: f64,
}

struct DayMetrics {
    date: String,
    sessions: u64,
    errors: u64,
    crashes: u64,
    users: u64,
    is_partial: bool,
    partial_hours: Option<u64>,
}

impl SentryConfig {
    fn from_env() -> anyhow::Result<Self> {
        let token = env::var("SENTRY_AUTH_TOKEN")
            .map_err(|_| anyhow::anyhow!("SENTRY_AUTH_TOKEN not set"))?;
        let org = env::var("SENTRY_ORG").map_err(|_| anyhow::anyhow!("SENTRY_ORG not set"))?;
        Ok(Self { token, org })
    }
}

fn sentry_get(config: &SentryConfig, path: &str) -> anyhow::Result<Value> {
    let client = reqwest::blocking::Client::new();
    let url = format!("{}{}", SENTRY_BASE_URL, path);
    let resp = client
        .get(&url)
        .header("Authorization", format!("Bearer {}", config.token))
        .send()?;

    if !resp.status().is_success() {
        let status = resp.status();
        let body = resp.text().unwrap_or_default();
        anyhow::bail!("Sentry API error {}: {}", status, body);
    }

    Ok(resp.json()?)
}

fn get_project_id(config: &SentryConfig, project_slug: &str) -> anyhow::Result<String> {
    let projects = sentry_get(
        config,
        &format!("/organizations/{}/projects/", config.org),
    )?;

    if let Some(arr) = projects.as_array() {
        for p in arr {
            if p["slug"].as_str() == Some(project_slug) {
                if let Some(id) = p["id"].as_str() {
                    return Ok(id.to_string());
                }
            }
        }
    }

    anyhow::bail!(
        "Project '{}' not found in org '{}'",
        project_slug,
        config.org
    )
}

fn get_sessions_by_day(
    config: &SentryConfig,
    project_id: &str,
    start: &str,
    end: &str,
) -> anyhow::Result<Vec<DayMetrics>> {
    let path = format!(
        "/organizations/{}/sessions/?project={}&field=sum(session)&field=count_unique(user)&groupBy=session.status&interval=1d&start={}&end={}",
        config.org, project_id, start, end
    );

    let data = sentry_get(config, &path)?;
    let now = Utc::now();

    let intervals = data["intervals"]
        .as_array()
        .ok_or_else(|| anyhow::anyhow!("No intervals in response"))?;

    let groups = data["groups"]
        .as_array()
        .ok_or_else(|| anyhow::anyhow!("No groups in response"))?;

    let mut days: Vec<DayMetrics> = Vec::new();

    for (i, interval) in intervals.iter().enumerate() {
        let date_str = interval.as_str().unwrap_or("");
        let display_date = if date_str.len() >= 10 {
            &date_str[..10]
        } else {
            date_str
        };

        let mut healthy: u64 = 0;
        let mut errored: u64 = 0;
        let mut crashed: u64 = 0;
        let mut abnormal: u64 = 0;

        let mut is_partial = false;
        let mut partial_hours = None;

        if let Ok(parsed_date) = NaiveDate::parse_from_str(display_date, "%Y-%m-%d") {
            let today = now.date_naive();
            if parsed_date == today {
                is_partial = true;
                partial_hours = Some(now.time().hour() as u64);
            } else if parsed_date > today {
                continue;
            }
        }

        let mut users: u64 = 0;

        for group in groups {
            let status = group["by"]["session.status"].as_str().unwrap_or("");
            let series = &group["series"]["sum(session)"];
            if let Some(arr) = series.as_array() {
                if let Some(val) = arr.get(i) {
                    let count = val.as_u64().unwrap_or(0);
                    match status {
                        "healthy" => healthy = count,
                        "errored" => errored = count,
                        "crashed" => crashed = count,
                        "abnormal" => abnormal = count,
                        _ => {}
                    }
                }
            }
            // count_unique(user) per status — sum across all statuses for total unique
            let user_series = &group["series"]["count_unique(user)"];
            if let Some(arr) = user_series.as_array() {
                if let Some(val) = arr.get(i) {
                    users += val.as_u64().unwrap_or(0);
                }
            }
        }

        let total_sessions = healthy + errored + crashed + abnormal;

        days.push(DayMetrics {
            date: display_date.to_string(),
            sessions: total_sessions,
            errors: errored,
            crashes: crashed,
            users,
            is_partial,
            partial_hours,
        });
    }

    Ok(days)
}

fn build_project_metrics(
    config: &SentryConfig,
    project_slug: &str,
    project_id: &str,
    start: &str,
    end: &str,
    rate_mode: RateMode,
    track_users: bool,
) -> anyhow::Result<ProjectMetrics> {
    let days = get_sessions_by_day(config, project_id, start, end)?;

    let total_sessions: u64 = days.iter().map(|d| d.sessions).sum();
    let total_errors: u64 = days.iter().map(|d| d.errors).sum();
    let total_crashes: u64 = days.iter().map(|d| d.crashes).sum();
    let total_users: u64 = days.iter().map(|d| d.users).sum();

    let (healthy_rate, rate_label) = match rate_mode {
        RateMode::CrashFree => {
            let healthy = total_sessions.saturating_sub(total_crashes);
            let rate = if total_sessions > 0 {
                (healthy as f64 / total_sessions as f64) * 100.0
            } else {
                0.0
            };
            (rate, "Crash-Free Rate".to_string())
        }
        RateMode::ErrorFree => {
            let healthy = total_sessions.saturating_sub(total_errors + total_crashes);
            let rate = if total_sessions > 0 {
                (healthy as f64 / total_sessions as f64) * 100.0
            } else {
                0.0
            };
            (rate, "Error-Free Rate".to_string())
        }
    };

    Ok(ProjectMetrics {
        name: project_slug.to_string(),
        rate_label,
        track_users,
        days,
        total_sessions,
        total_errors,
        total_crashes,
        total_users,
        healthy_rate,
    })
}

fn print_project_metrics(metrics: &ProjectMetrics) {
    print_section(&format!("Project: {}", metrics.name));

    let users_str = if metrics.track_users {
        format!(" | {} users", metrics.total_users)
    } else {
        " | users: not tracked".to_string()
    };

    print_message(
        if metrics.healthy_rate >= 90.0 {
            MessageType::Success
        } else if metrics.healthy_rate >= 75.0 {
            MessageType::Warning
        } else {
            MessageType::Error
        },
        &format!(
            "{}: {:.1}% | {} sessions{} | {} errors | {} crashes",
            metrics.rate_label,
            metrics.healthy_rate,
            metrics.total_sessions,
            users_str,
            metrics.total_errors,
            metrics.total_crashes
        ),
    );

    println!();
    if metrics.track_users {
        println!(
            "  {:<12} {:>10} {:>8} {:>10} {:>10}",
            "Date", "Sessions", "Users", "Errors", "Crashes"
        );
        println!("  {}", "-".repeat(54));
    } else {
        println!(
            "  {:<12} {:>10} {:>10} {:>10}",
            "Date", "Sessions", "Errors", "Crashes"
        );
        println!("  {}", "-".repeat(44));
    }

    for day in &metrics.days {
        let partial_note = if day.is_partial {
            format!(" ({}h so far)", day.partial_hours.unwrap_or(0))
        } else {
            String::new()
        };

        if metrics.track_users {
            println!(
                "  {:<12} {:>10} {:>8} {:>10} {:>10}{}",
                day.date, day.sessions, day.users, day.errors, day.crashes, partial_note
            );
        } else {
            println!(
                "  {:<12} {:>10} {:>10} {:>10}{}",
                day.date, day.sessions, day.errors, day.crashes, partial_note
            );
        }
    }
}

pub fn get_metrics(from: &str, to: &str) -> anyhow::Result<()> {
    print_section("Sentry Metrics (TEMPORARY)");
    print_message(
        MessageType::Warning,
        "This command is temporary and will be replaced by a proper monitoring solution.",
    );

    let config = SentryConfig::from_env()?;

    let from_date = NaiveDate::parse_from_str(from, "%Y-%m-%d")
        .map_err(|e| anyhow::anyhow!("Invalid 'from' date '{}': {}. Use YYYY-MM-DD", from, e))?;
    let to_date = NaiveDate::parse_from_str(to, "%Y-%m-%d")
        .map_err(|e| anyhow::anyhow!("Invalid 'to' date '{}': {}. Use YYYY-MM-DD", to, e))?;

    if from_date > to_date {
        anyhow::bail!("'from' date must be before 'to' date");
    }

    let end_date = to_date
        .succ_opt()
        .ok_or_else(|| anyhow::anyhow!("Date overflow"))?;

    let start = format!("{}T00:00:00Z", from_date);
    let end = format!("{}T00:00:00Z", end_date);

    print_message(
        MessageType::Info,
        &format!("Period: {} to {} (inclusive)", from, to),
    );
    print_divider();

    let ge_id = get_project_id(&config, "godot-explorer")?;
    let am_id = get_project_id(&config, "auth-mobile")?;

    let ge_metrics = build_project_metrics(
        &config,
        "godot-explorer",
        &ge_id,
        &start,
        &end,
        RateMode::CrashFree,
        true,
    )?;
    print_project_metrics(&ge_metrics);

    print_divider();

    let am_metrics = build_project_metrics(
        &config,
        "auth-mobile",
        &am_id,
        &start,
        &end,
        RateMode::ErrorFree,
        false,
    )?;
    print_project_metrics(&am_metrics);

    Ok(())
}

pub fn push_metrics(from: &str, to: &str) -> anyhow::Result<()> {
    let slack_webhook =
        env::var("SLACK_WEBHOOK_URL").map_err(|_| anyhow::anyhow!("SLACK_WEBHOOK_URL not set"))?;

    let config = SentryConfig::from_env()?;

    let from_date = NaiveDate::parse_from_str(from, "%Y-%m-%d")
        .map_err(|e| anyhow::anyhow!("Invalid 'from' date '{}': {}", from, e))?;
    let to_date = NaiveDate::parse_from_str(to, "%Y-%m-%d")
        .map_err(|e| anyhow::anyhow!("Invalid 'to' date '{}': {}", to, e))?;

    if from_date > to_date {
        anyhow::bail!("'from' date must be before 'to' date");
    }

    let end_date = to_date
        .succ_opt()
        .ok_or_else(|| anyhow::anyhow!("Date overflow"))?;

    let start = format!("{}T00:00:00Z", from_date);
    let end = format!("{}T00:00:00Z", end_date);

    let ge_id = get_project_id(&config, "godot-explorer")?;
    let am_id = get_project_id(&config, "auth-mobile")?;

    let ge_metrics = build_project_metrics(
        &config,
        "godot-explorer",
        &ge_id,
        &start,
        &end,
        RateMode::CrashFree,
        true,
    )?;
    let am_metrics = build_project_metrics(
        &config,
        "auth-mobile",
        &am_id,
        &start,
        &end,
        RateMode::ErrorFree,
        false,
    )?;

    // Also print to terminal
    print_section("Sentry Metrics (TEMPORARY)");
    print_project_metrics(&ge_metrics);
    print_divider();
    print_project_metrics(&am_metrics);
    print_divider();

    let ge_emoji = rate_emoji(ge_metrics.healthy_rate);
    let am_emoji = rate_emoji(am_metrics.healthy_rate);

    let ge_days_text = format_days_for_slack(&ge_metrics.days, ge_metrics.track_users);
    let am_days_text = format_days_for_slack(&am_metrics.days, am_metrics.track_users);

    let slack_payload = serde_json::json!({
        "blocks": [
            {
                "type": "header",
                "text": {
                    "type": "plain_text",
                    "text": format!(":bar_chart: Sentry Daily Digest ({} to {})", from, to)
                }
            },
            {
                "type": "section",
                "text": {
                    "type": "mrkdwn",
                    "text": format!(
                        "{} *godot-explorer* — {}: *{:.1}%*\n\
                        Sessions: {} | Users: {} | Errors: {} | Crashes: {}",
                        ge_emoji,
                        ge_metrics.rate_label,
                        ge_metrics.healthy_rate,
                        ge_metrics.total_sessions,
                        ge_metrics.total_users,
                        ge_metrics.total_errors,
                        ge_metrics.total_crashes
                    )
                }
            },
            {
                "type": "section",
                "text": {
                    "type": "mrkdwn",
                    "text": format!("```{}```", ge_days_text)
                }
            },
            {
                "type": "divider"
            },
            {
                "type": "section",
                "text": {
                    "type": "mrkdwn",
                    "text": format!(
                        "{} *auth-mobile* — {}: *{:.1}%*\n\
                        Sessions: {} | Users: _not tracked_ | Errors: {} | Crashes: {}",
                        am_emoji,
                        am_metrics.rate_label,
                        am_metrics.healthy_rate,
                        am_metrics.total_sessions,
                        am_metrics.total_errors,
                        am_metrics.total_crashes
                    )
                }
            },
            {
                "type": "section",
                "text": {
                    "type": "mrkdwn",
                    "text": format!("```{}```", am_days_text)
                }
            },
            {
                "type": "context",
                "elements": [
                    {
                        "type": "mrkdwn",
                        "text": ":construction: _This report is temporary._"
                    }
                ]
            }
        ]
    });

    print_message(MessageType::Step, "Pushing to Slack...");

    let client = reqwest::blocking::Client::new();
    let resp = client.post(&slack_webhook).json(&slack_payload).send()?;

    if resp.status().is_success() {
        print_message(MessageType::Success, "Metrics pushed to Slack!");
    } else {
        let status = resp.status();
        let body = resp.text().unwrap_or_default();
        anyhow::bail!("Slack webhook error {}: {}", status, body);
    }

    Ok(())
}

fn rate_emoji(rate: f64) -> &'static str {
    if rate >= 90.0 {
        ":large_green_circle:"
    } else if rate >= 75.0 {
        ":large_yellow_circle:"
    } else {
        ":red_circle:"
    }
}

fn format_days_for_slack(days: &[DayMetrics], track_users: bool) -> String {
    let mut lines = if track_users {
        vec![format!(
            "{:<12} {:>10} {:>8} {:>8} {:>8}",
            "Date", "Sessions", "Users", "Errors", "Crashes"
        )]
    } else {
        vec![format!(
            "{:<12} {:>10} {:>8} {:>8}",
            "Date", "Sessions", "Errors", "Crashes"
        )]
    };

    for day in days {
        let partial = if day.is_partial {
            format!(" ({}h)", day.partial_hours.unwrap_or(0))
        } else {
            String::new()
        };

        if track_users {
            lines.push(format!(
                "{:<12} {:>10} {:>8} {:>8} {:>8}{}",
                day.date, day.sessions, day.users, day.errors, day.crashes, partial
            ));
        } else {
            lines.push(format!(
                "{:<12} {:>10} {:>8} {:>8}{}",
                day.date, day.sessions, day.errors, day.crashes, partial
            ));
        }
    }

    lines.join("\n")
}
