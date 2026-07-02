#!/usr/bin/env bash
# ONE-STEP debug-hub connect. Run this with the Bash tool's run_in_background:true
# and read its output for the verdict, then query with unified.sh.
#
# It: wires Android adb-reverse, ensures a hub is up (reuse or start), waits for
# the device to dial in, prints CONNECTED + a ping snapshot (or a relaunch hint),
# and — if it started the hub — keeps this task alive so the hub persists for
# follow-up `unified.sh` queries. Idempotent; safe to re-run.
set -uo pipefail
HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT="$(git -C "$HERE" rev-parse --show-toplevel 2>/dev/null || echo "$HERE/../../../..")"
DEV=9231 CON=9230

# 1. Android attached? wire the loopback reverse — debug builds dial 127.0.0.1:9231
#    (global.gd default), and adb-reverse forwards that to this Mac's hub.
if command -v adb >/dev/null 2>&1 \
	&& [ -n "$(adb devices 2>/dev/null | awk 'NR>1 && $2=="device"{print;exit}')" ]; then
	adb reverse tcp:$DEV tcp:$DEV >/dev/null 2>&1 && echo "· adb reverse tcp:$DEV -> hub (Android)"
fi

# 2. Ensure a hub. Reuse one that's already listening, else start it under THIS task.
started=""
if lsof -nP -iTCP:$DEV -sTCP:LISTEN >/dev/null 2>&1; then
	echo "· hub: reusing existing (device :$DEV / consumer :$CON)"
else
	echo "· hub: starting 'cargo run -- debug-hub' ..."
	(cd "$ROOT" && exec cargo run -- debug-hub) &
	started=$!
	for _ in $(seq 1 90); do
		lsof -nP -iTCP:$DEV -sTCP:LISTEN >/dev/null 2>&1 && break
		sleep 1
	done
fi
lsof -nP -iTCP:$DEV -sTCP:LISTEN >/dev/null 2>&1 || {
	echo "x hub failed to bind :$DEV"
	exit 1
}
echo "· hub: LISTEN on :$DEV (consumers :$CON)"

# 3. Wait for a device to dial in (the client retries with <=30s backoff, so a hub
#    started late still gets picked up without an app restart).
echo "· waiting for a device to connect (<=35s) ..."
connected=""
for _ in $(seq 1 35); do
	if lsof -nP -iTCP:$DEV 2>/dev/null | grep -q ESTABLISHED; then
		connected=1
		break
	fi
	sleep 1
done

# 4. Verdict.
if [ -n "$connected" ]; then
	echo "=== CONNECTED ==="
	REPLY_TIMEOUT=6 "$HERE/unified.sh" ping || true
else
	cat <<EOF
=== NO DEVICE after 35s — the app isn't dialing this hub ===
Relaunch it pointing at the hub; it connects on the next backoff tick:
  - Android editor-deploy: needs a DEBUG build with the loopback default
    (global.gd) -> rebuild + redeploy, or just: cargo run -- run --target android
  - iOS: relaunch the app (editor deploy auto-bakes ws://<LAN>:9231; accept the
    Local Network prompt)
  - desktop: cargo run -- run -- --scene-inspector=ws://127.0.0.1:9231
(the hub stays up — re-check with: $HERE/unified.sh ping)
EOF
fi

# 5. If we started the hub, keep this task alive so it persists for queries.
[ -n "$started" ] && wait "$started"
