class_name Places
extends Node

const PinCategories = {
	"FAVORITES": "favorites",
	"ART": "art",
	"CRYPTO": "crypto",
	"SOCIAL": "social",
	"GAME": "games",
	"SHOP": "shop",
	"EDUCATION": "education",
	"MUSIC": "music",
	"FASHION": "fashion",
	"CASINO": "casino",
	"SPORTS": "sports",
	"BUSINESS": "business",
	"POI": "poi"
}

class Categories:
	const ALL: String = "all"
	const FAVORITES: String = "favorites"
	const ART: String = "art"
	const CRYPTO: String = "crypto"
	const SOCIAL: String = "social"
	const GAMES: String = "games"
	const SHOP: String = "shop"
	const EDUCATION: String = "education"
	const MUSIC: String = "music"
	const FASHION: String = "fashion"
	const CASINO: String = "casino"
	const SPORTS: String = "sports"
	const BUSINESS: String = "business"
	const POI: String = "poi"

	
	const ALL_CATEGORIES: PackedStringArray = [
		ALL,
		FAVORITES,
		ART,
		CRYPTO,
		SOCIAL,
		GAMES,
		SHOP,
		EDUCATION,
		MUSIC,
		FASHION,
		CASINO,
		SPORTS,
		BUSINESS,
		POI
	]
