#!/usr/bin/env python3
"""Report one mobile build-pipeline event to the board (mobile-board Cloudflare Worker).

Replaces slack-root.py, slack-reply.sh and pr-card.py: the board stores the event, renders the Slack
root card + thread replies and keeps the PR build-card comment current. Each leg only reports what it
knows; the board merges non-empty fields (each leg owns its own) and never lets `status` regress from
failed/cancelled to success. Best-effort: never fails the build (missing token / HTTP error → print,
exit 0). Retries 429/5xx/connection errors; the event id makes retries idempotent.

Env (only NON-EMPTY values are sent):
  BUILD_REPORTER_TOKEN  required — no-op otherwise
  BOARD_EVENTS_URL      board base URL (default: the Worker's workers.dev host — the custom domain's
                        Bot Fight Mode returns 403 to GitHub-hosted runners)
  SOURCE                godot-explorer (default) | godot-asc-deploy
  RUN_ID / RUN_ATTEMPT  default $GITHUB_RUN_ID / $GITHUB_RUN_ATTEMPT (a called workflow shares the caller's)
  JOB                   default $GITHUB_JOB
  SHA, REF, PR_NUMBER   build context (REF falls back to BRANCH)
  RESET                 1 → start a fresh card for this run (prepare only)
  STATUS                building | success | failed | cancelled (only set it when you mean to)
  BUILD_NUMBER, BUILD_VERSION, BRANCH, TRIGGERED_BY, PR_URL, COMMIT, COMMIT_URL,
  IOS_LINE, ANDROID_LINE, APK_URL, AAB_URL, TESTFLIGHT_URL, RUN_URL      card fields
  MESSAGE               one Slack thread reply
  LOG                   one PR-card timeline entry

Writes slack_ts / slack_channel / run_id to $GITHUB_OUTPUT when the board returns them.
"""
import hashlib, json, os, sys, time, urllib.error, urllib.request

DEFAULT_URL = "https://mobile-board.mateo-a2b.workers.dev"
CARD_FIELDS = ["status", "build_number", "build_version", "branch", "triggered_by", "pr_url", "commit",
               "commit_url", "ios_line", "android_line", "apk_url", "aab_url", "testflight_url", "run_url"]

def env(k, d=""):
    # Single line only — the board stores every value as one line.
    return " ".join(os.environ.get(k, d).split())

token = env("BUILD_REPORTER_TOKEN")
if not token:
    print("report-build-event: BUILD_REPORTER_TOKEN not set — skipping (fork PR / local)"); sys.exit(0)
print(f"::add-mask::{token}")
base = (env("BOARD_EVENTS_URL") or DEFAULT_URL).rstrip("/")
repo = env("GITHUB_REPOSITORY") or "decentraland/godot-explorer"
run_id = env("RUN_ID") or env("GITHUB_RUN_ID")
run_attempt = env("RUN_ATTEMPT") or env("GITHUB_RUN_ATTEMPT") or "1"
job = env("JOB") or env("GITHUB_JOB")
server = env("GITHUB_SERVER_URL") or "https://github.com"

event = {"source": env("SOURCE") or "godot-explorer", "repository": "decentraland/godot-explorer",
         "job": job, "reset": env("RESET") in ("1", "true")}
if run_id:
    event["run_id"] = run_id
    event["run_attempt"] = int(run_attempt) if run_attempt.isdigit() else 1
    event["run_url"] = f"{server}/{repo}/actions/runs/{run_id}"
for key, name in (("sha", "SHA"), ("pr_number", "PR_NUMBER"), ("message", "MESSAGE"), ("log", "LOG")):
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
              f"{' stale' if data.get('stale') else ''}{' duplicate' if data.get('duplicate') else ''}"
              f"{' slack_error=' + pipeline['slack_error'] if pipeline.get('slack_error') else ''}"
              f"{' card_error=' + pipeline['card_error'] if pipeline.get('card_error') else ''}")
        gho = os.environ.get("GITHUB_OUTPUT")
        if gho:
            with open(gho, "a") as f:
                f.write(f"slack_ts={slack.get('ts', '')}\nslack_channel={slack.get('channel', '')}\nrun_id={data.get('run_id', '')}\n")
        sys.exit(0)
    except urllib.error.HTTPError as e:
        body = e.read().decode(errors="replace")[:300]
        if e.code in (429,) or e.code >= 500:
            print(f"report-build-event: attempt {attempt}/3 HTTP {e.code}: {body}", file=sys.stderr)
        else:
            print(f"report-build-event: HTTP {e.code} (not retried): {body}", file=sys.stderr); sys.exit(0)
    except Exception as e:  # URLError, timeout, bad JSON
        print(f"report-build-event: attempt {attempt}/3 failed: {e}", file=sys.stderr)
    if attempt < 3:
        time.sleep(backoff); backoff *= 2
print("report-build-event: giving up — the board did not record this event (build unaffected)", file=sys.stderr)
sys.exit(0)
