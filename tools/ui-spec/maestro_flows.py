#!/usr/bin/env python3
"""Writes one Maestro flow per gallery id: cold-start the preview deep link, wait for the screen to settle, screenshot.

  python3 tools/ui-spec/maestro_flows.py FLOW_DIR --source a,b | --ids x,y
  maestro test --test-output-dir SHOTS_DIR FLOW_DIR

A flow per id (not one long flow) so a screen that never gets ready fails alone and the rest still run.
"""
import argparse, json, os

ROOT = os.path.abspath(os.path.join(os.path.dirname(__file__), "..", ".."))
IDS_JSON = os.path.join(ROOT, "rn", "src", "gallery", "ids.json")
APP_ID = "org.decentraland.godotexplorer"
READY_TIMEOUT_MS = 30000

FLOW = """appId: {app}
---
- stopApp
- openLink: dclrn://preview/{id}
# A starved emulator can make the launcher ANR; its dialog covers the app.
- runFlow:
    when:
      visible: ".*isn't responding.*"
    commands:
      - tapOn: "Wait"
- extendedWaitUntil:
    visible:
      id: preview-ready
    timeout: {timeout}
- takeScreenshot: {shot}
"""


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("flow_dir")
    group = ap.add_mutually_exclusive_group(required=True)
    group.add_argument("--source")
    group.add_argument("--ids")
    args = ap.parse_args()

    rows = json.load(open(IDS_JSON))
    if args.ids:
        wanted = set(args.ids.split(","))
        rows = [r for r in rows if r["id"] in wanted]
    else:
        sources = set(args.source.split(","))
        rows = [r for r in rows if r["source"] in sources]

    os.makedirs(args.flow_dir, exist_ok=True)
    for row in rows:
        ident = row["id"]
        flow = FLOW.format(app=APP_ID, id=ident, timeout=READY_TIMEOUT_MS, shot=ident)
        name = ident.replace("/", "__")
        with open(os.path.join(args.flow_dir, name + ".yaml"), "w") as f:
            f.write(flow)
    print(f"{len(rows)} flows -> {args.flow_dir}")


if __name__ == "__main__":
    main()
