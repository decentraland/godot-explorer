#!/usr/bin/env python3
"""Tests for the i18n string extractor.

Run:  python3 -m unittest discover -s tools/i18n -p 'test_*.py'

Stdlib unittest only, matching the repo's no-extra-dependency convention for Python tooling
(tests/check_asset_imports.py, scripts/*.py). Every test builds a throwaway UI tree in a temp
directory, so nothing here touches godot/src or the real baseline.
"""

import os
import shutil
import tempfile
import unittest

import catalogue as cat
import extract_strings as ex
import format_csv as fmt

DEFAULT_RULE = "nplurals=2; plural=(n != 1);"


class Fixture:
    """A temp repo with a UI tree, a baseline and a locale dir."""

    def __init__(self):
        self.root = tempfile.mkdtemp()
        self.ui = os.path.join(self.root, "godot", "src", "ui")
        self.locale = os.path.join(self.root, "godot", "locale")
        os.makedirs(self.ui)
        os.makedirs(self.locale)
        self.baseline = os.path.join(self.root, "baseline.txt")
        self.exempt = os.path.join(self.root, "not_translatable.txt")

    def scene(self, name, *properties):
        """Write a .tscn with one node block carrying the given `prop = "value"` lines."""
        body = '[gd_scene format=3]\n\n[node name="Root" type="Control"]\n'
        for prop, value in properties:
            body += '%s = "%s"\n' % (prop, value)
        path = os.path.join(self.ui, name)
        with open(path, "w", encoding="utf-8") as handle:
            handle.write(body)
        return path

    def script(self, name, body):
        path = os.path.join(self.ui, name)
        with open(path, "w", encoding="utf-8") as handle:
            handle.write(body)
        return path

    def csv(self, locale, entries=(), rule=DEFAULT_RULE):
        """entries: iterable of (key, forms) or (key, forms, context, plural_key)."""
        items = []
        for item in entries:
            key, forms = item[0], item[1]
            context = item[2] if len(item) > 2 else ""
            plural_key = item[3] if len(item) > 3 else ""
            forms = [forms] if isinstance(forms, str) else list(forms)
            items.append(cat.Entry(key=key, context=context, plural_key=plural_key, forms=forms))
        path = cat.locale_path(locale, self.locale)
        cat.write(cat.Catalogue(locale=locale, plural_rule=rule, entries=items), path)
        return path

    def collect(self):
        return ex.collect(ui_root=self.ui, repo_root=self.root)

    def unkeyed(self):
        return set(self.collect()[1])

    def keys(self):
        return set(key for _path, key in self.collect()[0])

    def write_baseline(self, entries):
        ex.write_baseline(entries, path=self.baseline)

    def load_baseline(self):
        return ex.load_baseline(path=self.baseline)

    def write_exemptions(self, entries):
        ex.write_baseline(entries, path=self.exempt)

    def load_exemptions(self):
        return ex.load_exemptions(path=self.exempt)

    def effective_unkeyed(self):
        """Unkeyed strings after exemptions are applied, as main() computes it."""
        return self.unkeyed() - self.load_exemptions()

    def new_literals(self):
        """What the CI check would fail on: unkeyed strings absent from the baseline."""
        return self.effective_unkeyed() - self.load_baseline()

    def stale_exemptions(self):
        return ex.stale_exemptions(self.load_exemptions(), self.unkeyed())

    def translation_problems(self):
        return [p for p in fmt.validate(locale_dir=self.locale, sources={})
                if not p.startswith("project.godot")]

    def cleanup(self):
        shutil.rmtree(self.root, ignore_errors=True)


class ExtractorTestCase(unittest.TestCase):
    def setUp(self):
        self.fx = Fixture()
        self.addCleanup(self.fx.cleanup)


class TestDetection(ExtractorTestCase):
    def test_prose_is_reported_as_unkeyed(self):
        self.fx.scene("a.tscn", ("text", "Sign Out"))
        self.assertEqual(self.fx.unkeyed(), {("godot/src/ui/a.tscn", "Sign Out")})

    def test_translation_key_is_not_unkeyed(self):
        self.fx.scene("a.tscn", ("text", "COMMON_SIGN_OUT"))
        self.assertEqual(self.fx.unkeyed(), set())
        self.assertEqual(self.fx.keys(), {"COMMON_SIGN_OUT"})

    def test_placeholder_and_custom_props_are_scanned(self):
        self.fx.scene("a.tscn", ("placeholder_text", "Search"), ("title", "Settings"))
        self.assertEqual(len(self.fx.unkeyed()), 2)

    def test_non_prose_is_ignored(self):
        # Digits, keycaps, counters, coordinates and symbols must never be keyed.
        for value in ["1", "99+", "F", "0/15", "150, 150", "#", "#####", "", "   "]:
            with self.subTest(value=value):
                self.fx.scene("a.tscn", ("text", value))
                self.assertEqual(self.fx.unkeyed(), set(), "%r should not need a key" % value)

    def test_formatted_number_is_flagged_for_a_human_decision(self):
        # "%d%%" looks like pure formatting, but percent spacing is locale-dependent (Spanish
        # prefers "50 %"). Flagging costs one baseline line; not flagging ships untranslatable
        # text, so the checker errs toward surfacing it.
        self.fx.scene("a.tscn", ("text", "%d%%"))
        self.assertEqual(self.fx.unkeyed(), {("godot/src/ui/a.tscn", "%d%%")})

    def test_gdscript_assignment_is_scanned(self):
        self.fx.script("a.gd", 'func f():\n\tlabel.text = "Hello there"\n')
        self.assertEqual(self.fx.unkeyed(), {("godot/src/ui/a.gd", "Hello there")})

    def test_gdscript_comments_are_ignored(self):
        self.fx.script("a.gd", 'func f():\n\t# label.text = "Commented out"\n')
        self.assertEqual(self.fx.unkeyed(), set())

    def test_excluded_directories_are_skipped(self):
        for excluded in ["test", "tools", "debug_panel"]:
            with self.subTest(dir=excluded):
                sub = os.path.join(self.fx.ui, excluded)
                os.makedirs(sub, exist_ok=True)
                with open(os.path.join(sub, "x.tscn"), "w", encoding="utf-8") as handle:
                    handle.write('[node name="R" type="Control"]\ntext = "Debug Only"\n')
                self.assertEqual(self.fx.unkeyed(), set())

    def test_only_node_blocks_are_scanned(self):
        # A known limitation, pinned so a future change to it is deliberate: text living in a
        # [sub_resource] block is not scanned.
        path = os.path.join(self.fx.ui, "a.tscn")
        with open(path, "w", encoding="utf-8") as handle:
            handle.write('[sub_resource type="X" id="1"]\ntext = "Inside Sub Resource"\n')
        self.assertEqual(self.fx.unkeyed(), set())


class TestHelperComposedStrings(ExtractorTestCase):
    """Prose that never reaches a `.text =` assignment.

    These were invisible until Step 1: the scanner only looked at assignments and scene
    properties, so ~61 strings returned from helpers sat untranslated while the baseline
    reported 3 remaining.
    """

    def test_prose_returned_from_a_helper_is_found(self):
        self.fx.script("a.gd", 'func f() -> String:\n\treturn "Friend Request Received"\n')
        self.assertEqual(self.fx.unkeyed(), {("godot/src/ui/a.gd", "Friend Request Received")})

    def test_prose_returned_with_formatting_is_found(self):
        self.fx.script("a.gd", 'func f() -> String:\n\treturn "Sent to %s" % who\n')
        self.assertEqual(self.fx.unkeyed(), {("godot/src/ui/a.gd", "Sent to %s")})

    def test_setter_argument_is_found(self):
        self.fx.script("a.gd", 'func f():\n\tlabel.set_value("Not connected yet")\n')
        self.assertEqual(self.fx.unkeyed(), {("godot/src/ui/a.gd", "Not connected yet")})

    def test_head_of_a_concatenation_is_found(self):
        self.fx.script("a.gd", 'func f():\n\tvar s = "Created by " + creator\n')
        self.assertEqual(self.fx.unkeyed(), {("godot/src/ui/a.gd", "Created by ")})

    def test_identifiers_and_single_tokens_are_not_prose(self):
        # The heuristic that took candidates from 114 to 65: ids, colours and file stems are
        # returned from -> String functions just as prose is, so the type is no help.
        for value in ["temp-file", "notification_bell", "green", "feet", "main"]:
            with self.subTest(value=value):
                self.fx.script("a.gd", 'func f() -> String:\n\treturn "%s"\n' % value)
                self.assertEqual(self.fx.unkeyed(), set(), "%r is not prose" % value)

    def test_returning_a_key_is_not_flagged(self):
        self.fx.script("a.gd", 'func f() -> String:\n\treturn tr("NOTIF_HEADER")\n')
        self.assertEqual(self.fx.unkeyed(), set())
        self.assertEqual(self.fx.keys(), {"NOTIF_HEADER"})

    def test_static_context_translate_call_counts_as_key_usage(self):
        # tr() is a non-static Object method, so a static helper must use TranslationServer;
        # that form was previously invisible to the key scan.
        self.fx.script("a.gd", 'static func f() -> String:\n\treturn TranslationServer.translate("NOTIF_HEADER")\n')
        self.assertEqual(self.fx.keys(), {"NOTIF_HEADER"})
        self.assertEqual(self.fx.unkeyed(), set())

    def test_translation_key_type_counts_as_key_usage(self):
        # TranslationKey makes the mistake a parse error at the call site, so the scanner only
        # has to notice the key here rather than learn every display API's parameter list.
        self.fx.script("a.gd", 'func f() -> void:\n\tmodal.set_body(TranslationKey.new("MODAL_BODY"))\n')
        self.assertEqual(self.fx.keys(), {"MODAL_BODY"})
        self.assertEqual(self.fx.unkeyed(), set())

    def test_prose_inside_translation_key_is_flagged(self):
        # The type stops a String reaching a TranslationKey *parameter*, but
        # TranslationKey.new("Try again.") is still type-legal — the debug guard in _init
        # catches it at runtime, and this catches it in CI.
        self.fx.script("a.gd", 'func f() -> void:\n\tmodal.set_body(TranslationKey.new("Try restarting the app."))\n')
        self.assertEqual(
            self.fx.unkeyed(), {("godot/src/ui/a.gd", "Try restarting the app.")}
        )

    def test_debug_surfaces_are_excluded_wholesale(self):
        for excluded in ["multiplayer_debug", "scene_stats_panel"]:
            with self.subTest(dir=excluded):
                sub = os.path.join(self.fx.ui, excluded)
                os.makedirs(sub, exist_ok=True)
                with open(os.path.join(sub, "x.gd"), "w", encoding="utf-8") as handle:
                    handle.write('func f() -> String:\n\treturn "Pulse: not in build"\n')
                self.assertEqual(self.fx.unkeyed(), set())


class TestAddRemoveModify(ExtractorTestCase):
    """The three things a developer actually does to a line of UI text."""

    def setUp(self):
        super().setUp()
        self.fx.scene("a.tscn", ("text", "Sign Out"))
        self.fx.write_baseline(self.fx.unkeyed())
        self.assertEqual(self.fx.new_literals(), set(), "fixture should start clean")

    def test_add_new_literal_fails(self):
        self.fx.scene("a.tscn", ("text", "Sign Out"), ("text", "Enable Turbo Mode"))
        self.assertEqual(
            self.fx.new_literals(), {("godot/src/ui/a.tscn", "Enable Turbo Mode")}
        )

    def test_add_new_key_passes(self):
        self.fx.scene("a.tscn", ("text", "Sign Out"), ("text", "SETTINGS_TURBO_MODE"))
        self.assertEqual(self.fx.new_literals(), set())
        self.assertIn("SETTINGS_TURBO_MODE", self.fx.keys())

    def test_add_literal_to_a_file_already_in_the_baseline_still_fails(self):
        # The baseline is keyed on (path, string), so debt in a file does not license more of it.
        self.fx.scene("a.tscn", ("text", "Sign Out"), ("text", "Another New String"))
        self.assertIn(("godot/src/ui/a.tscn", "Another New String"), self.fx.new_literals())

    def test_remove_literal_leaves_a_stale_entry_but_no_new_literal(self):
        self.fx.scene("a.tscn", ("text", "COMMON_SIGN_OUT"))
        self.assertEqual(self.fx.new_literals(), set())
        stale = self.fx.load_baseline() - self.fx.unkeyed()
        self.assertEqual(stale, {("godot/src/ui/a.tscn", "Sign Out")})

    def test_delete_whole_file_leaves_a_stale_entry_but_no_new_literal(self):
        os.remove(os.path.join(self.fx.ui, "a.tscn"))
        self.assertEqual(self.fx.new_literals(), set())
        self.assertEqual(len(self.fx.load_baseline() - self.fx.unkeyed()), 1)

    def test_modify_literal_is_reported_as_new(self):
        # Rewording an unkeyed string surfaces it. Intended: the fix is to key it. The error
        # message calls this case out explicitly so it is not mistaken for a tool bug.
        self.fx.scene("a.tscn", ("text", "Log Out"))
        self.assertEqual(self.fx.new_literals(), {("godot/src/ui/a.tscn", "Log Out")})

    def test_move_file_is_reported_as_new(self):
        # The baseline records paths, so relocating a component surfaces its strings.
        os.remove(os.path.join(self.fx.ui, "a.tscn"))
        os.makedirs(os.path.join(self.fx.ui, "pages"), exist_ok=True)
        self.fx.scene(os.path.join("pages", "a.tscn"), ("text", "Sign Out"))
        self.assertEqual(
            self.fx.new_literals(), {("godot/src/ui/pages/a.tscn", "Sign Out")}
        )

    def test_updating_the_baseline_clears_the_failure(self):
        self.fx.scene("a.tscn", ("text", "Log Out"))
        self.assertTrue(self.fx.new_literals())
        self.fx.write_baseline(self.fx.unkeyed())
        self.assertEqual(self.fx.new_literals(), set())


class TestExemptions(ExtractorTestCase):
    """not_translatable.txt records permanent decisions, matched on an exact (path, string).

    These pin what happens when the tree moves underneath those entries — the same matrix the
    baseline has. Their absence is why the rename/reword behaviour went unnoticed.
    """

    def setUp(self):
        super().setUp()
        self.fx.scene("a.tscn", ("text", "Lazaro"))
        self.fx.write_exemptions({("godot/src/ui/a.tscn", "Lazaro")})

    def test_exempt_string_is_not_reported(self):
        self.assertEqual(self.fx.effective_unkeyed(), set())
        self.assertEqual(self.fx.stale_exemptions(), [])

    def test_reworded_placeholder_resurfaces_and_orphans_its_entry(self):
        self.fx.scene("a.tscn", ("text", "Lazarus"))
        self.assertEqual(self.fx.effective_unkeyed(), {("godot/src/ui/a.tscn", "Lazarus")})
        self.assertEqual(self.fx.stale_exemptions(), [("godot/src/ui/a.tscn", "Lazaro")])

    def test_renamed_file_resurfaces_every_entry_for_that_file(self):
        os.remove(os.path.join(self.fx.ui, "a.tscn"))
        self.fx.scene("b.tscn", ("text", "Lazaro"))
        self.assertEqual(self.fx.effective_unkeyed(), {("godot/src/ui/b.tscn", "Lazaro")})
        self.assertEqual(self.fx.stale_exemptions(), [("godot/src/ui/a.tscn", "Lazaro")])

    def test_deleted_string_leaves_a_reported_stale_entry(self):
        # Previously silent: the entry died with nothing to warn about it.
        os.remove(os.path.join(self.fx.ui, "a.tscn"))
        self.assertEqual(self.fx.effective_unkeyed(), set())
        self.assertEqual(self.fx.stale_exemptions(), [("godot/src/ui/a.tscn", "Lazaro")])

    def test_exemption_is_scoped_to_its_path(self):
        # The same text elsewhere is NOT exempt; a decision is about one site, not a string.
        self.fx.scene("b.tscn", ("text", "Lazaro"))
        self.assertEqual(self.fx.effective_unkeyed(), {("godot/src/ui/b.tscn", "Lazaro")})

    def test_exemption_takes_precedence_over_the_baseline(self):
        self.fx.write_baseline(self.fx.unkeyed())
        self.assertEqual(self.fx.new_literals(), set())
        self.assertEqual(self.fx.effective_unkeyed(), set())

    def test_exempt_values_with_newlines_and_tabs_round_trip(self):
        entries = {("godot/src/ui/a.tscn", "Ooops...\nWent Wrong"),
                   ("godot/src/ui/a.tscn", "Tabbed\tvalue")}
        self.fx.write_exemptions(entries)
        self.assertEqual(self.fx.load_exemptions(), entries)


class TestKeyMarkers(ExtractorTestCase):
    """`# i18n-keys:` declares keys the scanner cannot see.

    Opt-in on purpose: most SCREAMING_SNAKE literals in this codebase are analytics event names,
    month abbreviations and category ids, not translation keys, so treating them all as keys
    would be mostly false positives.
    """

    def declared(self, known):
        return ex.declared_keys(known, ui_root=self.fx.ui, repo_root=self.fx.root)

    def test_explicit_keys_are_declared(self):
        self.fx.script("a.gd", "# i18n-keys: TIP_A, TIP_B\nconst K = [\"TIP_A\", \"TIP_B\"]\n")
        declared, problems = self.declared({"TIP_A", "TIP_B"})
        self.assertEqual(declared, {"TIP_A", "TIP_B"})
        self.assertEqual(problems, [])

    def test_declared_key_missing_from_the_catalogue_is_reported(self):
        self.fx.script("a.gd", "# i18n-keys: TIP_A, TIP_TYPO\n")
        _declared, problems = self.declared({"TIP_A"})
        self.assertEqual(len(problems), 1)
        self.assertIn("TIP_TYPO", problems[0])

    def test_glob_declares_matching_literals_in_the_same_file(self):
        self.fx.script("a.gd", '# i18n-keys: PROFILE_*\nconst T = {"key": "PROFILE_GENDER_FEMALE"}\n')
        declared, problems = self.declared({"PROFILE_GENDER_FEMALE"})
        self.assertEqual(declared, {"PROFILE_GENDER_FEMALE"})
        self.assertEqual(problems, [])

    def test_glob_catches_a_misspelt_key(self):
        # The case the old heuristic could not catch: it only counted literals already in the
        # catalogue, so a typo was simply "not used" rather than an error.
        self.fx.script("a.gd", '# i18n-keys: PROFILE_*\nconst T = {"key": "PROFILE_GENDER_FEMAIL"}\n')
        _declared, problems = self.declared({"PROFILE_GENDER_FEMALE"})
        self.assertEqual(len(problems), 1)
        self.assertIn("PROFILE_GENDER_FEMAIL", problems[0])

    def test_glob_matching_nothing_is_reported(self):
        # Otherwise a renamed key prefix leaves a marker that silently guards nothing.
        self.fx.script("a.gd", '# i18n-keys: NOTHING_*\nconst T = {"key": "PROFILE_GENDER_FEMALE"}\n')
        _declared, problems = self.declared({"PROFILE_GENDER_FEMALE"})
        self.assertEqual(len(problems), 1)
        self.assertIn("matches no literal", problems[0])

    def test_files_without_markers_declare_nothing(self):
        self.fx.script("a.gd", 'const T = {"key": "PROFILE_GENDER_FEMALE"}\n')
        declared, problems = self.declared({"PROFILE_GENDER_FEMALE"})
        self.assertEqual(declared, set())
        self.assertEqual(problems, [])


class TestBaselineRoundTrip(ExtractorTestCase):
    def test_multiline_and_tab_values_survive(self):
        entries = {
            ("a.tscn", "Something\nwent wrong"),
            ("b.tscn", "Tabbed\tvalue"),
            ("c.tscn", "Back\\slash"),
        }
        self.fx.write_baseline(entries)
        self.assertEqual(self.fx.load_baseline(), entries)

    def test_multiline_scene_string_is_detected_and_round_trips(self):
        path = os.path.join(self.fx.ui, "a.tscn")
        with open(path, "w", encoding="utf-8") as handle:
            handle.write('[node name="R" type="Control"]\ntext = "Something\nwent wrong"\n')
        unkeyed = self.fx.unkeyed()
        self.assertEqual(unkeyed, {("godot/src/ui/a.tscn", "Something\nwent wrong")})
        self.fx.write_baseline(unkeyed)
        self.assertEqual(self.fx.load_baseline(), unkeyed)
        self.assertEqual(self.fx.new_literals(), set())


class TestCatalogue(ExtractorTestCase):
    def test_canonical_order_is_sorted_and_idempotent(self):
        self.fx.csv("en", [("Z_LAST", "z"), ("A_FIRST", "a")])
        with open(cat.locale_path("en", self.fx.locale), encoding="utf-8") as handle:
            text = handle.read()
        self.assertLess(text.index("A_FIRST"), text.index("Z_LAST"))
        once = cat.dumps(cat.read(cat.locale_path("en", self.fx.locale)))
        cat.write(cat.read(cat.locale_path("en", self.fx.locale)), cat.locale_path("en", self.fx.locale))
        twice = cat.dumps(cat.read(cat.locale_path("en", self.fx.locale)))
        self.assertEqual(once, twice)

    def test_semicolons_quotes_newlines_and_backslashes_round_trip(self):
        # unescape_translations=false, so CSV quoting alone must carry all of these. A literal
        # backslash is the one that broke under unescape_translations=true.
        values = {
            "K_SEMI": "uno; dos; tres",
            "K_QUOTE": '[url="https://x.test/"]Ir[/url]',
            "K_NEWLINE": "Primera\nSegunda",
            "K_BSLASH": r"C:\ruta\archivo",
            "K_ALL": 'back\\slash "q" ; semi\nnewline',
        }
        self.fx.csv("en", list(values.items()))
        got = {e.key: e.text for e in cat.read(cat.locale_path("en", self.fx.locale)).entries}
        self.assertEqual(got, values)

    def test_plural_group_round_trips_through_continuation_rows(self):
        self.fx.csv("en", [("K", ["%d friend", "%d friends"], "", "K_PLURAL")])
        entry = cat.read(cat.locale_path("en", self.fx.locale)).by_key()[("K", "")]
        self.assertEqual(entry.plural_key, "K_PLURAL")
        self.assertEqual(entry.forms, ["%d friend", "%d friends"])

    def test_untranslated_plural_keeps_all_its_forms(self):
        # The normal pre-translation state: a plural entry with every form empty. Its continuation
        # row is a row of empty cells, which must NOT be mistaken for a blank line and dropped —
        # doing so silently turned a 2-form entry into a 1-form one and failed validation.
        self.fx.csv("es", [("K", ["", ""], "", "K_PLURAL")])
        entry = cat.read(cat.locale_path("es", self.fx.locale)).by_key()[("K", "")]
        self.assertEqual(entry.forms, ["", ""])
        self.fx.csv("en", [("K", ["one", "many"], "", "K_PLURAL")])
        self.assertEqual(self.fx.translation_problems(), [])

    def test_context_distinguishes_two_entries_with_one_key(self):
        self.fx.csv("en", [("K", "On", "button"), ("K", "Active", "label")])
        entries = cat.read(cat.locale_path("en", self.fx.locale)).by_key()
        self.assertEqual(entries[("K", "button")].text, "On")
        self.assertEqual(entries[("K", "label")].text, "Active")

    def test_continuation_row_without_a_plural_entry_is_rejected(self):
        path = cat.locale_path("en", self.fx.locale)
        with open(path, "w", encoding="utf-8") as handle:
            handle.write('"keys";"?plural";"?context";"en"\n')
            handle.write('"?pluralrule";"";"";"%s"\n' % DEFAULT_RULE)
            handle.write('"K";"";"";"plain"\n')
            handle.write('"";"";"";"orphan continuation"\n')
        with self.assertRaises(ValueError):
            cat.read(path)


class TestValidation(ExtractorTestCase):
    def test_matching_format_specifiers_pass(self):
        self.fx.csv("en", [("K", "sent to %s")])
        self.fx.csv("es", [("K", "enviado a %s")])
        self.assertEqual(self.fx.translation_problems(), [])

    def test_mismatched_format_specifier_is_reported(self):
        self.fx.csv("en", [("K", "sent to %s")])
        self.fx.csv("es", [("K", "enviado a %d")])
        problems = self.fx.translation_problems()
        self.assertEqual(len(problems), 1)
        self.assertIn("expects ['s'] but has ['d']", problems[0])

    def test_reordered_specifiers_are_reported(self):
        self.fx.csv("en", [("K", "%s has %d")])
        self.fx.csv("es", [("K", "%d tiene %s")])
        self.assertEqual(len(self.fx.translation_problems()), 1)

    def test_dropped_specifier_is_reported(self):
        self.fx.csv("en", [("K", "sent to %s")])
        self.fx.csv("es", [("K", "enviado")])
        self.assertEqual(len(self.fx.translation_problems()), 1)

    def test_literal_percent_is_not_a_specifier(self):
        self.fx.csv("en", [("K", "100%% done")])
        self.fx.csv("es", [("K", "100%% hecho")])
        self.assertEqual(self.fx.translation_problems(), [])

    def test_empty_translation_is_allowed(self):
        # Untranslated falls back to English; that is the normal mid-translation state.
        self.fx.csv("en", [("K", "sent to %s")])
        self.fx.csv("es", [("K", "")])
        self.assertEqual(self.fx.translation_problems(), [])

    def test_key_missing_from_reference_is_reported(self):
        self.fx.csv("en", [])
        self.fx.csv("es", [("ORPHAN", "huerfano")])
        problems = self.fx.translation_problems()
        self.assertEqual(len(problems), 1)
        self.assertIn("not in en.csv", problems[0])

    def test_plural_forms_are_each_checked(self):
        self.fx.csv("en", [("K", ["%d friend", "%d friends"], "", "K_PLURAL")])
        self.fx.csv("es", [("K", ["%d amigo", "amigos"], "", "K_PLURAL")])
        problems = self.fx.translation_problems()
        self.assertEqual(len(problems), 1)
        self.assertIn("[1]", problems[0])

    def test_wrong_plural_row_count_is_reported(self):
        # nplurals=2 but only one form: a dropped continuation row must not pass silently.
        self.fx.csv("en", [("K", ["%d friend"], "", "K_PLURAL")])
        problems = self.fx.translation_problems()
        self.assertTrue(any("expected 2" in p for p in problems), problems)

    def test_plural_key_must_be_named_after_its_singular(self):
        # Godot never registers the plural companion as a message of its own (it lives in
        # the ?plural column), so the only way to answer "does K_PLURAL exist?" is to strip
        # the suffix back to K -- which TranslationKey.is_known() does. If the name drifts,
        # that lookup silently reports a real key as missing.
        self.fx.csv("en", [("K", ["%d friend", "%d friends"], "", "K_MANY")])
        problems = self.fx.translation_problems()
        self.assertTrue(any("expected 'K_PLURAL'" in p for p in problems), problems)

    def test_duplicate_key_is_reported(self):
        self.fx.csv("en", [("K", "one"), ("K", "two")])
        self.assertTrue(any("duplicate" in p for p in self.fx.translation_problems()))

    def test_missing_plural_rule_is_reported(self):
        self.fx.csv("en", [("K", "x")], rule="")
        self.assertTrue(any("?pluralrule" in p for p in self.fx.translation_problems()))

    def test_stale_translation_is_reported(self):
        self.fx.csv("en", [("K", "New English")])
        self.fx.csv("es", [("K", "Espanol")])
        sources = {"es": {"K": fmt.source_hash("Old English")}}
        problems = [p for p in fmt.validate(locale_dir=self.fx.locale, sources=sources)
                    if not p.startswith("project.godot")]
        self.assertEqual(len(problems), 1)
        self.assertIn("stale", problems[0])

    def test_fresh_translation_is_not_stale(self):
        self.fx.csv("en", [("K", "English")])
        self.fx.csv("es", [("K", "Espanol")])
        sources = {"es": {"K": fmt.source_hash("English")}}
        problems = [p for p in fmt.validate(locale_dir=self.fx.locale, sources=sources)
                    if not p.startswith("project.godot")]
        self.assertEqual(problems, [])


if __name__ == "__main__":
    unittest.main()
