# Localization tooling (#2667)

Per-locale CSV catalogues under `godot/locale/`, imported by Godot's built-in `csv_translation`
importer. Stdlib Python only — no gettext, no bash — so every check runs on Windows, macOS, Linux
and CI with nothing installed.

## Commands

```sh
python3 tools/i18n/format_csv.py             # rewrite catalogues in canonical form
python3 tools/i18n/format_csv.py --check     # CI: canonical form + validity
python3 tools/i18n/extract_strings.py --check        # CI: every UI string is keyed
python3 tools/i18n/extract_strings.py --update-baseline
python3 tools/i18n/format_csv.py --record-sources    # after updating translations
python3 -m unittest discover -s tools/i18n -p 'test_*.py'
```

## Files

| File | Meaning |
|---|---|
| `godot/locale/{en,es,pt_BR}.csv` | the catalogues; `;`-delimited, every field quoted |
| `unkeyed_baseline.txt` | debt: strings not keyed **yet**. Must only shrink. Currently 0. |
| `not_translatable.txt` | permanent decisions: placeholders, ids, debug text. Grows deliberately. |
| `translation_sources.json` | hash of the English each translation was made against (staleness) |

Adding a locale means: create `<locale>.csv`, add it to `locale/translations` in `project.godot`
**only once it has translations** (an empty column imports as `<name>.en.translation` and the
reference would dangle), and add it to `SUPPORTED_LOCALES` in `locale_settings.gd` only when the
catalogue is complete — see unity-explorer#270.

## Passing keys through APIs

A display API takes **one of two things**, and which one is not obvious from `String`:

| The node it writes to | What the caller must pass |
|---|---|
| auto-translates (no `auto_translate_mode`) | a **key** — the node looks it up, and re-looks-it-up on a language change |
| `auto_translate_mode = 2` | **finished text** — usually `tr(...)`, or data that must never be looked up |

Guessing wrong fails silently in both directions: a key sent to a mode-2 node renders as
`MODAL_ADD_EMAIL_TITLE` on screen, and prose sent to an auto-translating node simply misses the
lookup and looks fine in English forever. An entire modal system, an OTP flow and nine toasts
shipped in English behind that ambiguity.

**New display APIs should type the parameter** so the engine settles it:

```gdscript
func set_body(body: TranslationKey) -> void      # give me a key, I translate it
func show_toast(text: String) -> void            # give me finished text or data
```

Passing a literal to the first is a **parse error**, caught by `cargo run -- check-gdscript`.
`Array[TranslationKey]` works the same way. `TranslationKey` lives in
`godot/src/config/translation_key.gd`; `raw()` gives the key for an auto-translating node, `text()`
the translated string for a mode-2 one, plus `format()`, `format_named()`, `plural()` and `upper()`.

**Never concatenate a key with a literal.** `set_body(CONNECTION_LOST_BODY + " Try again.")`
produces a string matching no key, so the lookup misses and the raw key is drawn on screen — that
shipped. Pass the pieces:

```gdscript
modal.set_body(TranslationKey.join([
	TranslationKey.new("MODAL_CONNECTION_LOST_BODY"),
	TranslationKey.new("MODAL_TRY_RESTARTING_APP"),
]))
```

Each piece is translated on its own, so a translator can rewrite one paragraph without touching the
others and the separator stays presentation rather than copy.

Existing APIs still take `String`; they are converted opportunistically when a file is touched, not
in one sweep. The checker recognises `TranslationKey.new("KEY")` as key usage, and flags prose
passed to it.

## Declaring keys the scanner cannot see

An escape hatch, meant to stay rare (3 uses today). Only where a key lives in a data table or a
function return rather than a literal `tr("KEY")`:

```gdscript
# i18n-keys: TIP_WEARABLES, TIP_EMOTES     # explicit list
# i18n-keys: PROFILE_*                     # every matching literal in this file
```

Both forms are validated against the catalogue, so a typo fails the build.

## Pseudolocale sweep

The layout and leak check. **CI cannot run it** — full pages boot autoloads and networking — so it
is a manual pass on a running client.

```gdscript
TranslationServer.set_pseudolocalization_enabled(true)
```

Configured in `project.godot` as **brackets + expansion, without accent substitution**, so a
translated string renders as `«____Sign Out____»`.

**What it does and does not prove.** Godot pseudolocalizes the *output* of the translation lookup,
including the fallback — so a string that was never in the catalogue is bracketed just the same.
Verified live: the server-supplied place name `Winterfest Hockey 2026` renders as
`«____Winterfest Hockey 2026____»`. **Brackets therefore do not distinguish translated from
hardcoded.** What the sweep is actually for:

* **Layout** — the `expansion_ratio=0.4` padding simulates ES/PT running 20–30% longer, exposing
  clipping, truncation and bad wrapping. This is the main value.
* **Bypass detection** — text that never goes through `atr()` at all (a node with
  `auto_translate_mode = NEVER`, or text drawn manually) stays unbracketed and stands out.

**To find leaks, inspect the raw `.text` instead**, over the live tree via the debug hub — a node
whose raw text is neither a translation key nor server data is a hardcoded string. That is how the
`event_pills_bar` month abbreviations were found: `"APR"` is a single token, so the scanner's prose
filter skipped it, and the concatenation joined it with a bare `" "`.

```bash
cargo run -- debug-hub &
cargo run -- run -- --scene-inspector=ws://127.0.0.1:9231 &
.claude/skills/mobile-dev-debug-tool/scripts/unified.sh eval '<walk the tree, report non-key prose>'
```

Note that `.text` returns the *assigned* value (the key), never the rendered translation — use
`node.atr(node.text)` if you want what is actually drawn.

The markers are `«` `»` rather than the more obvious `⟦` `⟧`: Inter has no glyph for U+27E6/27E7,
so those render as tofu or fall through to a system font. `[` `]` would collide with BBCode.

Accent substitution is deliberately **off**. With it on, Godot also mangles BBCode tags
(`[ćôłôŕ=#ÐF́9ÇF́F́]`, which stops parsing) and named `{placeholder}` fields (which then fail to
substitute and render literally). Both produce false alarms. It was only ever needed to prove font
coverage, and that was verified statically instead: all 13 Inter faces cover Latin-1 Supplement
with no subsetting.

### Known blind spot

`tr_n()` output is **not** pseudolocalized, so plural strings render unbracketed and look like
leaks. There are 8, and they are correct — do not chase them:

```
CREDITS_PURCHASED            PLACE_DURATION_DAY
NOTIFICATION_DAYS_AGO        PLACE_DURATION_HR
NOTIFICATION_HOURS_AGO       PLACE_DURATION_MIN
NOTIFICATION_MINUTES_AGO     SOCIAL_MUTUAL_FRIENDS
```

Also expect single-token display fallbacks (`"Unknown"`, `"Someone"`) to appear unbracketed. Those
are genuine leaks: the scanner's prose filter requires an embedded space, so it cannot see them.
The sweep is how they get found.

### While sweeping, also check runtime switching

Change language with each screen open and look for text that does *not* change. Scene `text`
properties re-translate themselves; anything assigned from GDScript does not unless its component
overrides `_notification(NOTIFICATION_TRANSLATION_CHANGED)`. Three components do so far — the
settings page, the wearable filter buttons and the tip card. Panels rebuilt on open are fine; a
persistent one that goes stale needs the same hook.
