#!/usr/bin/env python3
#
# ⚠️ DUPLICATED FILE — a copy lives in the other repo.
# The mobile build pipeline spans two repos, so this script lives in BOTH
# decentraland/godot-explorer and decentraland/godot-asc-deploy. A change to the event contract
# (env vars, payload fields) must land in both copies. The asc-deploy copy predates the `::warning`
# annotations below and prints those cases as plain log lines; it behaves the same otherwise.
#
"""Report one mobile build-pipeline event to the board (mobile-board Cloudflare Worker).

Replaces slack-root.py, slack-reply.sh and pr-card.py: the board stores the event, renders the Slack
root card + thread replies and keeps the PR build-card comment current. Each leg only reports what it
knows; the board merges non-empty fields (each leg owns its own) and never lets `status` regress from
failed/cancelled to success. Best-effort: never fails the build — a rejected / unreported event or a
board-side Slack / PR-card error surfaces as a `::warning` annotation on the run, then exit 0. Retries
429/5xx/connection errors; the event id makes retries idempotent.

Env (only NON-EMPTY values are sent):
  BUILD_REPORTER_TOKEN  required — no-op otherwise (secret; absent on fork PRs)
  BOARD_EVENTS_URL      required — board base URL, from the repository variable of the same name.
                        CI uses the Worker's workers.dev host because the custom domain's Bot Fight
                        Mode returns 403 to GitHub-hosted runners. Unset → warning annotation, no report.
  SOURCE                godot-explorer (default) | godot-asc-deploy
  RUN_ID / RUN_ATTEMPT  godot-explorer only: the distribution run that owns the pipeline (default
                        $GITHUB_RUN_ID / $GITHUB_RUN_ATTEMPT — a called workflow shares the caller's).
                        godot-asc-deploy events are attached by SHA to the run that built the commit;
                        that repo's own run id travels as `source_run_id` (reference only, never a key).
  JOB                   default $GITHUB_JOB
  SHA, REF, PR_NUMBER   build context (REF falls back to BRANCH). godot-asc-deploy needs a 40-hex SHA:
                        a dispatch with sha=latest reports nothing.
  RESET                 1 → start a fresh card for this run (prepare only)
  STATUS                building | success | failed | cancelled (only set it when you mean to)
  BUILD_NUMBER, BUILD_VERSION, BRANCH, TRIGGERED_BY, PR_URL, COMMIT, COMMIT_URL,
  IOS_LINE, ANDROID_LINE, APK_URL, AAB_URL, TESTFLIGHT_URL, RUN_URL      card fields
                        (RUN_URL is the card's "View run" link: the godot-explorer run, never another repo's)
  MESSAGE               one Slack thread reply
  LOG                   one PR-card timeline entry

Writes slack_ts / slack_channel / run_id to $GITHUB_OUTPUT when the board returns them.
"""
import hashlib, json, os, re, sys, time, urllib.error, urllib.request

# Contract with the board: this list must stay a SUBSET of mobile-board's server/build-card-state.js
# CARD_FIELDS. The board rejects the WHOLE event (400) on an unknown field, so deploy the board first.
CARD_FIELDS = ["status", "build_number", "build_version", "branch", "triggered_by", "pr_url", "commit",
               "commit_url", "ios_line", "android_line", "apk_url", "aab_url", "testflight_url", "run_url"]
SHA_RE = re.compile(r"[0-9a-fA-F]{40}")

def env(k, d=""):
    # Single line only — the board stores every value as one line.
    return " ".join(os.environ.get(k, d).split())

token = env("BUILD_REPORTER_TOKEN")
if not token:
    print("report-build-event: BUILD_REPORTER_TOKEN not set — skipping (fork PR / local)"); sys.exit(0)
print(f"::add-mask::{token}")
base = env("BOARD_EVENTS_URL").rstrip("/")
if not base:
    print("::warning title=report-build-event::BOARD_EVENTS_URL is not set (repository variable) — "
          "this event was not reported to the board (build unaffected)"); sys.exit(0)
source = env("SOURCE") or "godot-explorer"
repo = env("GITHUB_REPOSITORY") or "decentraland/godot-explorer"
server = env("GITHUB_SERVER_URL") or "https://github.com"
own_run_id = env("GITHUB_RUN_ID")
own_attempt = env("GITHUB_RUN_ATTEMPT") or "1"
job = env("JOB") or env("GITHUB_JOB")
sha = env("SHA") if SHA_RE.fullmatch(env("SHA")) else ""

event = {"source": source, "repository": "decentraland/godot-explorer",
         "job": job, "reset": env("RESET") in ("1", "true")}
if source == "godot-explorer":
    run_id = env("RUN_ID") or own_run_id
    run_attempt = env("RUN_ATTEMPT") or own_attempt
    if not run_id:
        print("report-build-event: no run id — skipping"); sys.exit(0)
    event["run_id"] = run_id
    event["run_attempt"] = int(run_attempt) if run_attempt.isdigit() else 1
    event["run_url"] = f"{server}/{repo}/actions/runs/{run_id}"
else:
    # The board attaches this event to the godot-explorer run that built the commit (it rejects a
    # run_id from this source). Our own run id is sent for reference only.
    run_id, run_attempt = own_run_id, own_attempt
    if not sha:
        print("report-build-event: no 40-hex commit sha (dispatched with sha=latest?) — "
              "the board cannot attach this event, skipping"); sys.exit(0)
    if own_run_id:
        event["source_run_id"] = own_run_id
        event["source_run_url"] = f"{server}/{repo}/actions/runs/{own_run_id}"
if sha:
    event["sha"] = sha
for key, name in (("pr_number", "PR_NUMBER"), ("message", "MESSAGE"), ("log", "LOG")):
    if env(name):
        event[key] = env(name)
if env("REF") or env("BRANCH"):
    event["ref"] = env("REF") or env("BRANCH")
card = {k: env(k.upper()) for k in CARD_FIELDS if env(k.upper())}
if card:
    event["card"] = card
if not event["reset"] and not card and "message" not in event and "log" not in event:
    print("report-build-event: nothing to report — skipping"); sys.exit(0)
# Idempotency key: same run attempt + job + payload → the board stores it once (safe retries).
digest = hashlib.sha1(json.dumps(event, sort_keys=True, ensure_ascii=False).encode()).hexdigest()[:10]
event["event_id"] = f"{run_id or 'manual'}-{run_attempt}-{job or 'job'}-{digest}"

req = urllib.request.Request(f"{base}/api/builds/events", data=json.dumps(event, ensure_ascii=False).encode(),
    method="POST", headers={"Authorization": f"Bearer {token}", "Content-Type": "application/json",
                            "User-Agent": "godot-explorer-ci"})
backoff = 2
for attempt in range(1, 4):
    try:
        with urllib.request.urlopen(req, timeout=20) as r:
            data = json.loads(r.read() or b"{}")
        pipeline = data.get("pipeline") or {}
        slack = data.get("slack") or {}
        print(f"report-build-event: ok run={data.get('run_id')} v{data.get('version')} status={pipeline.get('status')}"
              f"{' stale' if data.get('stale') else ''}{' duplicate' if data.get('duplicate') else ''}")
        for k in ("slack_error", "card_error"):  # stored, but the board's Slack / PR-card write failed (its cron retries)
            if pipeline.get(k):
                print(f"::warning title=report-build-event::board {k}: {pipeline[k]}")
        gho = os.environ.get("GITHUB_OUTPUT")
        if gho:
            with open(gho, "a") as f:
                f.write(f"slack_ts={slack.get('ts', '')}\nslack_channel={slack.get('channel', '')}\nrun_id={data.get('run_id', '')}\n")
        sys.exit(0)
    except urllib.error.HTTPError as e:
        body = " ".join(e.read().decode(errors="replace")[:300].split())
        if e.code in (429,) or e.code >= 500:
            print(f"report-build-event: attempt {attempt}/3 HTTP {e.code}: {body}", file=sys.stderr)
        else:  # 400 / 401 / 404: visible on the run, but never fails the build
            print(f"::warning title=report-build-event::board rejected this event: HTTP {e.code} {body}"); sys.exit(0)
    except Exception as e:  # URLError, timeout, bad JSON
        print(f"report-build-event: attempt {attempt}/3 failed: {e}", file=sys.stderr)
    if attempt < 3:
        time.sleep(backoff); backoff *= 2
print("::warning title=report-build-event::giving up — the board did not record this event (build unaffected)")
sys.exit(0)
