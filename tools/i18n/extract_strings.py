#!/usr/bin/env python3
"""Extract translatable UI strings and keep them keyed.

Godot's own POT generator is editor-UI-internal (not in ClassDB, no CLI flag), so CI cannot
ask the engine to do this. This script is the replacement: stdlib only, no Godot binary, so it
runs in static_checks.yml where Python is already provisioned.

Two jobs:

  1. --check: fail if a UI string is a raw literal rather than a translation key.
  2. --check: fail if a key used in the UI is missing from the English catalogue (a key absent
     from the catalogue renders raw on screen; an empty translation falls back to English safely).

Because localization lands over several phases, (2) works as a ratchet against a baseline file
listing the literals that are not keyed *yet*. Anything not in the baseline fails immediately, so
new hardcoded strings cannot be added while the existing ones are worked through. Entries are
removed from the baseline as each phase converts its strings; the file only ever shrinks.

Usage:
    python3 tools/i18n/extract_strings.py --check             # CI: verify keys + baseline
    python3 tools/i18n/extract_strings.py --update-baseline   # re-record remaining literals
"""

import argparse
import os
import re
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import catalogue as cat  # noqa: E402

REPO_ROOT = os.path.dirname(os.path.dirname(os.path.dirname(os.path.abspath(__file__))))
UI_ROOT = os.path.join(REPO_ROOT, "godot", "src", "ui")
# Additional roots that render or declare UI text even though they sit outside src/ui:
# locale_format.gd builds dates and numbers, src/logic emits chat system messages, and
# global.gd emits them too. A path here may be a directory or a single file.
EXTRA_ROOTS = [
    os.path.join(REPO_ROOT, "godot", "src", "config"),
    os.path.join(REPO_ROOT, "godot", "src", "logic"),
    os.path.join(REPO_ROOT, "godot", "src", "global.gd"),
]
BASELINE_PATH = os.path.join(REPO_ROOT, "tools", "i18n", "unkeyed_baseline.txt")
EXEMPT_PATH = os.path.join(REPO_ROOT, "tools", "i18n", "not_translatable.txt")

# Paths excluded from localization entirely: developer tooling and debug surfaces.
EXCLUDED_DIR_RE = re.compile(
    r"(^|/)(tests?|tools?)(/|$)|debug_panel|multiplayer_debug|scene_stats_panel"
)

# Scene properties whose value is displayed to the user.
SCENE_TEXT_PROPS = ("text", "placeholder_text", "tooltip_text", "hint_tooltip")

# Custom exported properties on instanced components that also carry display text.
SCENE_CUSTOM_PROPS = ("title", "description", "custom_text", "place_holder", "underlined_text", "hint")

# A translation key: SCREAMING_SNAKE_CASE with at least one underscore.
KEY_RE = re.compile(r"^[A-Z][A-Z0-9]*(_[A-Z0-9]+)+$")

# `text = "..."` at the start of a line inside a .tscn node block.
SCENE_PROP_RE = re.compile(r'(?m)^([a-z_]+) = "((?:[^"\\]|\\.)*)"')

# GDScript display-text assignment, e.g. `label.text = "Hello"`.
# `.title` is included because DiscoverCarrousel exposes an exported `title` that assigns
# straight to a Label — three carousel headers shipped in English behind that gap.
GD_ASSIGN_RE = re.compile(r'\.(text|tooltip_text|placeholder_text|title)\s*=\s*"((?:[^"\\]|\\.)*)"')

# A translation key being resolved. tr() covers the common case; TranslationServer.translate()
# is how a *static* function must do it, since tr() is a non-static Object method.
KEY_CALL_RE = re.compile(
    # `translate(` is matched on its own because gdformat splits long chains across lines, so
    # `TranslationServer` and `. translate("KEY")` can end up on different lines.
    r'(?:\btr_?n?|\btranslate(?:_plural)?)\(\s*"((?:[^"\\]|\\.)*)"'
)

# Display text that never passes through a `.text =` assignment: returned from a helper, handed
# to a setter, or used as the head of a concatenation.
HELPER_PATTERNS = [
    re.compile(r'^\s*return\s+"((?:[^"\\]|\\.)+)"\s*$'),
    re.compile(r'^\s*return\s+.*?"((?:[^"\\]|\\.)+)".*[+%]'),
    re.compile(r'\b(?:set_text|set_value|set_title|set_warning_text|append_text|add_item|set_body|set_primary_button_text|set_secondary_button_text)\(\s*"((?:[^"\\]|\\.)+)"'),
    re.compile(r'^\s*(?:var\s+)?\w+\s*(?::=|=)\s*"((?:[^"\\]|\\.)+)"\s*[+%]'),
    # ternary else-branch: `x.text = tr("A") if c else "B"` — the assignment regex only sees
    # the head of the expression, so the else literal slipped through.
    re.compile(r'\belse\s+"((?:[^"\\]|\\.)+)"'),
    # a wrapped return whose literal and its % land on the same line inside parentheses
    re.compile(r'^\s*"((?:[^"\\]|\\.)+)"\s*%'),
    # `const SOME_LABEL = "Open external link?"` — modal_manager keeps 40+ titles, bodies and
    # button labels this way, and no assignment or setter pattern sees a bare const.
    re.compile(r'^\s*const\s+\w+\s*(?::\s*\w+\s*)?=\s*"((?:[^"\\]|\\.)+)"'),
]

# `auto_translate_mode = 2` is Node.AUTO_TRANSLATE_MODE_DISABLED.
AUTO_TRANSLATE_DISABLED_RE = re.compile(r"(?m)^auto_translate_mode\s*=\s*2\s*$")

# snake_case / kebab-case identifiers: "temp-file", "notification_bell", "feet", "green".
IDENTIFIER_RE = re.compile(r"^[a-z0-9]+([_-][a-z0-9]+)*$")

# Strings that are not prose and must never get a key: digits, punctuation, symbols,
# single letters (keycaps), counters like "0/15", and coordinates like "150, 150".
NON_PROSE_RE = re.compile(r"^[\W\d_]*$|^[A-Za-z]$|^\d+\s*/\s*\d+$|^[\d\s,.:%+#-]+$")


def is_excluded(path):
    return bool(EXCLUDED_DIR_RE.search(path.replace(os.sep, "/")))


def walk(extension, ui_root=None):
    roots = [ui_root] if ui_root else [UI_ROOT] + EXTRA_ROOTS
    for scan_root in roots:
        yield from _walk_one(extension, scan_root)


def _walk_one(extension, scan_root):
    if os.path.isfile(scan_root):
        if scan_root.endswith(extension) and not is_excluded(scan_root):
            yield scan_root
        return
    for root, _dirs, files in os.walk(scan_root):
        if is_excluded(root):
            continue
        for name in sorted(files):
            if name.endswith(extension):
                yield os.path.join(root, name)


def rel(path, repo_root=None):
    return os.path.relpath(path, repo_root or REPO_ROOT).replace(os.sep, "/")


def needs_key(value):
    """True if this display string is prose that should be a translation key."""
    if not value.strip():
        return False
    return not NON_PROSE_RE.match(value.strip())


def looks_like_prose(value):
    """Whether a bare literal is user-facing prose rather than an identifier.

    Applied only to the helper patterns, which are far noisier than a `.text =` assignment: of
    114 raw candidates in this codebase, roughly half were ids, colour names or file stems.
    Requiring an embedded space and rejecting snake_case/kebab identifiers cuts that to 65 with
    about 85% precision. It is a heuristic, so false positives are expected and get recorded in
    not_translatable.txt with a justification — over-surfacing beats silently missing prose.
    """
    value = value.strip()
    if IDENTIFIER_RE.match(value):
        return False
    if " " not in value:
        # A single token is usually an id ("temp-file", "green", "feet"). But a single ALL-CAPS
        # word is almost always a button label — "EQUIP", "OK", "SAVE" — and those hid here
        # until now, because requiring an embedded space excluded every one of them.
        if not (value.isupper() and value.isalpha() and len(value) > 1):
            return False
    return needs_key(value) and not KEY_RE.match(value)


def collect(ui_root=None, repo_root=None):
    """Return (keys, unkeyed) where each is a list of (file, string) pairs."""
    keys, unkeyed = [], []
    props = SCENE_TEXT_PROPS + SCENE_CUSTOM_PROPS

    for path in walk(".tscn", ui_root):
        with open(path, encoding="utf-8") as handle:
            content = handle.read()
        # Only look inside [node ...] blocks, so resource defaults are not treated as UI text.
        for block in re.split(r"(?m)^(?=\[)", content):
            if not block.startswith("[node"):
                continue
            # A node with auto_translate_mode = 2 (DISABLED) never runs its text through a
            # translation lookup, so its text cannot need a key — it is showing server or
            # user-generated data. The flag lives beside the node, which makes it survive
            # renames and rewording, unlike a not_translatable.txt entry matched on
            # (path, exact string).
            if AUTO_TRANSLATE_DISABLED_RE.search(block):
                continue
            for match in SCENE_PROP_RE.finditer(block):
                prop, value = match.group(1), match.group(2)
                if prop not in props:
                    continue
                if KEY_RE.match(value):
                    keys.append((rel(path, repo_root), value))
                elif needs_key(value):
                    unkeyed.append((rel(path, repo_root), value))

    for path in walk(".gd", ui_root):
        with open(path, encoding="utf-8") as handle:
            for line in handle:
                if line.lstrip().startswith("#"):
                    continue
                for match in GD_ASSIGN_RE.finditer(line):
                    value = match.group(2)
                    if KEY_RE.match(value):
                        keys.append((rel(path, repo_root), value))
                    elif needs_key(value):
                        unkeyed.append((rel(path, repo_root), value))
                for match in KEY_CALL_RE.finditer(line):
                    # No KEY_RE filter: if it is passed to tr(), it is a key by definition and
                    # must exist in the catalogue. Filtering on shape hid single-word keys.
                    if match.group(1):
                        keys.append((rel(path, repo_root), match.group(1)))

                # Prose returned from helpers, or built by concatenation, never reaches a
                # `.text =` assignment, so the scan above cannot see it. These patterns do.
                for pattern in HELPER_PATTERNS:
                    for match in pattern.finditer(line):
                        value = match.group(1)
                        if KEY_RE.match(value) or not looks_like_prose(value):
                            continue
                        unkeyed.append((rel(path, repo_root), value))

    return keys, unkeyed


def encode_value(value):
    """One line per entry, so newlines and tabs are escaped rather than stored raw."""
    return value.replace("\\", "\\\\").replace("\t", "\\t").replace("\n", "\\n")


def decode_value(value):
    out, escaped = [], False
    for char in value:
        if escaped:
            out.append({"n": "\n", "t": "\t"}.get(char, char))
            escaped = False
        elif char == "\\":
            escaped = True
        else:
            out.append(char)
    return "".join(out)


def load_baseline(path=None):
    path = path or BASELINE_PATH
    if not os.path.exists(path):
        return set()
    entries = set()
    with open(path, encoding="utf-8") as handle:
        for line in handle:
            line = line.rstrip("\n")
            if not line or line.startswith("#"):
                continue
            path, _, value = line.partition("\t")
            entries.add((path, decode_value(value)))
    return entries


def load_exemptions(path=None):
    """Strings deliberately never translated: runtime-overwritten placeholders, raw error codes
    from third-party SDKs, version strings, debug-only controls.

    Distinct from the baseline on purpose. The baseline is debt that must shrink to zero; these
    are permanent decisions, and mixing them would mean the baseline could never reach zero and
    would stop signalling anything.
    """
    return load_baseline(path or EXEMPT_PATH)


# An escape hatch, meant to stay rare. Use it only where a key genuinely cannot be seen by the
# scanner — held in a data table, returned from a function, or built at runtime — so that the
# scanner itself stays simple instead of chasing strings through indirection. Anything written as
# a plain tr("LITERAL") needs no marker and must not get one.
#
# Explicit declaration of keys the scanner cannot see, e.g.
#     # i18n-keys: TIP_WEARABLES, TIP_EMOTES
#     # i18n-keys: PROFILE_*
# A bare name declares one key. A NAME* glob declares every key-shaped literal in the same file
# that starts with NAME. Both are validated against the catalogue, so a typo fails rather than
# silently resolving to nothing at runtime.
KEY_MARKER_RE = re.compile(r"#\s*i18n-keys:\s*(.+)")
KEY_LITERAL_RE = re.compile(r'"([A-Z][A-Z0-9_]{2,})"')


def declared_keys(known_keys, ui_root=None, repo_root=None):
    """Return (declared, problems) for every `# i18n-keys:` marker in the scanned .gd files.

    Opt-in on purpose: 136 of the 260 key-shaped literals in this codebase are not translation
    keys at all (analytics event names, month abbreviations, category ids), so treating every
    SCREAMING_SNAKE literal as a key would be mostly false positives.
    """
    declared, problems = set(), []
    for path in walk(".gd", ui_root):
        with open(path, encoding="utf-8") as handle:
            content = handle.read()
        markers = KEY_MARKER_RE.findall(content)
        if not markers:
            continue
        literals = set(KEY_LITERAL_RE.findall(content))
        for marker in markers:
            for token in re.split(r"[,\s]+", marker.strip()):
                if not token:
                    continue
                if token.endswith("*"):
                    prefix = token[:-1]
                    matched = {lit for lit in literals if lit.startswith(prefix)}
                    if not matched:
                        problems.append(
                            "%s: i18n-keys marker %r matches no literal in this file"
                            % (rel(path, repo_root), token)
                        )
                    declared |= matched
                    for key in sorted(matched):
                        if key not in known_keys:
                            problems.append(
                                "%s: %s (matched by %r) is not in %s.csv"
                                % (rel(path, repo_root), key, token, cat.REFERENCE_LOCALE)
                            )
                else:
                    declared.add(token)
                    if token not in known_keys:
                        problems.append(
                            "%s: declared key %s is not in %s.csv"
                            % (rel(path, repo_root), token, cat.REFERENCE_LOCALE)
                        )
    return declared, problems


def stale_exemptions(exempt, unkeyed_raw):
    """Exemptions that no longer match anything in the tree.

    Exemptions are matched on an exact (path, string) pair, so renaming a file or rewording a
    placeholder silently orphans its entries. Left unreported they accumulate, and worse, a dead
    entry can later match a *new* string at the same path with the same text — exempting it with
    nobody having decided to. Reported as a warning rather than a failure: a rename already fails
    loudly via the strings that resurface, so failing twice adds noise, not information.
    """
    return sorted(set(exempt) - set(unkeyed_raw))


def write_baseline(unkeyed, path=None):
    header = [
        "# UI strings that are not translation keys yet, one per line as <path>\\t<string>.",
        "# The i18n check fails on any literal NOT listed here, so new hardcoded strings are",
        "# blocked while the existing ones are converted phase by phase.",
        "#",
        "# This file must only ever shrink. Remove entries as they are keyed; never add.",
        "# Regenerate with: python3 tools/i18n/extract_strings.py --update-baseline",
        "",
    ]
    lines = sorted(
        "%s\t%s" % (path, encode_value(value)) for path, value in set(unkeyed)
    )
    with open(path or BASELINE_PATH, "w", encoding="utf-8") as handle:
        handle.write("\n".join(header + lines) + "\n")


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--check", action="store_true", help="verify, do not write (for CI)")
    parser.add_argument("--update-baseline", action="store_true", help="re-record unkeyed literals")
    args = parser.parse_args()

    keys, unkeyed_raw = collect()
    exempt = load_exemptions()
    unkeyed = [entry for entry in unkeyed_raw if entry not in exempt]

    if args.update_baseline:
        write_baseline(unkeyed)
        print("Baseline updated: %d unkeyed string(s) recorded." % len(set(unkeyed)))
        if not args.check:
            return 0

    failed = False

    # Every key used in the UI must exist in the English catalogue. A key that is missing renders
    # raw on screen, which is the one failure mode the issue explicitly forbids.
    used = sorted(set(key for _path, key in keys))
    known = set(entry.key for entry in cat.read(cat.locale_path(cat.REFERENCE_LOCALE)).entries)
    missing = [key for key in used if key not in known]
    if missing:
        print("ERROR: %d key(s) used in the UI but absent from %s.csv:\n"
              % (len(missing), cat.REFERENCE_LOCALE))
        for key in missing:
            print("  " + key)
        failed = True

    # Keys referenced indirectly (through a dict, an array, or string building) are declared
    # with an `# i18n-keys:` comment rather than guessed at.
    indirect, marker_problems = declared_keys(known)
    if marker_problems:
        print("ERROR: %d i18n-keys marker problem(s):\n" % len(marker_problems))
        for problem in marker_problems:
            print("  " + problem)
        failed = True

    orphans = sorted(known - set(used) - indirect)
    if orphans:
        print("\nNote: %d catalogue key(s) are no longer used in the UI: %s%s"
              % (len(orphans), ", ".join(orphans[:5]), " ..." if len(orphans) > 5 else ""))

    baseline = load_baseline()
    new_literals = sorted(set(unkeyed) - baseline)
    if new_literals:
        print("ERROR: %d hardcoded UI string(s) without a translation key:\n" % len(new_literals))
        for path, value in new_literals:
            print('  %s: "%s"' % (path, value))
        print(
            "\nReplace each with a translation key (SCREAMING_SNAKE_CASE) and add it to"
            "\ngodot/locale/en.csv. Strings that are genuinely not prose (digits, keycaps,"
            "\nsymbols) should be reported as a false positive rather than keyed."
            "\n"
            "\nEditing an English string that was never keyed also lands here, because the new"
            "\nwording is not in the baseline. Keying it is the intended fix; if that is out of"
            "\nscope for your change, run --update-baseline instead."
        )
        exempt_paths = set(path for path, _value in exempt)
        touched = sorted(set(path for path, _value in new_literals) & exempt_paths)
        if touched:
            print(
                "\n%d of these file(s) already carry entries in not_translatable.txt:"
                "\n  %s"
                "\nIf the string is a renamed or reworded placeholder rather than new copy, update"
                "\nthat file instead of keying it — exemptions match on an exact (path, string)."
                % (len(touched), "\n  ".join(touched))
            )
        failed = True

    # Deliberately a warning, not a failure. A stale entry means a string was keyed, deleted or
    # reworded — the baseline is merely looser than it needs to be, which cannot let a new
    # hardcoded string through. Failing here would turn every unrelated PR that touches a label
    # red, and the reflex fix (--update-baseline) would loosen the ratchet rather than tighten it.
    dead = stale_exemptions(exempt, unkeyed_raw)
    if dead:
        print(
            "\nNote: %d entr(ies) in not_translatable.txt no longer match anything. A file rename"
            "\nor a reworded placeholder orphans them; delete or update them:"
            "\n  %s" % (len(dead), "\n  ".join('%s: "%s"' % e for e in dead[:10]))
        )

    stale = sorted(baseline - set(unkeyed))
    if stale:
        print(
            "\nNote: %d baseline entr(ies) no longer exist. Tighten the baseline when convenient:"
            "\n  python3 tools/i18n/extract_strings.py --update-baseline" % len(stale)
        )

    if not failed:
        print(
            "i18n check passed: %d key(s) in use, %d string(s) still awaiting conversion."
            % (len(used), len(baseline))
        )
    return 1 if failed else 0


if __name__ == "__main__":
    sys.exit(main())
