class_name RandomGeneratorUtil
extends Node

# Expanded lists of futuristic or fantastical first and last names
const FIRST_NAMES = [
	"Zara",
	"Neo",
	"Luna",
	"Orion",
	"Kael",
	"Nova",
	"Jax",
	"Aria",
	"Cael",
	"Lyra",
	"Axel",
	"Seren",
	"Thane",
	"Elara",
	"Riven",
	"Eris",
	"Drake",
	"Iris",
	"Cyrus",
	"Vega",
	"Kai",
	"Aster",
	"Rune",
	"Talia",
	"Zane",
	"Echo",
	"Blaze",
	"Sage"
]

const LAST_NAMES = [
	"Skyforge",
	"Nightwing",
	"Ironheart",
	"Frostborn",
	"Dawnblade",
	"Voidblade",
	"Galewind",
	"Wolfbane",
	"Raveneye",
	"Firebrand",
	"Stargazer",
	"Nightsky",
	"Sunflare",
	"Starfall"
]

static func generate_unique_name() -> String:
	var first = FIRST_NAMES[randi() % FIRST_NAMES.size()]
	var last = LAST_NAMES[randi() % LAST_NAMES.size()]
	return "%s %s" % [first, last]
