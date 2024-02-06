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
	"Zephyr",
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
	"Sage",
	"Aurora"
]
const LAST_NAMES = [
	"Starwalker",
	"Voidseeker",
	"Skyforge",
	"Darkweaver",
	"Lightbringer",
	"Moonshadow",
	"Stormrider",
	"Sunwhisper",
	"Flameheart",
	"Nightwing",
	"Ironheart",
	"Frostborn",
	"Dawnblade",
	"Shadowmere",
	"Starshield",
	"Voidblade",
	"Galewind",
	"Mystweaver",
	"Skybreaker",
	"Dreamseeker",
	"Wolfbane",
	"Raveneye",
	"Thunderstrike",
	"Soulkeeper",
	"Firebrand",
	"Stargazer",
	"Nightsky",
	"Sunflare",
	"Voidwalker",
	"Starfall"
]


static func generate_unique_name() -> String:
	var first = FIRST_NAMES[randi() % FIRST_NAMES.size()]
	var last = LAST_NAMES[randi() % LAST_NAMES.size()]
	return "%s %s" % [first, last]
