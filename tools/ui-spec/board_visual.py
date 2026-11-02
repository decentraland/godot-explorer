#!/usr/bin/env python3
"""The mobile board's visual-snapshot service from CI (screenshots and reports live in its R2 bucket).

  board_visual.py fetch-official --ref REF --out DIR           download the accepted screenshots of a branch
  board_visual.py open-run --kind pr|official --sha S --ref R [--base-ref B] [--pr N]   prints the run id
  board_visual.py upload-report --run ID --report visual-report
  board_visual.py complete-run --run ID --report visual-report
  board_visual.py publish-official --ref REF --sha S --dir shots   replace a branch's accepted screenshots

Env: BUILD_REPORTER_TOKEN (secret) and BOARD_EVENTS_URL (repository variable), as report-build-event.py.
Without them (fork PRs, local runs) every command does nothing and says so; `fetch-official` exits 3 when the board
has no screenshots for the ref yet.
"""
import argparse, json, os, sys, time, urllib.error, urllib.parse, urllib.request
from concurrent.futures import ThreadPoolExecutor

sys.path.insert(0, os.path.dirname(__file__))
import visual_report

TOKEN = os.environ.get("BUILD_REPORTER_TOKEN", "")
BASE = os.environ.get("BOARD_EVENTS_URL", "").rstrip("/")
WORKERS = 8
MAX_FILE = 8 * 1024 * 1024


def request(method, path, data=None, content_type="application/json", retries=3):
    headers = {"Authorization": f"Bearer {TOKEN}", "User-Agent": "godot-explorer-ci", "Content-Type": content_type}
    for attempt in range(1, retries + 1):
        req = urllib.request.Request(f"{BASE}{path}", data=data, method=method, headers=headers)
        try:
            with urllib.request.urlopen(req, timeout=60) as r:
                return r.status, r.read()
        except urllib.error.HTTPError as e:
            body = e.read()
            if e.code < 500 or attempt == retries:
                return e.code, body
        except urllib.error.URLError:
            if attempt == retries:
                raise
        time.sleep(2 * attempt)


def configured():
    if TOKEN and BASE:
        return True
    print("board_visual: BUILD_REPORTER_TOKEN / BOARD_EVENTS_URL not set, skipping (fork PR or local run)", file=sys.stderr)
    return False


def ref_key(ref):
    return ref.replace("/", "~")


def put_files(prefix, root, names):
    def put(name):
        path = os.path.join(root, name)
        if os.path.getsize(path) > MAX_FILE:
            print(f"::warning title=board_visual::{name} is over 8 MB, not uploaded")
            return True
        with open(path, "rb") as f:
            code, body = request("PUT", f"{prefix}/{urllib.parse.quote(name)}", f.read(), "application/octet-stream")
        if code != 201:
            print(f"::error title=board_visual::upload of {name} failed: {code} {body[:200]!r}")
        return code == 201

    with ThreadPoolExecutor(WORKERS) as pool:
        ok = list(pool.map(put, names))
    return all(ok)


def listing(root, suffixes):
    return sorted(
        os.path.relpath(os.path.join(base, n), root)
        for base, _, names in os.walk(root)
        for n in names
        if n.endswith(suffixes)
    )


def fetch_official(ref, out):
    if not configured():
        return 3
    code, body = request("GET", f"/api/visual/official/{urllib.parse.quote(ref_key(ref))}/manifest")
    if code == 404:
        print(f"board_visual: no official screenshots for {ref} yet")
        return 3
    if code != 200:
        print(f"::warning title=board_visual::manifest of {ref}: {code}")
        return 3
    ids = json.loads(body)["ids"]

    def get(ident):
        c, data = request("GET", f"/api/visual/official/{urllib.parse.quote(ref_key(ref))}/files/{urllib.parse.quote(ident)}.png")
        if c != 200:
            return False
        dest = os.path.join(out, ident + ".png")
        os.makedirs(os.path.dirname(dest), exist_ok=True)
        with open(dest, "wb") as f:
            f.write(data)
        return True

    with ThreadPoolExecutor(WORKERS) as pool:
        ok = list(pool.map(get, ids))
    print(f"board_visual: {sum(ok)}/{len(ids)} official screenshots of {ref} downloaded")
    return 0 if all(ok) else 1


def open_run(args):
    if not configured():
        return 0
    event = {
        "run_id": os.environ["GITHUB_RUN_ID"],
        "run_attempt": int(os.environ.get("GITHUB_RUN_ATTEMPT", "1")),
        "sha": args.sha,
        "ref": args.ref,
        "base_ref": args.base_ref or None,
        "pr_number": int(args.pr) if args.pr else None,
        "kind": args.kind,
        "run_url": f"{os.environ.get('GITHUB_SERVER_URL', 'https://github.com')}/{os.environ['GITHUB_REPOSITORY']}/actions/runs/{os.environ['GITHUB_RUN_ID']}",
    }
    code, body = request("POST", "/api/visual/runs", json.dumps(event).encode())
    if code != 201:
        print(f"::error title=board_visual::could not open the run: {code} {body[:200]!r}")
        return 1
    print(json.loads(body)["id"])
    return 0


def upload_report(run, report):
    if not configured():
        return 0
    names = [n for n in listing(report, (".png", ".html", ".json"))]
    return 0 if put_files(f"/api/visual/runs/{run}/files", report, names) else 1


def complete_run(run, report):
    if not configured():
        return 0
    summary = visual_report.build_summary(report)
    summary.pop("captured")
    code, body = request("POST", f"/api/visual/runs/{run}/complete", json.dumps({"summary": summary}).encode())
    if code != 200:
        print(f"::error title=board_visual::could not close the run: {code} {body[:200]!r}")
        return 1
    print("board_visual: run closed")
    return 0


def publish_official(ref, sha, shots):
    if not configured():
        return 0
    names = listing(shots, (".png",))
    prefix = f"/api/visual/official/{urllib.parse.quote(ref_key(ref))}"
    if not put_files(f"{prefix}/files", shots, names):
        return 1
    ids = [n[: -len(".png")] for n in names]
    code, body = request("POST", f"{prefix}/complete", json.dumps({"ref": ref, "sha": sha, "ids": ids}).encode())
    if code != 200:
        print(f"::error title=board_visual::could not publish: {code} {body[:200]!r}")
        return 1
    print(f"board_visual: {len(ids)} official screenshots of {ref} published")
    return 0


def main():
    parser = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawTextHelpFormatter)
    sub = parser.add_subparsers(dest="cmd", required=True)
    f = sub.add_parser("fetch-official")
    f.add_argument("--ref", required=True)
    f.add_argument("--out", required=True)
    o = sub.add_parser("open-run")
    o.add_argument("--kind", choices=["pr", "official"], required=True)
    o.add_argument("--sha", required=True)
    o.add_argument("--ref", required=True)
    o.add_argument("--base-ref", default="")
    o.add_argument("--pr", default="")
    u = sub.add_parser("upload-report")
    u.add_argument("--run", required=True)
    u.add_argument("--report", default="visual-report")
    c = sub.add_parser("complete-run")
    c.add_argument("--run", required=True)
    c.add_argument("--report", default="visual-report")
    p = sub.add_parser("publish-official")
    p.add_argument("--ref", required=True)
    p.add_argument("--sha", required=True)
    p.add_argument("--dir", required=True)
    args = parser.parse_args()
    code = {
        "fetch-official": lambda: fetch_official(args.ref, args.out),
        "open-run": lambda: open_run(args),
        "upload-report": lambda: upload_report(args.run, args.report),
        "complete-run": lambda: complete_run(args.run, args.report),
        "publish-official": lambda: publish_official(args.ref, args.sha, args.dir),
    }[args.cmd]()
    sys.exit(code)


if __name__ == "__main__":
    main()
