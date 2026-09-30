class_name UrlHost
extends RefCounted

## Host checks for URLs that arrive as deeplink params.
##
## Only `scheme://host[:port][/path]` is accepted: a plain hostname or an IP
## literal, no userinfo, query or fragment. Anything looser returns false.

const DECENTRALAND_DOMAINS: Array[String] = [
	"decentraland.org", "decentraland.zone", "decentraland.today", "dclregenesislabs.xyz"
]

static var _url_regex: RegEx = RegEx.create_from_string(
	"^(wss?|https?)://([a-z0-9.-]+|\\[[0-9a-f:]+\\])(:[0-9]{1,5})?(/[a-z0-9._~%!$&'()*+,;=:/-]*)?$"
)


## True for loopback, private IPv4 (10/8, 172.16/12, 192.168/16), `localhost`
## and `*.local` hosts.
static func is_local_network(url: String) -> bool:
	var host := _host(url)
	if host == "localhost" or host == "::1" or host.ends_with(".local"):
		return true
	var octets := host.split(".")
	if octets.size() != 4:
		return false
	for o in octets:
		if not o.is_valid_int() or int(o) < 0 or int(o) > 255:
			return false
	var a := int(octets[0])
	var b := int(octets[1])
	return a == 127 or a == 10 or (a == 192 and b == 168) or (a == 172 and b >= 16 and b <= 31)


## True for https URLs on a Decentraland domain or one of its subdomains.
static func is_decentraland_https(url: String) -> bool:
	if not url.to_lower().begins_with("https://"):
		return false
	var host := _host(url)
	for domain in DECENTRALAND_DOMAINS:
		if host == domain or host.ends_with("." + domain):
			return true
	return false


## Lower-cased host (IPv6 brackets stripped), or "" when `url` does not match.
static func _host(url: String) -> String:
	var m := _url_regex.search(url.to_lower())
	if m == null:
		return ""
	return m.get_string(2).trim_prefix("[").trim_suffix("]")
