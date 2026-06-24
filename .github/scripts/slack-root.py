#!/usr/bin/env python3
"""Create or update the Slack "root" status card for the mobile build pipeline.

Merge-on-read: the full card state is stored in the message `metadata`. On update we READ
the current state (needs channels:history / groups:history), merge in ONLY the fields this
call provides (non-empty env vars), re-render, and chat.update. So each leg owns its own
fields (e.g. Android owns ANDROID_LINE/APK_URL, iOS owns IOS_LINE/build number) and no leg
ever clobbers another's. Best-effort: never fails the build.

Env (only NON-EMPTY values are merged; everything else is preserved):
  SLACK_BOT_TOKEN, SLACK_CHANNEL   required
  SLACK_TS                         if set -> read+merge+chat.update; else chat.postMessage
  STATUS                           building | success | failed   (only set it when you mean to)
  BUILD_VERSION, TRIGGERED_BY
  PR_NUMBER, PR_URL, COMMIT, COMMIT_URL
  IOS_LINE, ANDROID_LINE, APK_URL, RUN_URL, TESTFLIGHT_URL

On create prints `ts=<ts>` and appends ts/channel to $GITHUB_OUTPUT.
"""
import os, sys, json, urllib.request, urllib.parse

API = "https://slack.com/api"
FIELDS = ["status", "build_version", "triggered_by", "pr_number", "pr_url",
          "commit", "commit_url", "ios_line", "android_line", "apk_url",
          "run_url", "testflight_url"]

def env(k, d=""): return os.environ.get(k, d).strip()

token, channel = env("SLACK_BOT_TOKEN"), env("SLACK_CHANNEL")
if not token or not channel:
    print("slack-root: no token/channel — skipping"); sys.exit(0)
ts = env("SLACK_TS")

def slack(method, payload):
    req = urllib.request.Request(f"{API}/{method}", data=json.dumps(payload).encode(),
        headers={"Authorization": f"Bearer {token}", "Content-type": "application/json"})
    return json.load(urllib.request.urlopen(req))

def read_state():
    """Fetch the current card state from the message metadata (best-effort)."""
    q = urllib.parse.urlencode({"channel": channel, "latest": ts, "limit": 1,
                                "inclusive": "true", "include_all_metadata": "true"})
    req = urllib.request.Request(f"{API}/conversations.history?{q}",
        headers={"Authorization": f"Bearer {token}"})
    try:
        d = json.load(urllib.request.urlopen(req))
        msgs = d.get("messages", []) if d.get("ok") else []
        if msgs and msgs[0].get("ts") == ts:
            return dict(msgs[0].get("metadata", {}).get("event_payload", {}) or {})
        print(f"slack-root: could not read prior state (ok={d.get('ok')} err={d.get('error')})", file=sys.stderr)
    except Exception as e:
        print(f"slack-root: read failed: {e}", file=sys.stderr)
    return {}

# Fields this call wants to set (non-empty only) — everything else is preserved.
incoming = {k: env(k.upper()) for k in FIELDS if env(k.upper())}

if ts:
    state = read_state()
    state.update(incoming)
else:
    state = {"status": "building"}   # baseline for a fresh card
    state.update(incoming)

status = (state.get("status") or "building").lower()
emoji = {"building": "⏳", "success": "✅", "failed": "❌", "cancelled": "⚠️"}.get(status, "⏳")
label = {"building": "BUILDING", "success": "SUCCESS", "failed": "FAILED", "cancelled": "CANCELLED"}.get(status, status.upper())
bv = state.get("build_version") or "—"
trig = state.get("triggered_by") or "—"
commit, commit_url = state.get("commit", ""), state.get("commit_url", "")
pr_number, pr_url = state.get("pr_number", ""), state.get("pr_url", "")
ios_line, android_line = state.get("ios_line", ""), state.get("android_line", "")
apk_url = state.get("apk_url", "")
tf_url = state.get("testflight_url") or "https://appstoreconnect.apple.com/apps"
run_url = state.get("run_url", "")

commit_md = f"<{commit_url}|`{commit}`>" if (commit_url and commit) else (f"`{commit}`" if commit else "—")
pr_md = f"<{pr_url}|#{pr_number}>" if (pr_url and pr_number) else (f"#{pr_number}" if pr_number else "—")

blocks = [
    {"type": "header", "text": {"type": "plain_text",
        "text": f"🤖 Mobile build pipeline · {emoji} {label} 🍏", "emoji": True}},
    {"type": "section", "fields": [
        {"type": "mrkdwn", "text": f"*Build version:*\n`{bv}`"},
        {"type": "mrkdwn", "text": f"*Triggered by:*\n{trig}"},
        {"type": "mrkdwn", "text": f"*PR:*\n{pr_md}"},
        {"type": "mrkdwn", "text": f"*Commit:*\n{commit_md}"},
    ]},
]
plat = []
if ios_line: plat.append(f"*iOS:* {ios_line}")
if android_line: plat.append(f"*Android:* {android_line}")
if plat:
    blocks.append({"type": "section", "text": {"type": "mrkdwn", "text": "      ".join(plat)}})

elements = []
if apk_url:
    elements.append({"type": "button", "style": "primary",
        "text": {"type": "plain_text", "text": "📱 Download APK", "emoji": True}, "url": apk_url})
if status == "success":
    elements.append({"type": "button",
        "text": {"type": "plain_text", "text": "🍏 TestFlight", "emoji": True}, "url": tf_url})
if pr_url:
    elements.append({"type": "button",
        "text": {"type": "plain_text", "text": "🔗 PR", "emoji": True}, "url": pr_url})
if run_url:
    elements.append({"type": "button",
        "text": {"type": "plain_text", "text": "View run", "emoji": True}, "url": run_url})
if elements:
    blocks.append({"type": "actions", "elements": elements})

payload = {"channel": channel, "blocks": blocks, "text": f"Mobile build pipeline {label}",
           "metadata": {"event_type": "mobile_build_pipeline", "event_payload": state}}
method = "chat.update" if ts else "chat.postMessage"
if ts:
    payload["ts"] = ts

try:
    resp = slack(method, payload)
except Exception as e:
    print(f"slack-root: {method} failed: {e}", file=sys.stderr); sys.exit(0)
if not resp.get("ok"):
    print(f"slack-root: {method} error: {resp.get('error')}", file=sys.stderr); sys.exit(0)

new_ts = resp.get("ts", "")
print(f"ts={new_ts}")
gho = os.environ.get("GITHUB_OUTPUT")
if gho and new_ts:
    with open(gho, "a") as f:
        f.write(f"ts={new_ts}\nchannel={channel}\n")
