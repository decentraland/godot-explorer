---
name: i18n
description: Use whenever adding, changing, or reviewing user-facing text in this repo — a label, button, placeholder, toast, modal, notification, chat system message, or any string a player reads. Encodes the translation-key workflow behind `godot/locale/*.csv` and `tools/i18n/`: when a node needs a raw KEY versus finished text (`auto_translate_mode`), the `TranslationKey` type, named-placeholder and plural rules, the scanner's blind spots and the `# i18n-keys:` escape hatch, and the checks that must pass. Trigger on "add a label/button/message", hardcoded string, `tr(`, `TranslationKey`, `godot/locale`, `tools/i18n`, catalogue, translation, locale, pseudolocale.
---

# Decentraland Godot Explorer — localization

Every player-visible string is a **key** in `godot/locale/en.csv`, translated in `es.csv` and
`pt_BR.csv`. CI fails on any new hardcoded UI text. Read this before adding a string.

The failure this system exists to prevent is **silent**: a key sent to the wrong kind of node
renders as `MODAL_ADD_EMAIL_TITLE` on screen, and prose sent to a translating node simply never
gets looked up and stays English forever. An entire modal system, an OTP flow and nine toasts
shipped that way before the type below existed.

## The one rule that matters: key or finished text?

A display node is one of two kinds, and which one decides what you assign to it.

| The node | What to give it | Why |
|---|---|---|
| **auto-translating** (no `auto_translate_mode` in the `.tscn`) | a **raw KEY** — `"SETTINGS_AUDIO"` | the node looks it up itself, and re-looks-it-up on a language change |
| **`auto_translate_mode = 2`** (DISABLED) | **finished text** — `tr("KEY")` / `TranslationKey.text()` | it never looks anything up; it normally carries server or user data |

Mode 2 exists for nodes that show data — a username, a scene title, a creator's PointerEvents
text. Those must never go through a lookup.

**A mode-2 node holding a key is always a bug** and fails CI (`extract_strings.py`). If a node
sometimes shows data and sometimes shows our copy, keep it mode 2 and assign `tr(...)` from
GDScript — then add a `_notification(NOTIFICATION_TRANSLATION_CHANGED)` hook, because text
assigned from code does **not** re-translate itself.

## Adding a string

1. **Pick a key.** `SCREAMING_SNAKE_CASE`, namespaced by area: `SETTINGS_*`, `PROFILE_*`,
   `MODAL_*`, `NOTIF_*`, `BACKPACK_*`, `COMMON_*` for shared labels. Reuse an existing key rather
   than adding a synonym — 34 English strings are already shared by two or more keys.
2. **Add it to all three catalogues.** English is the source; `es` and `pt_BR` must be filled
   (see `tools/i18n/glossary.md` for register and the terms that stay in English).
3. **Reference it** — a raw key in the `.tscn` for an auto-translating node, or
   `TranslationKey.new("KEY")` / `tr("KEY")` from GDScript for a mode-2 one.
4. **Run the checks** (bottom of this file), including `--record-sources`.

## `TranslationKey`

`godot/src/config/translation_key.gd`. Typing a parameter `TranslationKey` makes "passed English
where a key belongs" a **parse error**, caught by `cargo run -- check-gdscript`:

```gdscript
func set_body(body: TranslationKey) -> void      # give me a key, I translate it
func show_toast(text: String) -> void            # give me finished text or data
```

- `raw()` — the key, for an auto-translating node (prefer this there; it re-translates itself)
- `text()` — the translated string, for a mode-2 node
- `format(values: Dictionary)` — translated then filled by **name**
- `plural(count)` — selects the grammatical form, **unsubstituted**
- `upper()`, `exists()`, `is_key()`, `is_known()`, `many()`, `join()`

**Never concatenate a key with a literal.** `set_body(CONNECTION_LOST_BODY + " Try again.")`
matches no entry, so the raw key is drawn on screen — that shipped. Pass the pieces:

```gdscript
modal.set_body_parts([
    TranslationKey.new("MODAL_CONNECTION_LOST_BODY"),
    TranslationKey.new("MODAL_TRY_RESTARTING_APP"),
])
```

## Placeholders are named, never positional

`format_csv.py --check` **rejects any `%` specifier in the catalogue.** A positional argument
pins the word order to English, and word order is the first thing a translation changes:
`"I like %s %s"` cannot become *"me gustan las flores rojas"* — the adjective has to move.

```gdscript
tr("TOAST_WORLD_UNAVAILABLE_BODY").format({"world": name, "error": reason})
```

Names may be reordered and repeated; the check compares the *set* of names and fails on a dropped
or invented one. That matters because `String.format()` leaves an unfilled `{field}` in the
output — a typo is drawn on screen as literal braces.

**`String.format()` substitutes text and nothing else.** There is no width, precision or padding
syntax — `{n:.2f}` is emitted verbatim, not applied. Render the value first, in the caller:

```gdscript
key.format({"seconds": "%02d" % secs, "size": LocaleFormat.number(mb, 1)})
```

Use `LocaleFormat` (`godot/src/config/locale_format.gd`) for anything locale-shaped:
`number()`, `time_of_day()`, `short_date()`, `month_name()`, `weekday_name()`.

## Plurals

Two steps: `plural()` picks the form, `format()` fills it.

```gdscript
TranslationKey.new("SOCIAL_MUTUAL_FRIENDS").plural(n).format({"friends": LocaleFormat.number(n)})
```

The plural entry lives in the `?plural` column and **must** be named `<KEY>_PLURAL` (enforced).
`count` selects the grammatical form; what the reader sees is whatever you put in the field, so
the two can differ without a special case. Note `auto_translate` calls `tr()` and never `tr_n()`,
so a plural string can only ever be set from code — never left as a scene default.

## The scanner's blind spots

`extract_strings.py` scans `godot/src/ui` plus `config/`, `logic/`, `notifications/`,
`global.gd`, `connection_quality_monitor.gd`, `notifications_manager.gd`. Within those it sees
`.tscn` display properties, `.text =` assignments, and a fixed list of setter patterns
(`set_text`, `add_item`, `set_body`, `return "…"`, bare `const`…). It does **not** see:

- **strings passed as function arguments** — `_add_row(template, "Scene Lights", …)`
- **data tables** — an `Array[String]` of display names in another file
- **`@export var` defaults** — these silently clobber a correct key set in the `.tscn`
  (`dcl_text_edit.gd` did exactly this)
- **single lowercase tokens** — `"weekly"` looks like an identifier to the prose filter
- **anything on a mode-2 node** — skipped wholesale, since those normally carry data

When a key is real but invisible to the scanner, declare it so the catalogue check still
validates it:

```gdscript
# i18n-keys: SETTINGS_GRAPHIC_PROFILE_*, SETTINGS_SKYBOX_*
# i18n-keys: TOOLTIP_VIEW_PROFILE
```

Both forms are validated against the catalogue, so a typo fails the build.

## Anti-patterns — do not do these

- **Don't pad with spaces or newlines for layout.** `text = "     METAMASK"` and `"RESET ALL\n"`
  both shipped. A translated label re-centres, so the padding stops lining up. Use
  `theme_override_constants/separation`, a container, or `custom_minimum_size`.
- **Don't let server metadata shadow a key.** `metadata.get("title", tr(KEY))` means the English
  server value always wins and the key never fires. Prefer our key for types we recognise.
- **Don't assign resolved text to an auto-translating node.** It freezes in the old locale on a
  language change. Assign the key.
- **Don't put translator notes in `?context`.** That column is part of the message identity —
  `tr(key, context)` looks up the pair — so text there breaks every lookup. It is a
  disambiguator, and this catalogue does not need one: the key already disambiguates.
- **Don't translate a value that is also an identifier.** If a string is compared anywhere
  (`if entry.text == "View profile"`), make the comparison use the key and translate only at the
  point of display.
- **Don't add a locale to `SUPPORTED_LOCALES` until its catalogue is complete.** Shipping a
  half-translated locale is the failure behind unity-explorer#270.

## Text from Rust

Rust carries no catalogue. If a Rust path must produce player-visible text, emit a **key** and
resolve it in GDScript — see `scene_manager.rs` emitting `TOOLTIP_VIEW_PROFILE`, resolved in
`tooltip_label.gd`. Do not blanket-translate a field that also carries creator-authored text.

## Verification checklist

```sh
python3 tools/i18n/format_csv.py                 # canonicalize the catalogues
python3 tools/i18n/format_csv.py --check         # plural naming, named-field parity, staleness
python3 tools/i18n/extract_strings.py --check    # every UI string keyed; no key on a mode-2 node
python3 -m unittest discover -s tools/i18n -p 'test_*.py'
python3 tools/i18n/format_csv.py --record-sources # after editing translations
gdformat godot/ && gdlint godot/
cargo run -- check-gdscript                      # TranslationKey misuse is a parse error
cargo run -- test-i18n                           # headless TranslationKey suite
```

- [ ] Key exists in **all three** catalogues; `es`/`pt_BR` actually translated.
- [ ] The node's `auto_translate_mode` matches what you assigned (key vs finished text).
- [ ] Placeholders are `{named}`; no `%` specifier in the catalogue.
- [ ] Text assigned from GDScript re-translates via `_notification(NOTIFICATION_TRANSLATION_CHANGED)`.
- [ ] `--record-sources` run, so a later English edit flags its translations as stale.
- [ ] After changing a `.csv`, `cargo run -- import-assets` — the `.translation` binaries are
      build artifacts and go stale otherwise (tests will fail against the old text).

## Checking it live

CI cannot render anything, so layout and leaks are a manual pass. Settings offers a
**Pseudolocale (QA)** language in non-production builds that brackets and pads every string
(`«____Sign Out____»`) to expose clipping. Brackets do **not** prove a string is translated —
Godot pseudolocalizes the untranslated fallback too.

To find leaks, inspect raw `.text` over the live tree with the `mobile-dev-debug-tool` skill.
Three scans, each catching a different failure:

1. auto-translating nodes whose text is **not** a key → prose that will never be looked up
2. mode-2 nodes whose text matches an **English catalogue value** → text that went stale
3. mode-2 nodes whose text **is** a key → the key is being drawn on screen

Only the loaded tree is visible, so open the pages you changed first — most screens are not
instantiated at boot, which is how ten defects survived a green CI run.

## Reference

- `tools/i18n/README.md` — the workflow in full, including the pseudolocale sweep
- `tools/i18n/glossary.md` — register (informal *tú* / *você*) and terms kept in English
- `tools/i18n/not_translatable.txt` — permanent exclusions, each with a justification
- `tools/i18n/unkeyed_baseline.txt` — debt ratchet, currently 0; it may only shrink
