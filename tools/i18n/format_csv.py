#!/usr/bin/env python3
"""Canonicalise and validate the translation catalogues.

Mirrors the write/check duality the repo already uses for GDScript (`gdformat` vs `gdformat -d`)
and Rust (`cargo fmt` vs `cargo fmt --check`):

    python3 tools/i18n/format_csv.py                   rewrite in canonical form
    python3 tools/i18n/format_csv.py --check           verify, exit 1 on any problem (CI)
    python3 tools/i18n/format_csv.py --record-sources  re-record English source hashes

Canonical form is *ours* rather than a tool's output, so it is deterministic and identical on every
platform: header, `?pluralrule`, then entries sorted by (key, context), `;`-delimited with every
field quoted. Stdlib only — no gettext, no bash, runs the same on Windows, macOS, Linux and CI.

Validations, in the order a reader cares about:
  * structure — parseable, one locale column, continuation rows attached to a plural entry
  * plural groups exactly `nplurals` rows, so a dropped or inserted row cannot pass silently
  * no duplicate (key, context)
  * every key present in the reference (en) catalogue
  * format specifiers agreeing with the reference per key, checked per plural form
  * translations not stale against the recorded English source hash
  * project.godot referencing exactly those locales that actually carry translations
"""

import argparse
import hashlib
import json
import os
import re
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import catalogue as cat  # noqa: E402

SOURCES_PATH = os.path.join(os.path.dirname(os.path.abspath(__file__)), "translation_sources.json")
PROJECT_GODOT = os.path.join(cat.REPO_ROOT, "godot", "project.godot")


def source_hash(text):
    return hashlib.sha1(text.encode("utf-8")).hexdigest()[:12]


def load_sources(path=None):
    path = path or SOURCES_PATH
    if not os.path.exists(path):
        return {}
    with open(path, encoding="utf-8") as handle:
        return json.load(handle)


def save_sources(data, path=None):
    with open(path or SOURCES_PATH, "w", encoding="utf-8") as handle:
        json.dump(data, handle, indent=2, sort_keys=True, ensure_ascii=False)
        handle.write("\n")


def translated_locales(locale_dir=None):
    """Locales whose catalogue carries at least one translation.

    Godot derives a Translation's locale from its messages: a column with no translations yields a
    resource that falls back to locale "en", so the imported artifact is misnamed (es.en.translation
    rather than es.es.translation). Referencing such a locale from project.godot would therefore
    point at a file that does not exist. This is also the #270 discipline — never enable a locale
    before its table has content.
    """
    out = []
    for locale in cat.locales(locale_dir):
        catalogue = cat.read(cat.locale_path(locale, locale_dir))
        if any(form for entry in catalogue.entries for form in entry.forms):
            out.append(locale)
    return out


def check_project_godot(locale_dir=None, project_path=None):
    problems = []
    path = project_path or PROJECT_GODOT
    if not os.path.exists(path):
        return problems
    with open(path, encoding="utf-8") as handle:
        content = handle.read()
    match = re.search(r"locale/translations=PackedStringArray\(([^)]*)\)", content)
    if not match:
        return ["project.godot: no locale/translations entry"]
    referenced = set(re.findall(r'res://locale/([A-Za-z_]+)\.[A-Za-z_]+\.translation', match.group(1)))
    expected = set(translated_locales(locale_dir))
    for locale in sorted(expected - referenced):
        problems.append(
            "project.godot: %s.csv has translations but is not referenced in locale/translations"
            % locale
        )
    for locale in sorted(referenced - expected):
        problems.append(
            "project.godot: references %s but %s.csv has no translations, so the imported "
            "artifact is named %s.en.translation and the reference dangles" % (locale, locale, locale)
        )
    return problems


def validate(locale_dir=None, sources=None):
    """Return a list of human-readable problems. Empty means the catalogues are sound."""
    problems = []
    sources = load_sources() if sources is None else sources
    locales = cat.locales(locale_dir)
    if cat.REFERENCE_LOCALE not in locales:
        return ["missing reference catalogue %s.csv" % cat.REFERENCE_LOCALE]

    reference = cat.read(cat.locale_path(cat.REFERENCE_LOCALE, locale_dir))
    ref_entries = reference.by_key()

    for locale in locales:
        name = "%s.csv" % locale
        try:
            catalogue = cat.read(cat.locale_path(locale, locale_dir))
        except ValueError as error:
            problems.append(str(error))
            continue

        if catalogue.locale != locale:
            problems.append("%s: locale column is %r, expected %r" % (name, catalogue.locale, locale))
        nplurals = catalogue.nplurals
        if nplurals is None:
            problems.append("%s: missing or unparseable ?pluralrule row" % name)

        seen = set()
        for entry in catalogue.entries:
            ident = (entry.key, entry.context)
            if ident in seen:
                problems.append(
                    "%s: duplicate key %r%s"
                    % (name, entry.key, " (context %r)" % entry.context if entry.context else "")
                )
            seen.add(ident)

            expected_forms = nplurals if entry.is_plural and nplurals else 1
            if len(entry.forms) != expected_forms:
                problems.append(
                    "%s: %r has %d form(s), expected %d (nplurals=%s)"
                    % (name, entry.key, len(entry.forms), expected_forms, nplurals)
                )

            if locale == cat.REFERENCE_LOCALE:
                continue

            if ident not in ref_entries:
                problems.append("%s: key %r is not in %s.csv" % (name, entry.key, cat.REFERENCE_LOCALE))
                continue

            ref = ref_entries[ident]
            for index, form in enumerate(entry.forms):
                if not form:
                    continue  # untranslated: falls back to English, which is the normal state
                ref_form = ref.forms[index] if index < len(ref.forms) else ref.text
                want, got = cat.format_specs(ref_form), cat.format_specs(form)
                if want != got:
                    problems.append(
                        "%s: %r%s expects %s but has %s"
                        % (
                            name,
                            entry.key,
                            "[%d]" % index if len(entry.forms) > 1 else "",
                            want or "no format specifiers",
                            got or "none",
                        )
                    )

            recorded = sources.get(locale, {}).get(entry.key)
            if any(entry.forms) and recorded and recorded != source_hash(ref.text):
                problems.append(
                    "%s: %r is stale — the English source changed since it was translated "
                    "(re-translate, then run --record-sources)" % (name, entry.key)
                )

    problems.extend(check_project_godot(locale_dir))
    return problems


def record_sources(locale_dir=None):
    reference = cat.read(cat.locale_path(cat.REFERENCE_LOCALE, locale_dir))
    ref_entries = reference.by_key()
    data = {}
    for locale in cat.locales(locale_dir):
        if locale == cat.REFERENCE_LOCALE:
            continue
        catalogue = cat.read(cat.locale_path(locale, locale_dir))
        recorded = {}
        for entry in catalogue.entries:
            if any(entry.forms) and (entry.key, entry.context) in ref_entries:
                recorded[entry.key] = source_hash(ref_entries[(entry.key, entry.context)].text)
        if recorded:
            data[locale] = recorded
    save_sources(data)
    return sum(len(v) for v in data.values())


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--check", action="store_true", help="verify, do not write (for CI)")
    parser.add_argument("--record-sources", action="store_true", help="re-record English source hashes")
    args = parser.parse_args()

    locales = cat.locales()
    if not locales:
        print("No catalogues in godot/locale/ — nothing to do.")
        return 0

    if args.record_sources:
        print("Recorded %d source hash(es)." % record_sources())

    failed = False
    for locale in locales:
        path = cat.locale_path(locale)
        try:
            catalogue = cat.read(path)
        except ValueError as error:
            print("ERROR: %s" % error)
            failed = True
            continue
        canonical = cat.dumps(catalogue)
        with open(path, encoding="utf-8", newline="") as handle:
            current = handle.read()
        if args.check:
            if current != canonical:
                print("ERROR: %s.csv is not in canonical form. Run: python3 tools/i18n/format_csv.py" % locale)
                failed = True
        elif current != canonical:
            with open(path, "w", encoding="utf-8", newline="") as handle:
                handle.write(canonical)
            print("Formatted %s.csv" % locale)

    problems = validate()
    if problems:
        print("\nERROR: %d catalogue problem(s):\n" % len(problems))
        for problem in problems:
            print("  " + problem)
        failed = True

    if not failed:
        total = len(cat.read(cat.locale_path(cat.REFERENCE_LOCALE)).entries)
        print("Catalogue check passed: %d key(s) across %d locale(s)." % (total, len(locales)))
    return 1 if failed else 0


if __name__ == "__main__":
    sys.exit(main())
