#!/usr/bin/env bash
#
# ⚠️ DUPLICATED FILE — keep in sync with the identical copy in the other repo.
# The mobile build pipeline spans two repos, so this script lives in BOTH
# decentraland/godot-explorer and decentraland/godot-asc-deploy. Edit both copies together.
#
# Post a Slack message as a threaded reply (best-effort).
#   usage: slack-reply.sh "<text>"
# Reads SLACK_BOT_TOKEN, SLACK_CHANNEL, SLACK_TS from the environment. No-op (exit 0) if
# token or channel is missing, so it never fails the build. If SLACK_TS is set the message
# is a reply in that thread; otherwise it posts standalone.
set -euo pipefail

TEXT="${1:?text required}"
TOKEN="${SLACK_BOT_TOKEN:-}"
CH="${SLACK_CHANNEL:-}"
TS="${SLACK_TS:-}"

if [ -z "$TOKEN" ] || [ -z "$CH" ]; then
  echo "slack: no token/channel — skipping post"
  exit 0
fi

BODY=$(TEXT="$TEXT" CH="$CH" TS="$TS" python3 - <<'PY'
import json, os
d = {"channel": os.environ["CH"], "text": os.environ["TEXT"]}
ts = os.environ.get("TS")
if ts:
    d["thread_ts"] = ts
print(json.dumps(d))
PY
)

curl -s -X POST https://slack.com/api/chat.postMessage \
  -H "Authorization: Bearer $TOKEN" -H 'Content-type: application/json' \
  --data "$BODY" \
  | python3 -c 'import sys,json; d=json.load(sys.stdin); print("slack ok=", d.get("ok"), d.get("error",""))'
