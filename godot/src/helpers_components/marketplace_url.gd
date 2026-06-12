class_name MarketplaceUrl
extends RefCounted
## Helpers for building Decentraland web-marketplace URLs.


## Appends the mobile-IAP view flag so the web marketplace renders its
## in-app-purchase layout. Preserves any existing query string (e.g. ?section=).
static func with_mobile_iap(url: String) -> String:
	var separator := "&" if "?" in url else "?"
	return url + separator + "view=mobile-iap"
