#!/usr/bin/env python3
"""Inventory of what `main` brings into `2.x`: one row per upstream PR, every touched file
classified against the 2.x tree, and a first-guess verdict to confirm by reading the PR.

Usage (from the repo root, after `git fetch origin main 2.x`):
    python3 .claude/skills/sync-main-into-2x/scripts/inventory.py [--base origin/2.x] [--main origin/main] [--gh]

--gh adds each PR's title and author from GitHub (needs `gh` auth).
"""
import argparse
import json
import re
import subprocess
import sys
from collections import Counter


def git(*args):
    return subprocess.run(["git", *args], check=True, capture_output=True, text=True).stdout


def exists(ref, path):
    return subprocess.run(
        ["git", "cat-file", "-e", f"{ref}:{path}"], capture_output=True
    ).returncode == 0


def on_2x(ref, path, crate_files):
    """Where the file lives on 2.x, or None. lib/src moved to the engine crate; what moved on
    into dcl_core (or was split) is offered by file name, as `?candidate`."""
    if path.startswith("lib/src/"):
        moved = "lib/crates/dcl_engine/src/" + path[len("lib/src/"):]
        if exists(ref, moved):
            return moved
        name = "/" + path.rsplit("/", 1)[-1]
        if name in ("/mod.rs", "/lib.rs"):
            return None
        hits = [f for f in crate_files if f.endswith(name)]
        return " ".join("?" + h for h in hits) or None
    return path if exists(ref, path) else None


def classify(path):
    if path.startswith(".github/"):
        return "ci"
    if path.startswith("godot/locale/") or path.startswith("rn/src/i18n/"):
        return "i18n"
    if path.endswith((".gd", ".gd.uid")) and path.startswith("godot/src/"):
        return "ui-gd" if path.startswith("godot/src/ui/") else "gd"
    if path.startswith("godot/src/ui/"):
        return "ui-scene"
    if path.startswith("godot/src/"):
        return "scene"
    if path.startswith("godot/"):
        return "asset"
    if path.startswith("lib/"):
        return "rust"
    if path.startswith("src/"):
        return "xtask"
    if path.startswith("plugins/"):
        return "plugin"
    if path.startswith("rn/"):
        return "rn"
    if path.startswith("tests/"):
        return "tests"
    return "other"


def guess(classes, subject):
    kinds = set(classes)
    if re.match(r"chore(\(release\))?: bump|chore\(release\)", subject):
        return "Taken (release chore) or Discarded"
    if kinds <= {"ci", "other", "tests"}:
        return "Taken"
    if kinds <= {"ui-gd", "ui-scene", "i18n"}:
        return "Discarded (GDScript UI) unless it is a feature -> RN"
    if kinds <= {"rust", "xtask", "asset", "i18n", "ci", "tests"}:
        return "Taken (merge) - check it is wired on 2.x"
    return "Ported/Adapted - read it"


def units(base, main):
    """(label, from, to, subject) per upstream change. A release-sync merge on main's first
    parent is expanded into the commits it brings that 2.x does not have yet."""
    out = []
    for line in git("log", "--reverse", "--first-parent", "--format=%H %P\t%s", f"{base}..{main}").splitlines():
        head, subject = line.split("\t", 1)
        sha, *parents = head.split()
        if len(parents) > 1 and re.search(r"release|sync", subject, re.I):
            inner = git("log", "--reverse", "--no-merges", "--format=%H\t%s",
                        f"{parents[0]}..{sha}", f"^{base}").splitlines()
            for row in inner:
                isha, isubject = row.split("\t", 1)
                out.append((isha, f"{isha}^", isha, isubject + f"  [via {sha[:10]}]"))
            continue
        out.append((sha, parents[0], sha, subject))
    return out


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--base", default="origin/2.x")
    ap.add_argument("--main", default="origin/main")
    ap.add_argument("--gh", action="store_true")
    args = ap.parse_args()

    crate_files = git("ls-tree", "-r", "--name-only", args.base, "lib/crates").splitlines()
    merge_base = git("merge-base", args.base, args.main).strip()
    print(f"merge-base {merge_base[:10]}  ({args.base} vs {args.main})\n")
    for sha, frm, to, subject in units(merge_base, args.main):
        files = [l.split("\t") for l in git("diff", "--name-status", "--no-renames", frm, to).splitlines()]
        if not files:
            continue
        pr = re.search(r"#(\d+)", subject)
        title = ""
        if args.gh and pr:
            info = json.loads(subprocess.run(
                ["gh", "pr", "view", pr.group(1), "--json", "title,author"],
                capture_output=True, text=True).stdout or "{}")
            title = f" — {info.get('title', '')} (@{info.get('author', {}).get('login', '?')})"
        classes = [classify(f[-1]) for f in files]
        counts = ", ".join(f"{k} {v}" for k, v in Counter(classes).most_common())
        print(f"## {sha[:10]} {subject}{title}")
        print(f"   {len(files)} files: {counts}")
        print(f"   first guess: {guess(classes, subject)}")
        for (status, *paths), kind in zip(files, classes):
            path = paths[-1]
            where = on_2x(args.base, path, crate_files)
            mark = "same path" if where == path else (f"-> {where}" if where else "NOT on 2.x")
            print(f"     {status} {kind:8} {path}  [{mark}]")
        print()


if __name__ == "__main__":
    sys.exit(main())
