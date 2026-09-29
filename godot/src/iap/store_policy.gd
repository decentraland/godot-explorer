class_name StorePolicy
extends RefCounted


## Single switch for every UI affordance that links out to a web purchase flow
## (Decentraland marketplace, OpenSea). Off on Android: the Google Play Payments policy
## forbids leading users to a payment method other than Play billing, and flagged
## v1.13.0 over it (#2808/#2814).
##
## Android has no IAP, so with this off it has NO in-app acquisition path at all.
## Turning it back on is a product decision, not a code cleanup.
static func can_show_external_purchase_links() -> bool:
	return not Global.is_android_or_emulating()
