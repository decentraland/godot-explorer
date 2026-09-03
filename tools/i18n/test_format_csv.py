#!/usr/bin/env python3
"""Tests for the catalogue validator's placeholder rules.

Run:  python3 -m unittest discover -s tools/i18n -p 'test_*.py'

Stdlib unittest only, matching the repo convention. Every test builds throwaway catalogues in a
temp directory, so nothing here touches godot/locale.
"""

import shutil
import tempfile
import unittest

import catalogue as cat
import format_csv as fmt

DEFAULT_RULE = "nplurals=2; plural=(n != 1);"


class PlaceholderRules(unittest.TestCase):
    """Positional args are rejected; named fields are checked by name, not by position."""

    def setUp(self):
        self.dir = tempfile.mkdtemp()

    def tearDown(self):
        shutil.rmtree(self.dir, ignore_errors=True)

    def write(self, locale, entries):
        cat.write(
            cat.Catalogue(
                locale=locale,
                plural_rule=DEFAULT_RULE,
                entries=[cat.Entry(key=key, forms=[form]) for key, form in entries],
            ),
            cat.locale_path(locale, self.dir),
        )

    def problems(self, en, es):
        """Validation problems for a two-locale catalogue, minus the project.godot wiring."""
        self.write("en", en)
        self.write("es", es)
        return [
            problem
            for problem in fmt.validate(locale_dir=self.dir, sources={})
            if "project.godot" not in problem
        ]

    def test_positional_specifier_is_rejected(self):
        # "I like %s %s" cannot become "las flores rojas": the adjective has to move, and a
        # positional arg cannot. Rejected in the reference too, so it never gets translated.
        problems = self.problems([("A", "Hello %s")], [("A", "Hola %s")])
        self.assertEqual(len(problems), 2, problems)
        self.assertTrue(all("positional %s" in problem for problem in problems), problems)

    def test_named_fields_may_be_reordered_and_repeated(self):
        # The whole point of naming them: word order is what a translation changes.
        self.assertEqual(
            self.problems([("A", "{a} then {b}")], [("A", "{b} luego {a} ({a})")]), []
        )

    def test_dropped_named_field_is_reported(self):
        # String.format() leaves an unfilled {field} in the output, so a dropped field is a
        # visible defect rather than a silently missing word.
        problems = self.problems([("A", "Hello {name}")], [("A", "Hola")])
        self.assertEqual(len(problems), 1, problems)
        self.assertIn("missing {name}", problems[0])

    def test_invented_named_field_is_reported(self):
        # A translated field name renders as literal braces on screen.
        problems = self.problems([("A", "Hello {name}")], [("A", "Hola {nombre}")])
        self.assertEqual(len(problems), 1, problems)
        self.assertIn("missing {name}", problems[0])
        self.assertIn("unknown {nombre}", problems[0])

    def test_untranslated_entry_is_not_checked_for_fields(self):
        # An empty translation falls back to English, which is the normal pre-translation state.
        self.assertEqual(self.problems([("A", "Hello {name}")], [("A", "")]), [])


if __name__ == "__main__":
    unittest.main()
