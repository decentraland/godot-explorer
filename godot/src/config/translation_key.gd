class_name TranslationKey
extends RefCounted
## A translation key, as a type rather than a bare [String].
##
## Display APIs that take a [String] are ambiguous: some want a key (their node
## auto-translates), others want finished text (their node has
## [code]auto_translate_mode = 2[/code]). Nothing in the signature says which, and
## guessing wrong fails silently in both directions — that is how an entire modal
## system, an OTP flow and nine toasts shipped in English while the key checker
## reported a clean tree.
##
## Typing the parameter states the contract and lets the engine enforce it:
## [codeblock]
## func set_body(body: TranslationKey) -> void    # give me a key, I translate it
## func show_toast(text: String) -> void          # give me finished text or data
## [/codeblock]
## Passing a literal to the first is a [b]parse error[/b], caught by
## [code]cargo run -- check-gdscript[/code] before it can run.
##
## The other half of the contract is that keys are never glued to literals.
## [code]set_body(SOME_KEY + " Try again.")[/code] produces a string matching no
## key, so the lookup misses and the raw key is drawn on screen. Pass the pieces
## and let [method join] assemble them, so each is translated on its own and a
## translator can rewrite one paragraph without touching the others.

## Paragraph break used by [method join].
const PARAGRAPH_SEPARATOR := "\n\n"

## Suffix Godot's CSV importer never registers as a message of its own: a plural form
## lives in the `?plural` column of its singular, reachable only through
## get_plural_message(). The catalogue names them `<KEY>_PLURAL` (enforced by
## format_csv.py), so the singular answers for them in [method is_known].
const PLURAL_SUFFIX := "_PLURAL"

## SCREAMING_SNAKE_CASE. The underscore is optional so single-word keys like
## COMMON_NEARBY's short cousins (NEARBY) are accepted.
const KEY_PATTERN := "^[A-Z][A-Z0-9]*(_[A-Z0-9]+)*$"

## The key itself. Read it through [method raw] rather than reaching in.
var key: String

static var _key_regex: RegEx = RegEx.create_from_string(KEY_PATTERN)


func _init(new_key: String) -> void:
	key = new_key
	# The type stops a String reaching a TranslationKey *parameter*, but
	# TranslationKey.new("Try again.") is still legal, so the shape is checked here.
	# Debug-only: this is a development guard, not a runtime cost.
	if OS.is_debug_build() and not new_key.is_empty() and not is_valid():
		push_error("TranslationKey expects a key, got rendered text: %s" % new_key)


## The raw key, for assignment to a node that auto-translates. Prefer this over
## [method text] there: the node re-translates itself on a language change,
## whereas resolved text would freeze in the old locale.
func raw() -> String:
	return key


## The translated string, for a node with [code]auto_translate_mode = 2[/code] and
## for anywhere text is composed before display.
func text() -> String:
	# tr() is an Object method, so an instance can call it even though a static
	# func cannot — that is why resolution lives here rather than in a helper.
	return tr(key)


## Translated, then filled by name.
##
## Named only: positional [code]%s[/code] pins the word order to English, and word order is the
## first thing a translation changes. "I like %s %s" cannot become "las flores rojas" — the
## adjective has to move. Named fields let the translator put them where the sentence needs
## them, and format_csv.py rejects a [code]%[/code] specifier in the catalogue for that reason.
##
## Godot's [method String.format] substitutes text and nothing else: it has no width, precision
## or padding syntax, and [code]{n:.2f}[/code] is emitted verbatim rather than applied. Anything
## that needs rendering — decimals, grouping, zero padding — is rendered by the caller first,
## usually through [LocaleFormat].
## [codeblock]
## key.format({"seconds": "%02d" % secs, "size": LocaleFormat.number(mb, 1)})
## [/codeblock]
## A field the string does not use is ignored; a field the string uses but the caller omits is
## drawn as a literal [code]{name}[/code], which is why the parity check exists.
func format(values: Dictionary) -> String:
	return tr(key).format(values)


## The plural form for [param count], [b]unsubstituted[/b] — fill it with [method String.format].
##
## Selecting the form and filling the placeholders are separate steps on purpose. Doing both at
## once forced the count to be the substituted value too, so any number that needed
## locale-aware formatting had to be smuggled in through a second parameter.
## [codeblock]
## TranslationKey.new("SOCIAL_MUTUAL_FRIENDS").plural(n).format({"friends": LocaleFormat.number(n)})
## [/codeblock]
## [param count] selects the grammatical form; what the reader sees is whatever the caller puts
## in [code]{count}[/code], so the two can differ without a special case.
##
## The companion key is derived rather than passed: format_csv.py enforces that a plural entry
## is named [code]<KEY>_PLURAL[/code], so there is nothing for a caller to get wrong.
##
## Note [code]auto_translate[/code] calls [code]tr()[/code] and never [code]tr_n()[/code],
## so a plural string can only ever be set from code — never left to a scene default.
func plural(count: int) -> String:
	return tr_n(key, key + PLURAL_SUFFIX, count)


## Translated and upper-cased, for the labels this project renders in caps.
func upper() -> String:
	return tr(key).to_upper()


## Whether this instance's key is shaped like a key rather than prose.
func is_valid() -> bool:
	return is_key(key)


## Debug aid. Deliberately not the bare key: an accidental "%s" % some_key should
## look wrong in a log rather than quietly produce something plausible.
func _to_string() -> String:
	return 'TranslationKey("%s")' % key


## Whether [param value] is shaped like a key rather than prose. Static so callers
## (and tests) can check a string without constructing one, which would trip the
## guard in [method _init].
static func is_key(value: String) -> bool:
	return _key_regex.search(value) != null


## Whether this key is actually in the shipped catalogue.
func exists() -> bool:
	return is_known(key)


## Whether [param value] resolves against the fallback catalogue.
##
## Deliberately checks the **fallback** locale, not the current one: on a device set
## to es-AR, get_translation_object("es_AR") is null while every key still resolves
## through the English table, so asking about the current locale reports everything
## missing. get_message() is used rather than tr() because tr() returns the key
## itself on a miss and so can never say "absent".
##
## Returns true when no catalogue is loaded at all — that means "cannot tell", and a
## check that cries wolf before translations register is worse than no check.
static func is_known(value: String) -> bool:
	var fallback := str(ProjectSettings.get_setting("internationalization/locale/fallback", "en"))
	var catalogue: Translation = TranslationServer.get_translation_object(fallback)
	if catalogue == null:
		return true
	if not str(catalogue.get_message(value)).is_empty():
		return true
	if value.ends_with(PLURAL_SUFFIX):
		var singular := value.left(value.length() - PLURAL_SUFFIX.length())
		return not str(catalogue.get_message(singular)).is_empty()
	return false


## Wraps a list of raw keys, for the ordered tables this project keeps (month names,
## weekday names, tip rotations) where writing each one out would be noise.
static func many(raw_keys: PackedStringArray) -> Array[TranslationKey]:
	var wrapped: Array[TranslationKey] = []
	for raw_key in raw_keys:
		wrapped.append(TranslationKey.new(raw_key))
	return wrapped


## Translates each key and joins the results, for text built from several catalogue
## entries. Static, but it delegates to [method text], so translation still goes
## through [code]tr()[/code].
static func join(keys: Array[TranslationKey], separator := PARAGRAPH_SEPARATOR) -> String:
	var parts := PackedStringArray()
	for entry in keys:
		parts.append(entry.text())
	return separator.join(parts)
