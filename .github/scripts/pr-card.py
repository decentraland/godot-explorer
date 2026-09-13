#!/usr/bin/env python3
"""Create or update the ONE "mobile build pipeline" status card comment on a PR.

The GitHub twin of slack-root.py. A single sticky PR comment (found via a hidden HTML marker)
mirrors the Slack root card — build number/version, branch, commit, trigger, per-platform state,
artifact links — plus a collapsed timeline that mirrors the Slack thread replies. Unlike the Slack
scripts this one lives ONLY here: godot-asc-deploy holds no token for this repo, so the iOS
outcome is mirrored by mobile_distribute's ios-wait job rather than written from there.

Merge-on-read, like Slack: the full card state is stored as JSON inside a hidden HTML comment at
the bottom of the body. On update we READ the current state, merge in ONLY the fields this call
provides (non-empty env vars), re-render the whole body and PATCH it. So each leg owns its own
fields (Android owns ANDROID_LINE/APK_URL/AAB_URL, iOS owns IOS_LINE/TESTFLIGHT_URL) and no leg
ever clobbers another's. Best-effort: never fails the build (no token / fork PR / API error →
print and exit 0).

Env (only NON-EMPTY values are merged; everything else is preserved):
  GH_TOKEN, PR_NUMBER              required — no-op otherwise (GITHUB_TOKEN is read as a fallback)
  GH_REPO                          owner/repo the PR lives in (default: $GITHUB_REPOSITORY)
  SHA                              commit this update is about. If the card already tracks a
                                   DIFFERENT commit the update is dropped: a leg of a superseded
                                   run (re-label on a newer commit) must not overwrite the card.
  RUN_ID                           workflow run this update belongs to ($GITHUB_RUN_ID). Same idea
                                   for a re-label of the SAME commit: a leg of the cancelled run
                                   must not overwrite the new run's card.
  RESET                            1 → start from a blank state (a new distribution; prepare only)
  STATUS                           building | success | failed | cancelled (only set it when you mean to)
  BUILD_NUMBER, BUILD_VERSION, BRANCH, TRIGGERED_BY, COMMIT, COMMIT_URL
  IOS_LINE, ANDROID_LINE, APK_URL, AAB_URL, TESTFLIGHT_URL, RUN_URL
  LOG                              one line to append to the timeline (timestamped here)
  PR_CARD_DRY_RUN                  1 → print the rendered body instead of calling GitHub
"""
import json, os, re, sys, time, urllib.error, urllib.request
from datetime import datetime, timezone

API = os.environ.get("GITHUB_API_URL", "").rstrip("/") or "https://api.github.com"
MARKER = "<!-- mobile-build-card -->"
STATE_RE = re.compile(r"<!-- mobile-build-card:state (\{.*\}) -->", re.S)
# The pre-card "distribution triggered" comment: upgraded in place so older PRs don't get a 2nd card.
LEGACY_MARKER = "### 📱 Mobile Distribution Triggered"
FIELDS = ["status", "build_number", "build_version", "branch", "triggered_by", "commit",
          "commit_url", "ios_line", "android_line", "apk_url", "aab_url", "testflight_url",
          "run_url"]
MAX_LOG = 40

def env(k, d=""):
    # Single line only: a newline would break the one-line hidden state comment.
    return " ".join(os.environ.get(k, d).split())

token = env("GH_TOKEN") or env("GITHUB_TOKEN")
repo = env("GH_REPO") or env("GITHUB_REPOSITORY")
pr = env("PR_NUMBER")
dry_run = env("PR_CARD_DRY_RUN") in ("1", "true")
if not pr or (not dry_run and not (token and repo)):
    print("pr-card: no token/repo/PR number — skipping"); sys.exit(0)

def gh(method, path, payload=None):
    data = json.dumps(payload).encode() if payload is not None else None
    req = urllib.request.Request(f"{API}{path}", data=data, method=method, headers={
        "Authorization": f"Bearer {token}", "Accept": "application/vnd.github+json",
        "X-GitHub-Api-Version": "2022-11-28", "Content-Type": "application/json"})
    for attempt in (1, 2):
        try:
            with urllib.request.urlopen(req, timeout=30) as r:
                body = r.read()
                return (json.loads(body) if body else {}), r.headers
        except urllib.error.HTTPError as e:
            if e.code >= 500 and attempt == 1:
                time.sleep(3); continue
            raise
        except urllib.error.URLError:
            if attempt == 1:
                time.sleep(3); continue
            raise

def find_comment():
    """(id, body) of the existing card — or of the legacy comment to upgrade — else (None, None)."""
    legacy = (None, None)
    page = 1
    while True:
        items, headers = gh("GET", f"/repos/{repo}/issues/{pr}/comments?per_page=100&page={page}")
        for c in items:
            body = c.get("body") or ""
            if MARKER in body:
                return c["id"], body
            if legacy[0] is None and LEGACY_MARKER in body:
                legacy = (c["id"], body)
        if 'rel="next"' not in (headers.get("Link") or ""):
            return legacy
        page += 1

def parse_state(body):
    m = STATE_RE.search(body or "")
    if not m:
        return {}
    try:
        return json.loads(m.group(1))
    except Exception as e:
        print(f"pr-card: could not parse prior state ({e}) — starting fresh", file=sys.stderr)
        return {}

# ---- read + merge -------------------------------------------------------------------------
reset = env("RESET") in ("1", "true")
cid, prev_body = (None, None)
if not dry_run:
    try:
        cid, prev_body = find_comment()
    except Exception as e:
        print(f"pr-card: could not list comments: {e}", file=sys.stderr); sys.exit(0)

state = {} if reset else parse_state(prev_body)
sha, run_id = env("SHA"), env("RUN_ID")
if not reset:
    if sha and state.get("sha") and state["sha"] != sha:
        print(f"pr-card: card tracks {state['sha'][:7]}, this update is for {sha[:7]} — stale run, skipping")
        sys.exit(0)
    if run_id and state.get("run_id") and state["run_id"] != run_id:
        print(f"pr-card: card belongs to run {state['run_id']}, this update is from run {run_id} — stale run, skipping")
        sys.exit(0)
if sha:
    state["sha"] = sha
if run_id:
    state["run_id"] = run_id
state.update({k: env(k.upper()) for k in FIELDS if env(k.upper())})

now = datetime.now(timezone.utc)
log = env("LOG")
if log:
    state["log"] = (state.get("log") or []) + [f"`{now:%H:%M} UTC` {log}"]
    state["log"] = state["log"][-MAX_LOG:]

# ---- render -------------------------------------------------------------------------------
status = (state.get("status") or "building").lower()
emoji = {"building": "⏳", "success": "✅", "failed": "❌", "cancelled": "⚠️"}.get(status, "⏳")
label = status.upper()
bn = state.get("build_number") or "—"
bv = state.get("build_version") or "—"
branch, trig = state.get("branch", ""), state.get("triggered_by") or "—"
commit, commit_url = state.get("commit", ""), state.get("commit_url", "")
ios_line, android_line = state.get("ios_line") or "—", state.get("android_line") or "—"
apk_url, aab_url = state.get("apk_url", ""), state.get("aab_url", "")
tf_url, run_url = state.get("testflight_url", ""), state.get("run_url", "")

def link(text, url): return f"[{text}]({url})" if url else ""
commit_md = f"[`{commit}`]({commit_url})" if (commit_url and commit) else (f"`{commit}`" if commit else "—")
branch_md = f"`{branch}`" if branch else "—"
ios_art = link("🍏 TestFlight", tf_url) or "—"
android_art = " · ".join(x for x in (link("📱 Download APK", apk_url), link("📦 AAB", aab_url)) if x) or "—"

lines = [
    MARKER,
    f"## 📱 Mobile build pipeline · {emoji} {label}",
    "",
    "| | |",
    "|:--|:--|",
    f"| **Build number** | `{bn}` |",
    f"| **Build version** | `{bv}` |",
    f"| **Branch** | {branch_md} @ {commit_md} |",
    f"| **Triggered by** | {trig} |",
    "",
    "| Platform | Status | Artifact |",
    "|:--|:--|:--|",
    f"| 🍏 iOS | {ios_line} | {ios_art} |",
    f"| 🤖 Android | {android_line} | {android_art} |",
    "",
]
if run_url:
    lines += [f"🔗 {link('View run', run_url)}", ""]
if state.get("log"):
    lines += ["<details>", "<summary>Timeline</summary>", ""]
    lines += [f"- {entry}" for entry in state["log"]]
    lines += ["", "</details>", ""]
lines += [f"<sub>🔄 Updated {now:%Y-%m-%d %H:%M:%S} UTC</sub>", ""]

# Hidden state. GitHub's markdown treats `--` inside an HTML comment as "not a comment" (it would
# render as text), and `>` could close it — escape both inside the JSON (still valid JSON).
state_json = json.dumps(state, ensure_ascii=False, separators=(",", ":"))
state_json = state_json.replace(">", "\\u003e").replace("--", "-\\u002d")
lines.append(f"<!-- mobile-build-card:state {state_json} -->")
body = "\n".join(lines)

# ---- write --------------------------------------------------------------------------------
if dry_run:
    print(body); sys.exit(0)
try:
    if cid:
        gh("PATCH", f"/repos/{repo}/issues/comments/{cid}", {"body": body})
        print(f"pr-card: updated comment {cid} on {repo}#{pr}")
    else:
        resp, _ = gh("POST", f"/repos/{repo}/issues/{pr}/comments", {"body": body})
        print(f"pr-card: created comment {resp.get('id')} on {repo}#{pr}")
except urllib.error.HTTPError as e:
    # 403 on fork PRs (read-only GITHUB_TOKEN) is expected — never fail the build over the card.
    print(f"pr-card: GitHub API {e.code} {e.reason} — skipping", file=sys.stderr)
except Exception as e:
    print(f"pr-card: write failed: {e} — skipping", file=sys.stderr)
