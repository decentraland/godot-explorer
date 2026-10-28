---
name: i18n
description: Use whenever adding, changing, or reviewing user-facing text in this repo — a label, button, placeholder, toast, modal, notification, chat system message, or any string a player reads. Encodes the single-source translation workflow in `rn/src/i18n` (en, es, pt_BR), the `engine` area exported to `godot/locale/*.csv` for text the Godot engine draws itself, named-placeholder and plural rules, and the `rn/__tests__/i18n-coverage.test.ts` check. Trigger on "add a label/button/message", hardcoded string, `t(`, `plural(`, `rn/src/i18n`, `godot/locale`, catalogue, translation, locale, pseudolocale.
---

# i18n

`rn/src/i18n/<locale>/<area>.ts` is the **only** place text is authored. `godot/locale/*.csv` is a
generated export of the `engine` area; never edit it by hand.

## Where text goes

| The text is drawn by | Put it in | Read it with |
|---|---|---|
| React Native (nearly everything) | `rn/src/i18n/en/<area>.ts` | `t('KEY', { name })`, `plural('KEY', n, { n })` |
| Godot (emote wheel, interaction pills, chat system lines) | `rn/src/i18n/en/engine.ts` | `TranslationServer::translate` / `DclGlobal::tr` in Rust, `.tscn` `text = "KEY"` |

Rust or core code that needs text for the player emits a **key**, and RN resolves it; only the
engine's own drawing uses the `engine` area.

## Adding a string

1. Add `KEY: 'English text'` to the area file in `en/` (keys are `AREA_WORDS`, upper snake case; a new
   area is a new file spread into `en/index.ts`, `es/index.ts` and `pt_BR/index.ts`).
2. Add the `es` and `pt_BR` text in the matching files. Follow `rn/src/i18n/GLOSSARY.md` (register, terms
   that stay in English). A missing translation falls back to English, but the coverage test caps how many
   may be missing, so translate in the same PR. Engine keys must be translated in every locale.
3. Use it: `t('KEY')`. Never concatenate a key with a literal; use a named placeholder.
4. If it is an `engine` key, run `cd rn && node scripts/i18n-engine.mjs` and commit the CSVs.

## Rules

- **Named placeholders only**: `{name}`, filled by `t('KEY', { name })`. Translations may reorder them but
  never rename, drop or add one (the coverage test fails on it). No `%s` / `%d`.
- **Plurals**: `KEY` and `KEY_PLURAL`, picked by `plural('KEY', n)` (`n != 1`; pt_BR uses `n > 1`).
- **Keys are typed** (`I18nKey`): an unknown key is a compile error.
- **Dynamic keys** are fine when built from a literal prefix (`` t(`PROFILE_RARITY_${r}` as I18nKey) ``);
  the coverage test counts the prefix as a use.
- **Text that must stay literal** (brand names, dev-only strings): `// i18n-ignore` on the same or previous
  line.
- **BBCode** (`[b]`, `[color=#ccc]`) in engine strings passes through untouched.
- Do not pad spaces or newlines into labels for layout; translated text re-centres.

## Checks (all in CI via `rn_app.yml`)

```bash
cd rn
npx jest i18n-coverage            # hardcoded JSX text, unknown/unused keys, plural pairs, placeholders,
                                  # engine keys translated, untranslated-count ceiling
node scripts/i18n-engine.mjs --check   # godot/locale/*.csv match the engine area
```

The test cannot see text assembled at runtime; read the diff for that. `UNTRANSLATED_MAX` and
`UNUSED_ALLOWED` in the test are ratchets: lower or shrink them as keys are translated or removed; never
raise them to make a PR pass.

## Language switch

`setLocale()` in `rn/src/i18n/index.ts` swaps the table and remounts the root; the chosen locale is the core
`locale` setting, which the engine applies to its `TranslationServer` (`dcl_engine/src/app/locale.rs`), so
Godot-drawn text follows it. `localeFromDevice` and `detect_device_locale` must change together.
