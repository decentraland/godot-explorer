#!/usr/bin/env python3
"""Create or update the Slack "root" status card for the mobile build pipeline.

Best-effort: never fails the build (exits 0 even on Slack errors).
Reads everything from the environment so it can be reused across repos.

  SLACK_BOT_TOKEN, SLACK_CHANNEL   required
  SLACK_TS                         if set -> chat.update that message; else chat.postMessage
  STATUS                           building | success | failed   (default building)
  BUILD_VERSION, TRIGGERED_BY      strings
  PR_NUMBER, PR_URL                PR link
  COMMIT, COMMIT_URL               commit link
  IOS_LINE, ANDROID_LINE           per-platform status text (e.g. "✅ TestFlight build 624")
  APK_URL                          if set -> "Download APK" button
  TESTFLIGHT_URL, RUN_URL          extra buttons

On create it prints `ts=<ts>` and appends ts/channel to $GITHUB_OUTPUT.
"""
import os, sys, json, urllib.request

def env(k, d=""): return os.environ.get(k, d).strip()

token, channel = env("SLACK_BOT_TOKEN"), env("SLACK_CHANNEL")
if not token or not channel:
    print("slack-root: no token/channel — skipping"); sys.exit(0)

ts = env("SLACK_TS")
status = env("STATUS", "building").lower()
emoji = {"building": "⏳", "success": "✅", "failed": "❌"}.get(status, "⏳")
label = {"building": "BUILDING", "success": "SUCCESS", "failed": "FAILED"}.get(status, status.upper())

bv = env("BUILD_VERSION") or "—"
trig = env("TRIGGERED_BY") or "—"
commit, commit_url = env("COMMIT"), env("COMMIT_URL")
pr_number, pr_url = env("PR_NUMBER"), env("PR_URL")
ios_line, android_line = env("IOS_LINE"), env("ANDROID_LINE")
apk_url = env("APK_URL")
tf_url = env("TESTFLIGHT_URL", "https://appstoreconnect.apple.com/apps")
run_url = env("RUN_URL")

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

payload = {"channel": channel, "blocks": blocks, "text": f"Mobile build pipeline {label}"}
method = "chat.update" if ts else "chat.postMessage"
if ts:
    payload["ts"] = ts

req = urllib.request.Request(f"https://slack.com/api/{method}",
    data=json.dumps(payload).encode(),
    headers={"Authorization": f"Bearer {token}", "Content-type": "application/json"})
try:
    resp = json.load(urllib.request.urlopen(req))
except Exception as e:
    print(f"slack-root: request failed: {e}", file=sys.stderr); sys.exit(0)

if not resp.get("ok"):
    print(f"slack-root: error: {resp.get('error')}", file=sys.stderr); sys.exit(0)

new_ts = resp.get("ts", "")
print(f"ts={new_ts}")
gho = os.environ.get("GITHUB_OUTPUT")
if gho and new_ts:
    with open(gho, "a") as f:
        f.write(f"ts={new_ts}\nchannel={channel}\n")
