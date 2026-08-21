#!/usr/bin/env python3
"""Read, write and validate the Godot CSV translation catalogues.

One file per locale under godot/locale/ (en.csv, es.csv, pt_BR.csv), imported directly by Godot's
built-in `csv_translation` importer. Stdlib only, so the whole toolchain runs on Windows, macOS,
Linux and CI with nothing installed.

File shape, as understood by Godot 4.6:

    "keys";"?plural";"?context";"<locale>"
    "?pluralrule";"";"";"nplurals=2; plural=(n != 1);"
    "COMMON_CANCEL";"";"";"Cancelar"
    "FRIENDS_MUTUAL";"FRIENDS_MUTUAL_MANY";"";"%d amigo en comun"
    "";"";"";"%d amigos en comun"          <- continuation row: the next plural form

  * `?plural` holds the *plural key*; extra forms follow in rows with an empty `keys` cell.
  * `?pluralrule` is a row carrying this locale's rule; `nplurals` from it fixes how many rows
    a plural group must have, which is what makes a dropped or inserted row detectable.
  * `?context` disambiguates one key into several translations (e.g. "On" as a button vs a label).

Two settings are load-bearing and were established by importing fixtures, not from documentation
(the importer's escape options are undocumented):

  * `delimiter=1` is semicolon. The enum hint in the binary is `Comma,Semicolon,Tab` with no
    explicit indices, so this is positional; verified by importing a `;`-delimited file.
  * `unescape_translations=false`. With it *true*, Godot applies C-style unescaping that mangles
    literal backslashes — `C:\\ruta\\archivo` came back as `C:` CR `uta` BEL `rchivo`, because `\\`
    is not honoured as an escaped backslash before `\\r`/`\\a` are substituted. With it false,
    CSV quoting alone carries semicolons, quotes, backslashes and newlines losslessly. The cost is
    that a translation containing a newline spans several lines inside its quoted cell.
"""

import csv
import os
import re

REPO_ROOT = os.path.dirname(os.path.dirname(os.path.dirname(os.path.abspath(__file__))))
LOCALE_DIR = os.path.join(REPO_ROOT, "godot", "locale")
REFERENCE_LOCALE = "en"

KEYS_COL = "keys"
PLURAL_COL = "?plural"
CONTEXT_COL = "?context"
PLURAL_RULE_KEY = "?pluralrule"

DELIMITER = ";"

# Matches "nplurals=N;" in a Plural-Forms style rule.
NPLURALS_RE = re.compile(r"nplurals\s*=\s*(\d+)")

# A printf-style conversion. `%%` is a literal percent and carries no argument.
FORMAT_SPEC_RE = re.compile(r"%(?:%|(?:\d+\$)?[-+ #0]*[\d*]*(?:\.\d+)?([diouxXeEfgGcsp]))")


class Entry:
    """One translatable message: a key, optional context, and one form per plural index."""

    def __init__(self, key, context="", plural_key="", forms=None):
        self.key = key
        self.context = context
        self.plural_key = plural_key
        self.forms = list(forms or [])

    @property
    def is_plural(self):
        return bool(self.plural_key)

    @property
    def text(self):
        return self.forms[0] if self.forms else ""

    def sort_key(self):
        return (self.key, self.context)

    def __repr__(self):
        return "Entry(%r, context=%r, plural_key=%r, forms=%r)" % (
            self.key,
            self.context,
            self.plural_key,
            self.forms,
        )


class Catalogue:
    def __init__(self, locale, plural_rule="", entries=None):
        self.locale = locale
        self.plural_rule = plural_rule
        self.entries = list(entries or [])

    @property
    def nplurals(self):
        match = NPLURALS_RE.search(self.plural_rule or "")
        return int(match.group(1)) if match else None

    def by_key(self):
        return {(e.key, e.context): e for e in self.entries}


def locale_path(locale, locale_dir=None):
    return os.path.join(locale_dir or LOCALE_DIR, "%s.csv" % locale)


def locales(locale_dir=None):
    directory = locale_dir or LOCALE_DIR
    if not os.path.isdir(directory):
        return []
    return sorted(
        name[:-4] for name in os.listdir(directory) if name.endswith(".csv")
    )


def read(path):
    """Parse one locale CSV. Raises ValueError on structural problems."""
    with open(path, encoding="utf-8", newline="") as handle:
        rows = list(csv.reader(handle, delimiter=DELIMITER))

    # Drop only genuinely blank lines. A row of empty cells is NOT blank: it is the
    # continuation row of an untranslated plural form, which is the normal state for a
    # locale before translation. Filtering on "no non-empty cell" silently ate those.
    rows = [r for r in rows if r]
    if not rows:
        raise ValueError("%s is empty" % os.path.basename(path))

    header = rows[0]
    if not header or header[0] != KEYS_COL:
        raise ValueError(
            "%s: first column must be %r, found %r"
            % (os.path.basename(path), KEYS_COL, header[0] if header else None)
        )
    try:
        plural_idx = header.index(PLURAL_COL)
        context_idx = header.index(CONTEXT_COL)
    except ValueError:
        raise ValueError(
            "%s: header must contain %r and %r columns"
            % (os.path.basename(path), PLURAL_COL, CONTEXT_COL)
        )

    value_indices = [
        i for i, name in enumerate(header) if i not in (0, plural_idx, context_idx)
    ]
    if len(value_indices) != 1:
        raise ValueError(
            "%s: expected exactly one locale column, found %d"
            % (os.path.basename(path), len(value_indices))
        )
    value_idx = value_indices[0]
    locale = header[value_idx]

    def cell(row, index):
        return row[index] if index < len(row) else ""

    plural_rule, entries, current = "", [], None
    for number, row in enumerate(rows[1:], start=2):
        key = cell(row, 0)
        if key == PLURAL_RULE_KEY:
            plural_rule = cell(row, value_idx)
            current = None
            continue
        if not key:
            # Continuation row: the next plural form of the entry above.
            if current is None or not current.is_plural:
                raise ValueError(
                    "%s line %d: continuation row does not follow a plural entry"
                    % (os.path.basename(path), number)
                )
            current.forms.append(cell(row, value_idx))
            continue
        current = Entry(
            key=key,
            context=cell(row, context_idx),
            plural_key=cell(row, plural_idx),
            forms=[cell(row, value_idx)],
        )
        entries.append(current)

    return Catalogue(locale=locale, plural_rule=plural_rule, entries=entries)


def render(catalogue):
    """Canonical rows: header, ?pluralrule, then entries sorted by (key, context)."""
    rows = [[KEYS_COL, PLURAL_COL, CONTEXT_COL, catalogue.locale]]
    rows.append([PLURAL_RULE_KEY, "", "", catalogue.plural_rule])
    for entry in sorted(catalogue.entries, key=Entry.sort_key):
        forms = entry.forms or [""]
        rows.append([entry.key, entry.plural_key, entry.context, forms[0]])
        for form in forms[1:]:
            rows.append(["", "", "", form])
    return rows


def write(catalogue, path):
    with open(path, "w", encoding="utf-8", newline="") as handle:
        csv.writer(
            handle, delimiter=DELIMITER, quoting=csv.QUOTE_ALL, lineterminator="\n"
        ).writerows(render(catalogue))


def dumps(catalogue):
    import io

    buffer = io.StringIO()
    csv.writer(
        buffer, delimiter=DELIMITER, quoting=csv.QUOTE_ALL, lineterminator="\n"
    ).writerows(render(catalogue))
    return buffer.getvalue()


def format_specs(text):
    """Ordered conversion types, e.g. 'a %s b %d' -> ['s', 'd']."""
    return [m.group(1) for m in FORMAT_SPEC_RE.finditer(text) if m.group(1)]
