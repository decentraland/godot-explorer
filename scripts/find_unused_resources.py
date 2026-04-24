#!/usr/bin/env python3
"""
Find unused (orphan) and broken assets under godot/.

Tracks every referenceable asset type — not just scenes — and resolves refs
across GDScript, shaders, scenes, resources, plugin configs, project.godot,
export presets, and Rust code.

Tracked asset types:
  - scenes/resources : .tscn .tres
  - scripts          : .gd                         (uid from adjacent .gd.uid)
  - shaders          : .gdshader .gdshaderinc      (uid from adjacent .uid)
  - images           : .png .svg .jpg .jpeg .webp .bmp .tga  (uid from .import)
  - audio            : .wav .ogg .mp3                         (uid from .import)
  - fonts            : .ttf .otf                              (uid from .import)
  - 3D               : .glb .gltf                             (uid from .import)
  - data             : .json .res .stylebox

Reference discovery:
  - res://... strings in any text source
  - uid://... strings in any text source
  - [ext_resource path=..., uid=...] in .tscn/.tres (also used for broken detection)
  - instance_placeholder=, property values: widened res://uid:// scan in .tscn/.tres
  - #include "relative.gdshaderinc" in shaders
  - class_name X declared in a .gd → reached if any other .gd/.tscn/.tres
    uses the bare token X (Godot global class resolution)
  - Convention roots: res://default_bus_layout.tres, res://default_env.tres,
    and everything under res://tests/ (opened directly by humans in the editor)

Source files scanned for references:
  - godot/project.godot, godot/export_presets.cfg, godot/dclgodot.gdextension
  - godot/**/*.gd, godot/**/*.gdshader, godot/**/*.gdshaderinc
  - godot/**/plugin.cfg
  - godot/**/*.tscn, godot/**/*.tres
  - lib/src/**/*.rs

Limitations:
  - Dynamic loads like load("res://foo/" + name + ".tscn") are invisible.
  - A .gd declaring a short/common class_name may be kept alive by an unrelated
    substring match. Token-word matching reduces this; collisions are flagged
    as reachable (conservative).

Modes:
  default        : report only, exit 0
  --check        : exit 1 if a gated orphan or any broken file is found (CI mode)
  --check-types  : which types make --check fail. Default 'scene,resource,gd,
                   shader' — the types the static analyzer resolves confidently.
                   Pass 'all' to gate on every type (image/audio/font/model/
                   data too — likely noisy because those can be loaded by
                   dynamic path construction).
  --delete       : delete orphan .tscn/.tres (and their .uid sidecars)
  --delete-types : comma-separated list of asset types to also delete (e.g.
                   "gd,shader,image"). Requires --delete. Use with care.
  --json         : machine-readable output
"""

from __future__ import annotations

import argparse
import json
import re
import sys
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parent.parent
GODOT_DIR = REPO_ROOT / "godot"

# Editor caches / build artifacts / vendored binaries never walked into.
# Note: "lib" is excluded when walking *inside godot/* (where lib/ holds the
# built dcl GDExtension binary). Rust source scanning uses its own exclusions.
EXCLUDED_DIRS = {".godot", ".import", "lib", "target", "node_modules"}
RUST_EXCLUDED_DIRS = {"target", "node_modules", ".git"}

# Asset classes: suffix → type label used in the report / CLI.
ASSET_CLASSES = {
    ".tscn": "scene",
    ".tres": "resource",
    ".gd": "gd",
    ".gdshader": "shader",
    ".gdshaderinc": "shader",
    ".png": "image",
    ".svg": "image",
    ".jpg": "image",
    ".jpeg": "image",
    ".webp": "image",
    ".bmp": "image",
    ".tga": "image",
    ".wav": "audio",
    ".ogg": "audio",
    ".mp3": "audio",
    ".ttf": "font",
    ".otf": "font",
    ".glb": "model",
    ".gltf": "model",
    ".json": "data",
    ".res": "data",
    ".stylebox": "data",
}

# Files Godot treats as reachable even if nothing names them directly.
CONVENTION_ROOTS = {
    "res://default_bus_layout.tres",
    "res://default_env.tres",
    "res://google-services.json",  # Firebase config, loaded natively by Android
}

# res:// path prefixes whose contents are entry points: every asset under
# them is treated as a reachability root. Useful for test/demo scenes the
# team opens in the editor to run in isolation — they legitimately have no
# inbound code reference.
CONVENTION_ROOT_PREFIXES = (
    "res://tests/",
)

# res:// path prefixes whose contents are third-party payloads — addon
# binaries, vendored framework metadata, etc. Skip from orphan tracking.
EXCLUDED_RES_PREFIXES = (
    "res://addons/sentry/bin/",
    "res://addons/dcl-godot-android/bin/",
)

UID_ATTR_RE = re.compile(r'\buid="(uid://[a-z0-9]+)"')
PATH_ATTR_RE = re.compile(r'\bpath="([^"]+)"')
EXT_RESOURCE_RE = re.compile(r"\[ext_resource\b([^\]]*)\]")
RES_PATH_RE = re.compile(r'(res://[^\s"\']+)')
UID_REF_RE = re.compile(r"(uid://[a-z0-9]+)")
CLASS_NAME_DECL_RE = re.compile(
    r"^\s*(?:@\w+(?:\([^)]*\))?\s+)*class_name\s+([A-Z][A-Za-z0-9_]*)",
    re.MULTILINE,
)
PRELOAD_LOAD_RE = re.compile(r'\b(?:preload|load)\s*\(\s*"([^"]+)"\s*\)')
SHADER_INCLUDE_RE = re.compile(r'^\s*#include\s+"([^"]+)"', re.MULTILINE)
# Godot built-in global class names and engine types that sometimes appear as
# tokens in scripts. Keep the list small; we only use it to avoid tagging user
# scripts as "used" via engine-class names. Not strictly needed but tidier.
BUILTIN_CLASSES = set()


def iter_files(root: Path, suffixes=None, names=None):
    for p in root.rglob("*"):
        if not p.is_file():
            continue
        if any(part in EXCLUDED_DIRS for part in p.relative_to(root).parts[:-1]):
            continue
        if suffixes and p.suffix.lower() in suffixes:
            yield p
        elif names and p.name in names:
            yield p


def disk_to_res(p: Path) -> str:
    return "res://" + p.relative_to(GODOT_DIR).as_posix()


def res_to_disk(res_path: str) -> Path | None:
    if not res_path.startswith("res://"):
        return None
    return GODOT_DIR / res_path[len("res://"):]


def read_text(p: Path) -> str:
    try:
        return p.read_text(encoding="utf-8", errors="replace")
    except OSError:
        return ""


def asset_uid(p: Path) -> str | None:
    """Extract the uid for an asset by inspecting its file or sidecar."""
    suffix = p.suffix.lower()
    if suffix in (".tscn", ".tres"):
        text = read_text(p)
        first_nl = text.find("\n")
        header = text[:first_nl] if first_nl != -1 else text
        m = UID_ATTR_RE.search(header)
        return m.group(1) if m else None
    if suffix in (".gd", ".gdshader", ".gdshaderinc"):
        sidecar = p.with_suffix(p.suffix + ".uid")
        if sidecar.exists():
            line = read_text(sidecar).strip().splitlines()[0] if read_text(sidecar).strip() else ""
            if line.startswith("uid://"):
                return line
        return None
    # Imported binary assets (images/audio/fonts/models) keep uid in .import.
    import_sidecar = p.with_suffix(p.suffix + ".import")
    if import_sidecar.exists():
        m = UID_ATTR_RE.search(read_text(import_sidecar))
        return m.group(1) if m else None
    return None


def parse_tscn_tres(path: Path):
    """Return (own_uid, ref_paths, ref_uids, ext_resource_paths)."""
    text = read_text(path)
    first_nl = text.find("\n")
    header = text[:first_nl] if first_nl != -1 else text
    own_uid_match = UID_ATTR_RE.search(header)
    own_uid = own_uid_match.group(1) if own_uid_match else None

    ref_paths = set(RES_PATH_RE.findall(text))
    ref_uids = set(UID_REF_RE.findall(text))
    if own_uid:
        ref_uids.discard(own_uid)

    ext_resource_paths = []
    for m in EXT_RESOURCE_RE.finditer(text):
        pm = PATH_ATTR_RE.search(m.group(1))
        if pm:
            ext_resource_paths.append(pm.group(1))
    return own_uid, ref_paths, ref_uids, ext_resource_paths


def parse_gd(path: Path):
    """Return (class_name or None, res paths, uid refs, class tokens used)."""
    text = read_text(path)
    decl = CLASS_NAME_DECL_RE.search(text)
    class_name = decl.group(1) if decl else None
    ref_paths = set(RES_PATH_RE.findall(text))
    ref_uids = set(UID_REF_RE.findall(text))
    # preload("./foo.gd") / load("../bar.tscn"): resolve non-res:// arguments
    # relative to the .gd's directory.
    for arg in PRELOAD_LOAD_RE.findall(text):
        if arg.startswith("res://") or arg.startswith("uid://"):
            continue
        try:
            target = (path.parent / arg).resolve()
            ref_paths.add(disk_to_res(target))
        except (ValueError, OSError):
            pass
    # All CamelCase tokens the file mentions; filtered later against declared class names.
    tokens = set(re.findall(r"\b([A-Z][A-Za-z0-9_]*)\b", text))
    if class_name:
        tokens.discard(class_name)
    return class_name, ref_paths, ref_uids, tokens


def parse_shader(path: Path):
    """Return (res paths, uid refs, set of absolute res:// include targets)."""
    text = read_text(path)
    ref_paths = set(RES_PATH_RE.findall(text))
    ref_uids = set(UID_REF_RE.findall(text))
    includes = set()
    for rel in SHADER_INCLUDE_RE.findall(text):
        if rel.startswith("res://"):
            includes.add(rel)
            continue
        # relative path — resolve against the shader's directory
        target = (path.parent / rel).resolve()
        try:
            includes.add(disk_to_res(target))
        except ValueError:
            pass
    return ref_paths, ref_uids, includes


def collect_assets():
    """Map res_path → {'uid', 'suffix', 'type', 'disk'}."""
    assets = {}
    for p in iter_files(GODOT_DIR, suffixes=set(ASSET_CLASSES)):
        res_path = disk_to_res(p)
        if any(res_path.startswith(prefix) for prefix in EXCLUDED_RES_PREFIXES):
            continue
        assets[res_path] = {
            "uid": asset_uid(p),
            "suffix": p.suffix.lower(),
            "type": ASSET_CLASSES[p.suffix.lower()],
            "disk": p,
        }
    return assets


def collect_references(assets):
    """Walk every text source; return reachability inputs."""
    uid_index = {info["uid"]: rp for rp, info in assets.items() if info["uid"]}

    # Direct path/uid reference strings scooped up from every text source.
    raw_res_paths: set[str] = set()
    raw_uids: set[str] = set()

    # Class-name universe: declared in .gd, used as bare tokens in .gd/.tscn/.tres.
    class_decl: dict[str, str] = {}  # class_name → res_path of the declaring .gd
    class_tokens_used: set[str] = set()

    # Per-asset outgoing edges for the transitive walk.
    edges: dict[str, tuple[set[str], set[str]]] = {}

    # Track ext_resource paths for broken-file detection.
    ext_resource_paths: dict[str, list[str]] = {}

    # Config roots — also treat their res:// / uid:// refs as entry points,
    # and harvest CamelCase tokens (e.g. run/main_loop_type="ProjectMainLoop").
    config_files = [
        GODOT_DIR / "project.godot",
        GODOT_DIR / "export_presets.cfg",
        GODOT_DIR / "dclgodot.gdextension",
    ]
    for cf in config_files:
        if cf.exists():
            text = read_text(cf)
            raw_res_paths.update(RES_PATH_RE.findall(text))
            raw_uids.update(UID_REF_RE.findall(text))
            class_tokens_used.update(re.findall(r"\b([A-Z][A-Za-z0-9_]*)\b", text))

    # Plugin configs: the script= entry uses a path relative to the plugin.cfg's
    # directory, not a res:// path. Resolve those explicitly.
    for p in iter_files(GODOT_DIR, names={"plugin.cfg"}):
        text = read_text(p)
        raw_res_paths.update(RES_PATH_RE.findall(text))
        raw_uids.update(UID_REF_RE.findall(text))
        m = re.search(r'^\s*script\s*=\s*"([^"]+)"', text, re.MULTILINE)
        if m:
            rel = m.group(1)
            if rel.startswith("res://"):
                raw_res_paths.add(rel)
            else:
                target = (p.parent / rel).resolve()
                try:
                    raw_res_paths.add(disk_to_res(target))
                except ValueError:
                    pass

    # Scene/resource files: widened scan + ext_resource for broken-detection.
    for rp, info in assets.items():
        if info["suffix"] in (".tscn", ".tres"):
            own_uid, refs_p, refs_u, ext_paths = parse_tscn_tres(info["disk"])
            edges[rp] = (refs_p, refs_u)
            if ext_paths:
                ext_resource_paths[rp] = ext_paths
            raw_res_paths.update(refs_p)
            raw_uids.update(refs_u)

    # GDScript: paths, uids, class_name declarations, and CamelCase token usage.
    for rp, info in assets.items():
        if info["suffix"] == ".gd":
            decl, refs_p, refs_u, tokens = parse_gd(info["disk"])
            edges[rp] = (refs_p, refs_u)
            raw_res_paths.update(refs_p)
            raw_uids.update(refs_u)
            if decl:
                class_decl[decl] = rp
            class_tokens_used.update(tokens)

    # Shaders: paths, uids, and #include edges.
    for rp, info in assets.items():
        if info["suffix"] in (".gdshader", ".gdshaderinc"):
            refs_p, refs_u, includes = parse_shader(info["disk"])
            combined = refs_p | {inc for inc in includes if inc}
            edges[rp] = (combined, refs_u)
            raw_res_paths.update(combined)
            raw_uids.update(refs_u)

    # Rust sources — both the GDExtension library (lib/src) and the xtask
    # build system (src/) can load scenes by path. Scan every .rs in the repo.
    for p in REPO_ROOT.rglob("*.rs"):
        if not p.is_file():
            continue
        if any(part in RUST_EXCLUDED_DIRS for part in p.relative_to(REPO_ROOT).parts):
            continue
        text = read_text(p)
        raw_res_paths.update(RES_PATH_RE.findall(text))
        raw_uids.update(UID_REF_RE.findall(text))

    return {
        "raw_res_paths": raw_res_paths,
        "raw_uids": raw_uids,
        "class_decl": class_decl,
        "class_tokens_used": class_tokens_used,
        "edges": edges,
        "ext_resource_paths": ext_resource_paths,
        "uid_index": uid_index,
    }


def compute_reachability(assets, refs):
    uid_index = refs["uid_index"]

    # Phase 1: collect roots from direct path/uid refs + class_name usage + convention.
    roots: set[str] = set()
    for p in refs["raw_res_paths"]:
        if p in assets:
            roots.add(p)
    for u in refs["raw_uids"]:
        rp = uid_index.get(u)
        if rp:
            roots.add(rp)
    for decl_name, gd_rp in refs["class_decl"].items():
        if decl_name in refs["class_tokens_used"]:
            roots.add(gd_rp)
    for rp in CONVENTION_ROOTS:
        if rp in assets:
            roots.add(rp)
    if CONVENTION_ROOT_PREFIXES:
        for rp in assets:
            if any(rp.startswith(prefix) for prefix in CONVENTION_ROOT_PREFIXES):
                roots.add(rp)

    # Phase 2: transitive closure via scene/script/shader outgoing edges.
    reachable = set(roots)
    stack = list(roots)
    while stack:
        rp = stack.pop()
        edge = refs["edges"].get(rp)
        if not edge:
            continue
        ref_paths, ref_uids = edge
        for target in ref_paths:
            if target in assets and target not in reachable:
                reachable.add(target)
                stack.append(target)
        for ref_uid in ref_uids:
            target = uid_index.get(ref_uid)
            if target and target not in reachable:
                reachable.add(target)
                stack.append(target)
    return reachable


def find_broken(ext_resource_paths):
    broken = {}
    for rp, paths in ext_resource_paths.items():
        missing = []
        for ref in paths:
            if not ref.startswith("res://"):
                continue
            disk = res_to_disk(ref)
            if disk and not disk.exists():
                missing.append(ref)
        if missing:
            broken[rp] = missing
    return broken


def group_by_type(paths, assets):
    groups: dict[str, list[str]] = {}
    for rp in sorted(paths):
        t = assets[rp]["type"]
        groups.setdefault(t, []).append(rp)
    return groups


def delete_files(assets, paths_to_delete) -> int:
    removed = 0
    for rp in sorted(paths_to_delete):
        info = assets.get(rp)
        if not info:
            continue
        disk = info["disk"]
        try:
            disk.unlink()
            removed += 1
            print(f"  removed: {rp}")
        except OSError as e:
            print(f"  FAILED to remove {rp}: {e}", file=sys.stderr)
            continue
        # Sidecars: .uid next to scripts/shaders, .import next to imported assets.
        for sidecar_suffix in (".uid", ".import"):
            sidecar = disk.with_suffix(disk.suffix + sidecar_suffix)
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
    parser.add_argument("--delete", action="store_true", help="Delete orphan .tscn/.tres (and their .uid sidecars)")
    parser.add_argument(
        "--delete-types",
        default="",
        help="Comma-separated asset types to also delete when --delete is set "
             "(e.g. 'gd,shader,image,audio,font,model,data'). Use with care.",
    )
    parser.add_argument("--check", action="store_true", help="Exit non-zero if any orphan or broken file is found (for CI)")
    parser.add_argument(
        "--check-types",
        default="scene,resource,gd,shader",
        help="Comma-separated asset types that cause --check to fail. Default: "
             "scene,resource,gd,shader — the types the analyzer can resolve "
             "confidently. Pass 'all' to fail on any orphan type.",
    )
    args = parser.parse_args()

    assets = collect_assets()
    refs = collect_references(assets)
    reachable = compute_reachability(assets, refs)
    orphans = set(assets) - reachable
    broken = find_broken(refs["ext_resource_paths"])

    orphans_by_type = group_by_type(orphans, assets)
    totals_by_type: dict[str, int] = {}
    for info in assets.values():
        totals_by_type[info["type"]] = totals_by_type.get(info["type"], 0) + 1

    if args.json:
        print(json.dumps({
            "total": len(assets),
            "reachable": len(reachable),
            "orphans_by_type": {t: sorted(v) for t, v in orphans_by_type.items()},
            "broken": broken,
        }, indent=2))
        return 0

    print(f"godot root: {GODOT_DIR}")
    print(f"total tracked assets: {len(assets)}")
    print(f"reachable:            {len(reachable)}")
    print(f"orphan:               {len(orphans)}")
    print(f"broken:               {len(broken)}")
    print()
    print("counts by type (orphan / total):")
    for t in sorted(totals_by_type):
        orphan_n = len(orphans_by_type.get(t, []))
        print(f"  {t:8s}  {orphan_n:4d} / {totals_by_type[t]}")

    if orphans:
        print()
        print("=" * 72)
        print("ORPHAN FILES (no inbound reference)")
        print("=" * 72)
        for t in sorted(orphans_by_type):
            files = orphans_by_type[t]
            print(f"\n-- {t} ({len(files)}) --")
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
        extra_types = {t.strip() for t in args.delete_types.split(",") if t.strip()}
        # Default deletion scope: scenes and resources only.
        delete_types = {"scene", "resource"} | extra_types
        to_delete = {rp for rp in orphans if assets[rp]["type"] in delete_types}
        if not to_delete:
            print("\nnothing to delete for the selected types.")
        else:
            print()
            print("=" * 72)
            print(f"DELETING {len(to_delete)} orphan files (types: {sorted(delete_types)})")
            print("=" * 72)
            removed = delete_files(assets, to_delete)
            print(f"\nremoved {removed} file(s). broken files were NOT auto-deleted.")

    if args.check:
        if args.check_types.strip().lower() == "all":
            gating_orphans = orphans
            gating_label = "any type"
        else:
            gating_types = {t.strip() for t in args.check_types.split(",") if t.strip()}
            gating_orphans = {rp for rp in orphans if assets[rp]["type"] in gating_types}
            gating_label = ",".join(sorted(gating_types))
        if gating_orphans or broken:
            print(
                f"\n::error::{len(gating_orphans)} orphan ({gating_label}) and "
                f"{len(broken)} broken asset(s) found. Run "
                "`python3 scripts/find_unused_resources.py` locally to inspect.",
                file=sys.stderr,
            )
            return 1

    return 0


if __name__ == "__main__":
    sys.exit(main())
