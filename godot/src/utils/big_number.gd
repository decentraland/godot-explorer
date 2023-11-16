class_name BigNumber
extends RefCounted
# Big number class for use in idle / incremental games and other games that needs very large numbers
# Can format large numbers using a variety of notation methods:
# AA notation like AA, AB, AC etc.
# Metric symbol notation k, m, G, T etc.
# Metric name notation kilo, mega, giga, tera etc.
# Long names like octo-vigin-tillion or millia-nongen-quin-vigin-tillion (based on work by Landon Curt Noll)
# Scientic notation like 13e37 or 42e42
# Long strings like 4200000000 or 13370000000000000000000000000000
# Please note that this class has limited precision and does not fully support negative exponents
# Integer division warnings should be disabled in Project Settings

var mantissa: float = 0.0
var exponent: int = 1

const postfixes_metric_symbol = {
	"0": "",
	"1": "k",
	"2": "M",
	"3": "G",
	"4": "T",
	"5": "P",
	"6": "E",
	"7": "Z",
	"8": "Y",
	"9": "R",
	"10": "Q"
}
const postfixes_metric_name = {
	"0": "",
	"1": "kilo",
	"2": "mega",
	"3": "giga",
	"4": "tera",
	"5": "peta",
	"6": "exa",
	"7": "zetta",
	"8": "yotta",
	"9": "ronna",
	"10": "quetta"
}
static var postfixes_aa = {
	"0": "",
	"1": "k",
	"2": "m",
	"3": "b",
	"4": "t",
	"5": "aa",
	"6": "ab",
	"7": "ac",
	"8": "ad",
	"9": "ae",
	"10": "af",
	"11": "ag",
	"12": "ah",
	"13": "ai",
	"14": "aj",
	"15": "ak",
	"16": "al",
	"17": "am",
	"18": "an",
	"19": "ao",
	"20": "ap",
	"21": "aq",
	"22": "ar",
	"23": "as",
	"24": "at",
	"25": "au",
	"26": "av",
	"27": "aw",
	"28": "ax",
	"29": "ay",
	"30": "az",
	"31": "ba",
	"32": "bb",
	"33": "bc",
	"34": "bd",
	"35": "be",
	"36": "bf",
	"37": "bg",
	"38": "bh",
	"39": "bi",
	"40": "bj",
	"41": "bk",
	"42": "bl",
	"43": "bm",
	"44": "bn",
	"45": "bo",
	"46": "bp",
	"47": "bq",
	"48": "br",
	"49": "bs",
	"50": "bt",
	"51": "bu",
	"52": "bv",
	"53": "bw",
	"54": "bx",
	"55": "by",
	"56": "bz",
	"57": "ca"
}
const alphabet_aa: Array[String] = [
	"a",
	"b",
	"c",
	"d",
	"e",
	"f",
	"g",
	"h",
	"i",
	"j",
	"k",
	"l",
	"m",
	"n",
	"o",
	"p",
	"q",
	"r",
	"s",
	"t",
	"u",
	"v",
	"w",
	"x",
	"y",
	"z"
]

const latin_ones: Array[String] = [
	"", "un", "duo", "tre", "quattuor", "quin", "sex", "septen", "octo", "novem"
]
const latin_tens: Array[String] = [
	"",
	"dec",
	"vigin",
	"trigin",
	"quadragin",
	"quinquagin",
	"sexagin",
	"septuagin",
	"octogin",
	"nonagin"
]
const latin_hundreds: Array[String] = [
	"",
	"cen",
	"duocen",
	"trecen",
	"quadringen",
	"quingen",
	"sescen",
	"septingen",
	"octingen",
	"nongen"
]
const latin_special: Array[String] = [
	"", "mi", "bi", "tri", "quadri", "quin", "sex", "sept", "oct", "non"
]

static var other = {
	"dynamic_decimals": true,
	"dynamic_numbers": 4,
	"small_decimals": 2,
	"thousand_decimals": 2,
	"big_decimals": 2,
	"scientific_decimals": 2,
	"logarithmic_decimals": 2,
	"thousand_separator": ".",
	"decimal_separator": ",",
	"postfix_separator": "",
	"reading_separator": "",
	"thousand_name": "thousand"
}

const MAX_MANTISSA = 1209600.0
const MANTISSA_PRECISSION = 0.0000001

const MIN_INTEGER: int = -9223372036854775807
const MAX_INTEGER: int = 9223372036854775806


# Recursive function to convert a string to scientific notation
# Only for large numbers with zeros on the right
static func to_scientific_notation(number_str: String, exponent: int = 0) -> String:
	# Check if the string is a valid number
	if number_str.is_empty():
		print("Invalid input")
		return "0"

	# Check if the number ends with "000" and is large enough
	if number_str.ends_with("000") and int(number_str) > 1000:
		# Remove the last three zeros and increase the exponent
		return to_scientific_notation(number_str.substr(0, number_str.length() - 3), exponent + 3)
	else:
		# If the exponent is greater than zero, convert to scientific notation
		if exponent > 0:
			return number_str + "e" + str(exponent)
		else:
			return number_str


func _init(m, e := 0):
	if m is String:
		if not m.contains("e"):
			m = to_scientific_notation(m)

		var scientific = m.split("e")
		mantissa = float(scientific[0])
		if scientific.size() > 1:
			exponent = int(scientific[1])
		else:
			exponent = 0
	elif m is BigNumber:
		mantissa = m.mantissa
		exponent = m.exponent
	else:
		_size_check(m)
		mantissa = m
		exponent = e
	calculate(self)
	pass


func _size_check(m):
	if m > MAX_MANTISSA:
		printerr("BIG ERROR: MANTISSA TOO LARGE, PLEASE USE EXPONENT OR SCIENTIFIC NOTATION")


static func _type_check(n):
	if typeof(n) == TYPE_INT or typeof(n) == TYPE_FLOAT:
		return {"mantissa": float(n), "exponent": int(0)}
	elif typeof(n) == TYPE_STRING:
		var split = n.split("e")
		return {"mantissa": float(split[0]), "exponent": int(0 if split.size() == 1 else split[1])}
	else:
		return n


func plus(n) -> BigNumber:
	n = _type_check(n)
	_size_check(n.mantissa)
	var exp_diff = n.exponent - exponent
	if exp_diff < 248:
		var scaled_mantissa = n.mantissa * pow(10, exp_diff)
		mantissa += scaled_mantissa
	elif is_less_than(n):
		mantissa = n.mantissa  #when difference between values is big, throw away small number
		exponent = n.exponent
	calculate(self)
	return self


func minus(n) -> BigNumber:
	n = _type_check(n)
	_size_check(n.mantissa)
	var exp_diff = n.exponent - exponent  #abs?
	if exp_diff < 248:
		var scaled_mantissa = n.mantissa * pow(10, exp_diff)
		mantissa -= scaled_mantissa
	elif is_less_than(n):
		mantissa = -MANTISSA_PRECISSION
		exponent = n.exponent
	calculate(self)
	return self


func multiply(n) -> BigNumber:
	n = _type_check(n)
	_size_check(n.mantissa)
	var new_exponent = n.exponent + exponent
	var new_mantissa = n.mantissa * mantissa
	while new_mantissa >= 10.0:
		new_mantissa /= 10.0
		new_exponent += 1
	mantissa = new_mantissa
	exponent = new_exponent
	calculate(self)
	return self


func divide(n) -> BigNumber:
	n = _type_check(n)
	_size_check(n.mantissa)
	if n.mantissa == 0:
		printerr("BIG ERROR: DIVIDE BY ZERO OR LESS THAN " + str(MANTISSA_PRECISSION))
		return self
	var new_exponent = exponent - n.exponent
	var new_mantissa = mantissa / n.mantissa
	while new_mantissa < 1.0 and new_mantissa > 0.0:
		new_mantissa *= 10.0
		new_exponent -= 1
	mantissa = new_mantissa
	exponent = new_exponent
	calculate(self)
	return self


func power_int(n: int):
	if n < 0:
		printerr("BIG ERROR: NEGATIVE EXPONENTS NOT SUPPORTED!")
		mantissa = 1.0
		exponent = 0
		return self
	if n == 0:
		mantissa = 1.0
		exponent = 0
		return self

	var y_mantissa = 1
	var y_exponent = 0

	while n > 1:
		calculate(self)
		if n % 2 == 0:  #n is even
			exponent = exponent + exponent
			mantissa = mantissa * mantissa
			n = n / 2  # warning-ignore:integer_division
		else:
			y_mantissa = mantissa * y_mantissa
			y_exponent = exponent + y_exponent
			exponent = exponent + exponent
			mantissa = mantissa * mantissa
			n = (n - 1) / 2  # warning-ignore:integer_division

	exponent = y_exponent + exponent
	mantissa = y_mantissa * mantissa
	calculate(self)
	return self


func power(n: float) -> BigNumber:
	if mantissa == 0:
		return self

	# fast track
	var temp: float = exponent * n
	if round(n) == n and temp < MAX_INTEGER and temp > MIN_INTEGER and temp != INF and temp != -INF:
		var newMantissa = pow(mantissa, n)
		if newMantissa != INF and newMantissa != -INF:
			mantissa = newMantissa
			exponent = int(temp)
			calculate(self)
			return self

	# a bit slower, still supports floats
	var newExponent: int = int(temp)
	var residue: float = temp - newExponent
	var newMantissa = pow(10, n * log10(mantissa) + residue)
	if newMantissa != INF and newMantissa != -INF:
		mantissa = newMantissa
		exponent = newExponent
		calculate(self)
		return self

	if round(n) != n:
		printerr("BIG ERROR: POWER FUNCTION DOES NOT SUPPORT LARGE FLOATS, USE INTEGERS!")

	return power_int(int(n))


func square_root() -> BigNumber:
	if exponent % 2 == 0:
		mantissa = sqrt(mantissa)
		exponent = exponent / 2
	else:
		mantissa = sqrt(mantissa * 10)
		exponent = (exponent - 1) / 2
	calculate(self)
	return self


func modulo(n) -> BigNumber:
	n = _type_check(n)
	_size_check(n.mantissa)
	var big = {"mantissa": mantissa, "exponent": exponent}
	divide(n)
	round_down()
	multiply(n)
	minus(big)
	mantissa = abs(mantissa)
	return self


func calculate(big):
	if big.mantissa >= 10.0 or big.mantissa < 1.0:
		var diff = int(floor(log10(big.mantissa)))
		if diff > -10 and diff < 248:
			var div = pow(10, diff)
			if div > MANTISSA_PRECISSION:
				big.mantissa /= div
				big.exponent += diff
	while big.exponent < 0:
		big.mantissa *= 0.1
		big.exponent += 1
	while big.mantissa >= 10.0:
		big.mantissa *= 0.1
		big.exponent += 1
	if big.mantissa == 0:
		big.mantissa = 0.0
		big.exponent = 0
	big.mantissa = snapped(big.mantissa, MANTISSA_PRECISSION)
	pass


func is_equal_to(n) -> bool:
	n = _type_check(n)
	calculate(n)
	return n.exponent == exponent and is_equal_approx(n.mantissa, mantissa)


func is_larger_than(n) -> bool:
	return !is_less_than_or_equal_to(n)


func is_larger_than_or_equal_to(n) -> bool:
	return !is_less_than(n)


func is_less_than(n) -> bool:
	n = _type_check(n)
	calculate(n)
	if (
		mantissa == 0
		and (n.mantissa > MANTISSA_PRECISSION or mantissa < MANTISSA_PRECISSION)
		and n.mantissa == 0
	):
		return false
	if exponent < n.exponent:
		return true
	elif exponent == n.exponent:
		if mantissa < n.mantissa:
			return true
		else:
			return false
	else:
		return false


func is_less_than_or_equal_to(n) -> bool:
	n = _type_check(n)
	calculate(n)
	if is_less_than(n):
		return true
	if n.exponent == exponent and is_equal_approx(n.mantissa, mantissa):
		return true
	return false


static func min_value(m, n) -> BigNumber:
	m = _type_check(m)
	if m.is_less_than(n):
		return m
	else:
		return n


static func max_value(m, n) -> BigNumber:
	m = _type_check(m)
	if m.is_larger_than(n):
		return m
	else:
		return n


static func abs_value(n) -> BigNumber:
	n.mantissa = abs(n.mantissa)
	return n


func round_down() -> BigNumber:
	if exponent == 0:
		mantissa = floor(mantissa)
		return self
	else:
		var precision = 1.0
		for i in range(min(8, exponent)):
			precision /= 10.0
		if precision < MANTISSA_PRECISSION:
			precision = MANTISSA_PRECISSION
		mantissa = snapped(mantissa, precision)
		return self


func log10(x):
	return log(x) * 0.4342944819032518


func abs_log_10():
	return exponent + log10(abs(mantissa))


func ln():
	return 2.302585092994045 * log_n(10)


func log_n(base):
	return (2.302585092994046 / log(base)) * (exponent + log10(mantissa))


func pow10(value: int):
	mantissa = pow(10, value % 1)
	exponent = int(value)


func _to_string() -> String:
	var mantissa_decimals = 0
	if str(mantissa).find(".") >= 0:
		mantissa_decimals = str(mantissa).split(".")[1].length()
	if mantissa_decimals > exponent:
		if exponent < 248:
			return str(mantissa * pow(10, exponent))
		else:
			return to_plain_scientific()
	else:
		var mantissa_string = str(mantissa).replace(".", "")
		for _i in range(exponent - mantissa_decimals):
			mantissa_string += "0"
		return mantissa_string


func to_plain_scientific():
	return str(mantissa) + "e" + str(exponent)


func to_scientific(no_decimals_on_small_values = false, force_decimals = false):
	if exponent < 3:
		var decimal_increments: float = 1 / (pow(10, other.scientific_decimals) / 10)
		var value = str(snappedf(mantissa * pow(10, exponent), decimal_increments))
		var split = value.split(".")
		if no_decimals_on_small_values:
			return split[0]
		if split.size() > 1:
			for i in range(other.logarithmic_decimals):
				if split[1].length() < other.scientific_decimals:
					split[1] += "0"
			return (
				split[0]
				+ other.decimal_separator
				+ split[1].substr(
					0,
					min(
						other.scientific_decimals,
						(
							other.dynamic_numbers - split[0].length()
							if other.dynamic_decimals
							else other.scientific_decimals
						)
					)
				)
			)
		else:
			return value
	else:
		var split = str(mantissa).split(".")
		if split.size() == 1:
			split.append("")
		if force_decimals:
			for i in range(other.scientific_decimals):
				if split[1].length() < other.scientific_decimals:
					split[1] += "0"
		return (
			split[0]
			+ other.decimal_separator
			+ split[1].substr(
				0,
				min(
					other.scientific_decimals,
					(
						other.dynamic_numbers - 1 - str(exponent).length()
						if other.dynamic_decimals
						else other.scientific_decimals
					)
				)
			)
			+ "e"
			+ str(exponent)
		)


func to_logarithmic(no_decimals_on_small_values = false) -> String:
	var decimal_increments: float = 1 / (pow(10, other.logarithmic_decimals) / 10)
	if exponent < 3:
		var value = str(snappedf(mantissa * pow(10, exponent), decimal_increments))
		var split = value.split(".")
		if no_decimals_on_small_values:
			return split[0]
		if split.size() > 1:
			for i in range(other.logarithmic_decimals):
				if split[1].length() < other.logarithmic_decimals:
					split[1] += "0"
			return (
				split[0]
				+ other.decimal_separator
				+ split[1].substr(
					0,
					min(
						other.logarithmic_decimals,
						(
							other.dynamic_numbers - split[0].length()
							if other.dynamic_decimals
							else other.logarithmic_decimals
						)
					)
				)
			)
		else:
			return value
	var dec = str(snappedf(abs(log(mantissa) / log(10) * 10), decimal_increments))
	dec = dec.replace(".", "")
	for i in range(other.logarithmic_decimals):
		if dec.length() < other.logarithmic_decimals:
			dec += "0"
	var formated_exponent = format_exponent(exponent)
	dec = dec.substr(
		0,
		min(
			other.logarithmic_decimals,
			(
				other.dynamic_numbers - formated_exponent.length()
				if other.dynamic_decimals
				else other.logarithmic_decimals
			)
		)
	)
	return "e" + formated_exponent + other.decimal_separator + dec


func format_exponent(value) -> String:
	if value < 1000:
		return str(value)
	var string = str(value)
	var mod = string.length() % 3
	var output = ""
	for i in range(0, string.length()):
		if i != 0 and i % 3 == mod:
			output += other.thousand_separator
		output += string[i]
	return output


func to_float() -> float:
	return snappedf(float(str(mantissa) + "e" + str(exponent)), 0.01)


func to_prefix(
	no_decimals_on_small_values = false,
	use_thousand_symbol = true,
	force_decimals = true,
	scientic_prefix = false
) -> String:
	var number: float = mantissa
	if not scientic_prefix:
		var hundreds = 1
		for _i in range(exponent % 3):
			hundreds *= 10
		number *= hundreds

	var split = str(number).split(".")
	if split.size() == 1:
		split.append("")
	if force_decimals:
		var max_decimals = max(
			max(other.small_decimals, other.thousand_decimals), other.big_decimals
		)
		for i in range(max_decimals):
			if split[1].length() < max_decimals:
				split[1] += "0"

	if no_decimals_on_small_values and exponent < 3:
		return split[0]
	elif exponent < 3:
		if other.small_decimals == 0 or split[1] == "":
			return split[0]
		else:
			return (
				split[0]
				+ other.decimal_separator
				+ split[1].substr(
					0,
					min(
						other.small_decimals,
						(
							other.dynamic_numbers - split[0].length()
							if other.dynamic_decimals
							else other.small_decimals
						)
					)
				)
			)
	elif exponent < 6:
		if other.thousand_decimals == 0 or (split[1] == "" and use_thousand_symbol):
			return split[0]
		else:
			if use_thousand_symbol:  # when the prefix is supposed to be using with a K for thousand
				for i in range(3):
					if split[1].length() < 3:
						split[1] += "0"
				return (
					split[0]
					+ other.decimal_separator
					+ split[1].substr(
						0,
						min(
							3,
							(
								other.dynamic_numbers - split[0].length()
								if other.dynamic_decimals
								else 3
							)
						)
					)
				)
			else:
				for i in range(3):
					if split[1].length() < 3:
						split[1] += "0"
				return split[0] + other.thousand_separator + split[1].substr(0, 3)
	else:
		if other.big_decimals == 0 or split[1] == "":
			return split[0]
		else:
			return (
				split[0]
				+ other.decimal_separator
				+ split[1].substr(
					0,
					min(
						other.big_decimals,
						(
							other.dynamic_numbers - split[0].length()
							if other.dynamic_decimals
							else other.big_decimals
						)
					)
				)
			)


func _latin_power(european_system) -> int:
	if european_system:
		return int(exponent / 3) / 2
	return int(exponent / 3) - 1


func _latin_prefix(european_system) -> String:
	var ones = _latin_power(european_system) % 10
	var tens = int(_latin_power(european_system) / 10) % 10
	var hundreds = int(_latin_power(european_system) / 100) % 10
	var millias = int(_latin_power(european_system) / 1000) % 10

	var prefix = ""
	if _latin_power(european_system) < 10:
		prefix = (
			latin_special[ones]
			+ other.reading_separator
			+ latin_tens[tens]
			+ other.reading_separator
			+ latin_hundreds[hundreds]
		)
	else:
		prefix = (
			latin_hundreds[hundreds]
			+ other.reading_separator
			+ latin_ones[ones]
			+ other.reading_separator
			+ latin_tens[tens]
		)

	for _i in range(millias):
		prefix = "millia" + other.reading_separator + prefix

	return prefix.lstrip(other.reading_separator).rstrip(other.reading_separator)


func _tillion_or_illion(european_system) -> String:
	if exponent < 6:
		return ""
	var powerKilo = _latin_power(european_system) % 1000
	if powerKilo < 5 and powerKilo > 0 and _latin_power(european_system) < 1000:
		return ""
	if powerKilo >= 7 and powerKilo <= 10 or int(powerKilo / 10) % 10 == 1:
		return "i"
	return "ti"


func _llion_or_lliard(european_system) -> String:
	if exponent < 6:
		return ""
	if int(exponent / 3) % 2 == 1 and european_system:
		return "lliard"
	return "llion"


func get_long_name(european_system = false, prefix = "") -> String:
	if exponent < 6:
		return ""
	else:
		return (
			prefix
			+ _latin_prefix(european_system)
			+ other.reading_separator
			+ _tillion_or_illion(european_system)
			+ _llion_or_lliard(european_system)
		)


func to_american_name(no_decimals_on_small_values = false) -> String:
	return to_long_name(no_decimals_on_small_values, false)


func to_european_name(no_decimals_on_small_values = false) -> String:
	return to_long_name(no_decimals_on_small_values, true)


func to_long_name(no_decimals_on_small_values = false, european_system = false) -> String:
	if exponent < 6:
		if exponent > 2:
			return (
				to_prefix(no_decimals_on_small_values)
				+ other.postfix_separator
				+ other.thousand_name
			)
		else:
			return to_prefix(no_decimals_on_small_values)

	var postfix = (
		_latin_prefix(european_system)
		+ other.reading_separator
		+ _tillion_or_illion(european_system)
		+ _llion_or_lliard(european_system)
	)

	return to_prefix(no_decimals_on_small_values) + other.postfix_separator + postfix


func to_metric_symbol(no_decimals_on_small_values = false) -> String:
	var target = int(exponent / 3)

	if not postfixes_metric_symbol.has(str(target)):
		return to_scientific()
	else:
		return (
			to_prefix(no_decimals_on_small_values)
			+ other.postfix_separator
			+ postfixes_metric_symbol[str(target)]
		)


func to_metric_name(no_decimals_on_small_values = false) -> String:
	var target = int(exponent / 3)

	if not postfixes_metric_name.has(str(target)):
		return to_scientific()
	else:
		return (
			to_prefix(no_decimals_on_small_values)
			+ other.postfix_separator
			+ postfixes_metric_name[str(target)]
		)


func to_aa(
	no_decimals_on_small_values = false, use_thousand_symbol = true, force_decimals = false
) -> String:
	var target: int = int(exponent / 3)
	var aa_index: String = str(target)
	var postfix: String = ""

	if not postfixes_aa.has(aa_index):
		var offset: int = target + 22
		var base: int = alphabet_aa.size()
		while offset > 0:
			offset -= 1
			var digit: int = offset % base
			postfix = alphabet_aa[digit] + postfix
			offset /= base
		postfixes_aa[aa_index] = postfix
	else:
		postfix = postfixes_aa[aa_index]

	if not use_thousand_symbol and target == 1:
		postfix = ""

	var prefix = to_prefix(no_decimals_on_small_values, use_thousand_symbol, force_decimals)

	return prefix + other.postfix_separator + postfix
