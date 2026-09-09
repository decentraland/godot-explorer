class_name LocaleSettings extends RefCounted

## UI language handling.
##
## Mirrors the static-apply pattern of GeneralSettings / GraphicSettings / AudioSettings:
## a `class_name` helper with no autoload wiring, called as `LocaleSettings.apply_locale()`.
## Locale logic deliberately lives here rather than on `Global` — `global.gd` sits exactly at
## gdlint's `max-public-methods: 45` ceiling, so adding methods there fails static checks.
##
## Translations are per-locale CSV catalogues under `res://locale/` (en.csv, es.csv, pt_BR.csv),
## imported by Godot's built-in `csv_translation` importer into `.translation` resources that are
## listed in project.godot under `[internationalization]`.

## Locales the language picker may offer.
##
## A locale MUST NOT be listed here until its catalogue is complete. An incomplete table is the
## failure mode behind unity-explorer#270 (ES/IT rolled back after shipping half-translated)
## and #2062 (a Spanish OS locale hung the desktop loading screen). English is the fallback
## and is always available.
const SUPPORTED_LOCALES: PackedStringArray = ["en", "es", "pt_BR"]

## Display name per locale, shown in the Settings picker. Deliberately written in the
## language itself — a user who cannot read the current UI language still needs to find
## their own in the list.
const LOCALE_DISPLAY_NAMES: Dictionary = {
	"en": "English",
	"es": "Español",
	"pt_BR": "Português (Brasil)",
	PSEUDO_LOCALE: "Pseudolocale (QA)",
}

const FALLBACK_LOCALE: String = "en"

## Pseudolocale — a QA aid, not a language.
##
## Selecting it keeps the UI in English but turns on Godot's pseudolocalization, which brackets
## and pads every translated string (`«____Sign Out____»`). The padding simulates ES/PT running
## 20-30% longer, so clipping, truncation and bad wrapping show up before any translation exists.
##
## Offered in every non-production build, which is the same gate the Dev Tools tab uses
## (`settings.gd`: `button_developer.visible = !Global.is_production()`).
##
## Note it does NOT prove a string is translated: Godot pseudolocalizes the untranslated fallback
## too, so a hardcoded English string is bracketed just the same. See tools/i18n/README.md.
const PSEUDO_LOCALE: String = "xx-pseudo"

## TEMPORARY: start every install in English rather than following the device locale.
##
## Only the default changes — the picker still offers every supported locale and an explicit
## choice is still honoured and persisted. Flip this back to `true` to restore device detection;
## [method detect_device_locale] is deliberately left untouched so that is a one-line change
## rather than a rewrite.
const FOLLOW_DEVICE_LOCALE: bool = false


## TEMPORARY: whether the Settings language picker is offered at all.
##
## Same gate as the pseudolocale and the Dev Tools tab. Together with
## [constant FOLLOW_DEVICE_LOCALE] this makes a production build behave exactly as it did before
## the localization work landed — English, with no way to change it — while internal builds get
## the full picker. Remove this gate (and flip the constant) to ship the locales.
static func is_language_picker_available() -> bool:
	return not Global.is_production()


## Whether the pseudolocale may be offered in the picker.
##
## Deliberately not gated on [method OS.is_debug_build]: CI exports with `--export-release`, so
## that check hid the option from the very builds QA installs. `is_production()` is what actually
## separates a shippable build from an internal one — only `release*` branches are built `--prod`.
static func is_pseudolocale_available() -> bool:
	return not Global.is_production()


## The locales the picker should list, including the pseudolocale in debug builds.
static func selectable_locales() -> PackedStringArray:
	var locales := PackedStringArray(SUPPORTED_LOCALES)
	if is_pseudolocale_available():
		locales.append(PSEUDO_LOCALE)
	return locales


## Resolve the locale to actually use: the saved override when it is still supported,
## otherwise the device locale, otherwise English.
##
## While [constant FOLLOW_DEVICE_LOCALE] is false the device step is skipped, so an install with
## no saved choice resolves to English regardless of the system language.
static func resolve_locale() -> String:
	# TEMPORARY: a production build is English regardless of what is saved. The setting persists
	# in user data that a non-production build may have written (same app id, same device), so
	# hiding the picker alone would still let a stored "es" surface in a store build.
	if not is_language_picker_available():
		return FALLBACK_LOCALE

	var configured: String = Global.get_config().locale
	if configured == PSEUDO_LOCALE and is_pseudolocale_available():
		# The pseudolocale renders English through Godot's pseudolocalization filter.
		return FALLBACK_LOCALE
	if not configured.is_empty() and configured in SUPPORTED_LOCALES:
		return configured
	return detect_device_locale() if FOLLOW_DEVICE_LOCALE else FALLBACK_LOCALE


## Best-supported match for the device's locale, falling back to English.
##
## Regional variants fold to their base language when the base is supported but the exact
## variant is not, so es_AR / es_MX / es_419 all resolve to "es".
static func detect_device_locale() -> String:
	var device: String = OS.get_locale()  # e.g. "es_AR", "pt_BR", "en_US"
	if device in SUPPORTED_LOCALES:
		return device

	var language: String = OS.get_locale_language()  # e.g. "es"
	if language in SUPPORTED_LOCALES:
		return language

	# A supported regional variant of the same language is better than English.
	for supported in SUPPORTED_LOCALES:
		if supported.begins_with(language + "_"):
			return supported

	return FALLBACK_LOCALE


## Push the resolved locale into the TranslationServer.
##
## Control nodes re-translate themselves on NOTIFICATION_TRANSLATION_CHANGED, so scene `text`
## properties update live. Text assigned from GDScript does NOT — those components refresh
## themselves via `_notification(NOTIFICATION_TRANSLATION_CHANGED)`.
static func apply_locale() -> void:
	TranslationServer.set_locale(resolve_locale())
	var pseudo: bool = Global.get_config().locale == PSEUDO_LOCALE and is_pseudolocale_available()
	TranslationServer.set_pseudolocalization_enabled(pseudo)


## Persist a language choice and apply it immediately.
##
## Pass an empty string to clear the override and follow the device locale again.
static func set_locale(new_locale: String) -> void:
	if not new_locale.is_empty() and not new_locale in selectable_locales():
		push_warning("LocaleSettings: ignoring unsupported locale '%s'" % new_locale)
		return

	var config := Global.get_config()
	config.locale = new_locale
	config.save_to_settings_file()
	apply_locale()


## Human-readable name for a locale, for the Settings picker.
static func get_display_name(locale: String) -> String:
	return LOCALE_DISPLAY_NAMES.get(locale, locale)
