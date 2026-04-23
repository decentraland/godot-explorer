#!/usr/bin/env python3
"""
Find unused (orphan) and broken .tscn/.tres files under godot/.

Algorithm
  1. Collect every .tscn/.tres under godot/, parse its uid (if any) and its
     ext_resource references (by path and by uid).
  2. Scan non-resource text files for "res://..." and "uid://..." strings.
     Sources:
       - godot/project.godot, godot/export_presets.cfg, godot/dclgodot.gdextension
       - godot/**/*.gd, godot/**/*.gdshader, godot/**/*.gdshaderinc
       - godot/addons/**/plugin.cfg
       - lib/src/**/*.rs  (the Rust side loads scenes by path via GDExtension)
  3. A resource is "externally referenced" if any of those files mention it
     by res:// path or by uid://.
  4. Reachable = transitive closure of externally-referenced resources over
     ext_resource edges in .tscn/.tres.
  5. Orphan = a tracked .tscn/.tres that is NOT reachable.
  6. Broken = a tracked .tscn/.tres with an ext_resource whose `path=` does
     not exist on disk.

Limitations (false-negative risks — i.e. things the script may call orphan
that are actually used):
  - Dynamic loads like `load("res://foo/" + name + ".tscn")` are invisible.
  - A .gd file that is itself unused but preloads a .tscn will still cause
    that .tscn to look reachable. Fixing that requires also computing .gd
    reachability, which is out of scope for this pass.

By default prints a report and exits 0. Pass --delete to remove orphan files
(and their .uid sidecars). Broken files are never auto-deleted; fix or prune
them by hand after reviewing.
"""

from __future__ import annotations

import argparse
import json
import re
import sys
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parent.parent
GODOT_DIR = REPO_ROOT / "godot"
LIB_DIR = REPO_ROOT / "lib"

RESOURCE_SUFFIXES = {".tscn", ".tres"}

# Editor/build artifacts and vendored binaries we never want to walk into.
EXCLUDED_DIRS = {".godot", ".import", "lib", "target", "node_modules"}

# Files Godot treats as reachable by convention even if no code references them.
CONVENTION_ROOTS = {
    "res://default_bus_layout.tres",  # default audio bus layout
    "res://default_env.tres",          # default world environment (if present)
}

UID_ATTR_RE = re.compile(r'\buid="(uid://[a-z0-9]+)"')
PATH_ATTR_RE = re.compile(r'\bpath="([^"]+)"')
EXT_RESOURCE_RE = re.compile(r"\[ext_resource\b([^\]]*)\]")

# Scanning non-resource text files: capture any res:// path or uid:// id,
# quoted or not. We allow anything except whitespace and a handful of
# terminators so we still pick up entries like  *res://foo.gd  in .godot files.
RES_PATH_RE = re.compile(r'(res://[^\s"\']+)')
UID_REF_RE = re.compile(r"(uid://[a-z0-9]+)")


def iter_files(root: Path, suffixes=None, names=None):
    """Walk root, skipping EXCLUDED_DIRS. Yield files matching suffix or name."""
    for p in root.rglob("*"):
        if not p.is_file():
            continue
        if any(part in EXCLUDED_DIRS for part in p.relative_to(root).parts[:-1]):
            continue
        if suffixes and p.suffix in suffixes:
            yield p
        elif names and p.name in names:
            yield p


def disk_to_res(p: Path) -> str:
    return "res://" + p.relative_to(GODOT_DIR).as_posix()


def res_to_disk(res_path: str) -> Path | None:
    if not res_path.startswith("res://"):
        return None
    return GODOT_DIR / res_path[len("res://"):]


def parse_resource_file(path: Path):
    """Return (own_uid, ref_paths, ref_uids, ext_resource_paths) for a .tscn/.tres.

    - `ref_paths` / `ref_uids` capture every res://... and uid://... occurrence in
      the file body (needed because instance_placeholder=, script paths, and some
      other properties reference scenes outside of [ext_resource] blocks).
    - `ext_resource_paths` is the narrower set of paths declared in
      [ext_resource ... path="res://..."]; used for broken-file detection.
    """
    try:
        text = path.read_text(encoding="utf-8", errors="replace")
    except OSError:
        return None, set(), set(), []

    header_end = text.find("\n")
    header = text[:header_end] if header_end != -1 else text
    own_uid_match = UID_ATTR_RE.search(header)
    own_uid = own_uid_match.group(1) if own_uid_match else None

    ref_paths = set(RES_PATH_RE.findall(text))
    ref_uids = set(UID_REF_RE.findall(text))
    # Don't count a file referencing itself by its own uid.
    if own_uid:
        ref_uids.discard(own_uid)

    ext_resource_paths = []
    for m in EXT_RESOURCE_RE.finditer(text):
        attrs = m.group(1)
        path_match = PATH_ATTR_RE.search(attrs)
        if path_match:
            ext_resource_paths.append(path_match.group(1))
    return own_uid, ref_paths, ref_uids, ext_resource_paths


def collect_resources():
    resources = {}
    for p in iter_files(GODOT_DIR, suffixes=RESOURCE_SUFFIXES):
        own_uid, ref_paths, ref_uids, ext_resource_paths = parse_resource_file(p)
        resources[disk_to_res(p)] = {
            "uid": own_uid,
            "ref_paths": ref_paths,
            "ref_uids": ref_uids,
            "ext_resource_paths": ext_resource_paths,
            "disk": p,
        }
    return resources


def scan_external_refs():
    """Collect every res://... and uid://... string found in non-resource files."""
    raw_paths: set[str] = set()
    raw_uids: set[str] = set()

    text_files: list[Path] = []
    for name in ("project.godot", "export_presets.cfg", "dclgodot.gdextension"):
        f = GODOT_DIR / name
        if f.exists():
            text_files.append(f)

    text_files.extend(iter_files(
        GODOT_DIR,
        suffixes={".gd", ".gdshader", ".gdshaderinc"},
    ))
    text_files.extend(iter_files(GODOT_DIR, names={"plugin.cfg"}))

    if LIB_DIR.exists():
        text_files.extend(iter_files(LIB_DIR, suffixes={".rs"}))

    for f in text_files:
        try:
            text = f.read_text(encoding="utf-8", errors="replace")
        except OSError:
            continue
        raw_paths.update(RES_PATH_RE.findall(text))
        raw_uids.update(UID_REF_RE.findall(text))

    return raw_paths, raw_uids


def resolve_external_refs(resources, raw_paths, raw_uids):
    """Return (set of res_paths referenced externally, list of unresolved uids)."""
    uid_index = {info["uid"]: rp for rp, info in resources.items() if info["uid"]}
    resolved: set[str] = set()
    for p in raw_paths:
        if p in resources:
            resolved.add(p)
    unresolved_uids = []
    for u in raw_uids:
        rp = uid_index.get(u)
        if rp:
            resolved.add(rp)
        else:
            unresolved_uids.append(u)
    return resolved, unresolved_uids, uid_index


def compute_reachable(resources, uid_index, roots):
    reachable = set(roots)
    stack = list(roots)
    while stack:
        rp = stack.pop()
        info = resources.get(rp)
        if not info:
            continue
        for ref_path in info["ref_paths"]:
            if ref_path in resources and ref_path not in reachable:
                reachable.add(ref_path)
                stack.append(ref_path)
        for ref_uid in info["ref_uids"]:
            target = uid_index.get(ref_uid)
            if target and target not in reachable:
                reachable.add(target)
                stack.append(target)
    return reachable


def find_broken(resources):
    """Resource is broken if any ext_resource `path=` does not exist on disk."""
    broken = {}
    for rp, info in resources.items():
        missing = []
        for ref_path in info["ext_resource_paths"]:
            if not ref_path.startswith("res://"):
                continue
            disk = res_to_disk(ref_path)
            if disk and not disk.exists():
                missing.append(ref_path)
        if missing:
            broken[rp] = missing
    return broken


def group_by_top_dir(paths):
    groups = {}
    for rp in sorted(paths):
        parts = rp[len("res://"):].split("/", 1)
        key = parts[0] if len(parts) > 1 else "<root>"
        groups.setdefault(key, []).append(rp)
    return groups


def delete_orphans(resources, orphans) -> int:
    removed = 0
    for rp in sorted(orphans):
        info = resources.get(rp)
        if not info:
            continue
        disk = info["disk"]
        sidecar = disk.with_suffix(disk.suffix + ".uid")
        try:
            disk.unlink()
            removed += 1
            print(f"  removed: {rp}")
        except OSError as e:
            print(f"  FAILED to remove {rp}: {e}", file=sys.stderr)
            continue
        if sidecar.exists():
            try:
                sidecar.unlink()
                print(f"  removed: {disk_to_res(sidecar)}")
            except OSError as e:
                print(f"  FAILED to remove sidecar {sidecar}: {e}", file=sys.stderr)
    return removed


def main():
    parser = argparse.ArgumentParser(
        description=__doc__,
        formatter_class=argparse.RawDescriptionHelpFormatter,
    )
    parser.add_argument("--json", action="store_true", help="Emit JSON instead of a human report")
    parser.add_argument("--delete", action="store_true", help="Delete orphan files (and their .uid sidecars)")
    parser.add_argument("--check", action="store_true", help="Exit non-zero if any orphan or broken file is found (for CI)")
    args = parser.parse_args()

    resources = collect_resources()
    raw_paths, raw_uids = scan_external_refs()
    roots, unresolved_uids, uid_index = resolve_external_refs(resources, raw_paths, raw_uids)
    for rp in CONVENTION_ROOTS:
        if rp in resources:
            roots.add(rp)
    reachable = compute_reachable(resources, uid_index, roots)
    orphans = set(resources) - reachable
    broken = find_broken(resources)

    if args.json:
        print(json.dumps({
            "total": len(resources),
            "reachable": len(reachable),
            "orphans": sorted(orphans),
            "broken": broken,
            "unresolved_uids_in_source": sorted(set(unresolved_uids)),
        }, indent=2))
        return 0

    print(f"godot root: {GODOT_DIR}")
    print(f"total .tscn/.tres: {len(resources)}")
    print(f"reachable:         {len(reachable)}")
    print(f"orphan:            {len(orphans)}")
    print(f"broken:            {len(broken)}")

    if orphans:
        print()
        print("=" * 72)
        print("ORPHAN FILES (no inbound reference)")
        print("=" * 72)
        for group, files in sorted(group_by_top_dir(orphans).items()):
            print(f"\n[{group}/]  ({len(files)})")
            for rp in files:
                print(f"  {rp}")

    if broken:
        print()
        print("=" * 72)
        print("BROKEN FILES (ext_resource path missing on disk)")
        print("=" * 72)
        for rp in sorted(broken):
            print(f"\n  {rp}")
            for missing in broken[rp]:
                print(f"    missing: {missing}")

    if args.delete:
        if not orphans:
            print("\nnothing to delete.")
            return 0
        print()
        print("=" * 72)
        print(f"DELETING {len(orphans)} orphan files")
        print("=" * 72)
        removed = delete_orphans(resources, orphans)
        print(f"\nremoved {removed} file(s). broken files were NOT auto-deleted.")

    if args.check and (orphans or broken):
        print(
            f"\n::error::{len(orphans)} orphan and {len(broken)} broken resource(s) "
            "found. Run `python3 scripts/find_unused_resources.py` locally and clean up, "
            "or delete with `--delete`.",
            file=sys.stderr,
        )
        return 1

    return 0


if __name__ == "__main__":
    sys.exit(main())
