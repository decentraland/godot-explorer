class_name LocaleFormat
extends RefCounted

## Locale-aware number and date formatting.
##
## Godot exposes no locale-aware number or date formatter — String.num() always emits "." as the
## decimal separator and never groups thousands, and Time only returns raw components. So the
## conventions live here, alongside LocaleSettings, following the same all-static pattern as
## GeneralSettings / GraphicSettings.
##
## Division of labour:
##   * STRUCTURAL differences live in this file — decimal separator, thousands grouping, and
##     whether the locale uses a 12- or 24-hour clock. A translator cannot express these.
##   * LEXICAL differences live in the catalogue — month and weekday names, "am"/"pm", and the
##     format strings themselves, which use NAMED placeholders so a locale can reorder them
##     ("{weekday}, {month} {day}" in en vs "{weekday}, {day} {month}" in es). GDScript's `%`
##     operator is positional and cannot reorder, which is why these use String.format().

## Per-locale conventions. Falls back to the reference locale for anything unlisted.
const RULES := {
	"en": {"decimal": ".", "group": ",", "clock24": false},
	"es": {"decimal": ",", "group": ".", "clock24": true},
	"pt_BR": {"decimal": ",", "group": ".", "clock24": true},
}

const FALLBACK_RULE := {"decimal": ".", "group": ",", "clock24": false}

# i18n-keys: DATE_MONTH_*, DATE_WEEKDAY_*, DATE_AM, DATE_PM
const MONTH_KEYS: PackedStringArray = [
	"DATE_MONTH_JAN",
	"DATE_MONTH_FEB",
	"DATE_MONTH_MAR",
	"DATE_MONTH_APR",
	"DATE_MONTH_MAY",
	"DATE_MONTH_JUN",
	"DATE_MONTH_JUL",
	"DATE_MONTH_AUG",
	"DATE_MONTH_SEP",
	"DATE_MONTH_OCT",
	"DATE_MONTH_NOV",
	"DATE_MONTH_DEC",
]

const WEEKDAY_KEYS: PackedStringArray = [
	"DATE_WEEKDAY_SUN",
	"DATE_WEEKDAY_MON",
	"DATE_WEEKDAY_TUE",
	"DATE_WEEKDAY_WED",
	"DATE_WEEKDAY_THU",
	"DATE_WEEKDAY_FRI",
	"DATE_WEEKDAY_SAT",
]


static func rules() -> Dictionary:
	return RULES.get(TranslationServer.get_locale(), FALLBACK_RULE)


## Abbreviated month name for a 1-based month number.
static func month_name(month: int) -> String:
	return TranslationServer.translate(MONTH_KEYS[clampi(month - 1, 0, 11)])


## Abbreviated weekday name for Godot's 0-based weekday (0 = Sunday).
static func weekday_name(weekday: int) -> String:
	return TranslationServer.translate(WEEKDAY_KEYS[clampi(weekday, 0, 6)])


## A number with the locale's decimal separator and thousands grouping.
static func number(value: float, decimals: int = 0) -> String:
	var rule: Dictionary = rules()
	var text := String.num(absf(value), decimals)
	var parts := text.split(".")
	var whole: String = parts[0]

	var grouped := ""
	var count := 0
	for i in range(whole.length() - 1, -1, -1):
		grouped = whole[i] + grouped
		count += 1
		if count % 3 == 0 and i > 0:
			grouped = rule["group"] + grouped

	if parts.size() > 1:
		grouped += rule["decimal"] + parts[1]
	return ("-" if value < 0 else "") + grouped


## Time of day, using the locale's 12- or 24-hour convention.
static func time_of_day(hour: int, minute: int) -> String:
	if rules()["clock24"]:
		return "%02d:%02d" % [hour, minute]
	var display_hour: int = 12 if (hour == 0 or hour == 12) else (hour - 12 if hour >= 12 else hour)
	var suffix := TranslationServer.translate("DATE_PM" if hour >= 12 else "DATE_AM")
	return "%02d:%02d%s" % [display_hour, minute, suffix]


## Abbreviated date, ordered by the locale rather than hard-coded month-first.
static func short_date(unix_sec: int) -> String:
	var dt: Dictionary = Time.get_datetime_dict_from_unix_time(unix_sec)
	return (
		TranslationServer
		. translate("DATE_SHORT_FORMAT")
		. format(
			{
				"weekday": weekday_name(dt.get("weekday", 0)),
				"month": month_name(dt.get("month", 1)),
				"day": dt.get("day", 1),
			}
		)
	)
