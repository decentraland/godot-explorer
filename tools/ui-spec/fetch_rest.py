#!/usr/bin/env python3
"""Download every missing manifest screenshot through the Figma REST API (batched, no MCP quota).

  python3 tools/ui-spec/fetch_rest.py                 # all missing
  python3 tools/ui-spec/fetch_rest.py --source menu   # one source
  python3 tools/ui-spec/fetch_rest.py --force         # re-download everything

Token: $FIGMA_TOKEN, else the first line of ~/.config/figma-token (file_content:read scope).
"""
import argparse, json, os, sys, time, urllib.error, urllib.parse, urllib.request
from collections import defaultdict
from PIL import Image

ROOT = os.path.join(os.path.dirname(os.path.abspath(__file__)), "..", "..", "docs", "rn-ui-spec")
BATCH = 12


def token():
    t = os.environ.get("FIGMA_TOKEN")
    if not t:
        p = os.path.expanduser("~/.config/figma-token")
        t = open(p).readline().strip() if os.path.exists(p) else ""
    if not t:
        sys.exit("no token: set FIGMA_TOKEN or write ~/.config/figma-token")
    return t


def get(url, tok=None, tries=6):
    req = urllib.request.Request(url, headers={"X-Figma-Token": tok} if tok else {})
    for i in range(tries):
        try:
            return urllib.request.urlopen(req, timeout=120).read()
        except urllib.error.HTTPError as e:
            if e.code in (429, 500, 502, 503, 504) and i < tries - 1:
                wait = int(e.headers.get("Retry-After", 0) or 0) or 15 * (i + 1)
                print(f"  HTTP {e.code}, waiting {wait}s", file=sys.stderr); time.sleep(wait); continue
            sys.exit(f"HTTP {e.code} for {url.split('?')[0]}: {e.read()[:200]!r}")


def main():
    ap = argparse.ArgumentParser(); ap.add_argument("--source"); ap.add_argument("--force", action="store_true")
    a = ap.parse_args(); tok = token()
    todo = [e for e in json.load(open(os.path.join(ROOT, "manifest.json")))["screens"]
            if e["kind"] != "superseded" and (not a.source or e["source"] == a.source)
            and (a.force or not os.path.exists(os.path.join(ROOT, e["file"])))]
    print(len(todo), "to download")
    by = defaultdict(list)
    for e in todo: by[e["fileKey"]].append(e)
    ok = bad = 0
    for fk, es in by.items():
        for i in range(0, len(es), BATCH):
            ch = es[i:i + BATCH]
            # strips can be 12000px wide; REST caps renders, so use a scale that keeps the long edge <= 3200
            scale = {e["id"]: min(1.0, 3200 / max(e["w"], e["h"])) for e in ch}
            for s in sorted(set(round(v, 3) for v in scale.values())):
                grp = [e for e in ch if round(scale[e["id"]], 3) == s]
                q = urllib.parse.urlencode({"ids": ",".join(e["nodeId"] for e in grp), "format": "png", "scale": s})
                res = json.loads(get(f"https://api.figma.com/v1/images/{fk}?{q}", tok))
                if res.get("err"): sys.exit(f"figma error: {res['err']}")
                for e in grp:
                    url = (res.get("images") or {}).get(e["nodeId"])
                    if not url: print("  no image for", e["id"]); bad += 1; continue
                    path = os.path.join(ROOT, e["file"]); os.makedirs(os.path.dirname(path), exist_ok=True)
                    open(path, "wb").write(get(url))
                    try: Image.open(path).verify(); ok += 1
                    except Exception: print("  corrupt", e["id"]); os.remove(path); bad += 1
            print(f"  {ok} ok, {bad} failed")
    print("done:", ok, "downloaded,", bad, "failed")


if __name__ == "__main__":
    main()
