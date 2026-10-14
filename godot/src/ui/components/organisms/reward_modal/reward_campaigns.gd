class_name RewardCampaigns
extends RefCounted

## Native reward-claim campaign table, the client-side mirror of Genesis Plaza's
## `central-plaza/src/modules/dispenser/claiming/claimConfig.ts`.
##
## Each entry describes a Decentraland Rewards campaign the client can claim
## natively through the reward modal (see reward_modal.gd), producing the same
## effect as tapping the in-scene dispenser: a signed POST to the rewards server
## that assigns the wearable to the player's wallet.

const REWARDS_SERVER_PROD := "https://rewards.decentraland.org"

## Keyed by a short reference used as the `?claim=<ref>` deeplink value.
## - campaign_id / campaign_key: the signed campaign credentials (from the rewards dashboard).
## - urn: OPTIONAL wearable urn for the pre-claim preview. When empty the modal shows its
##   placeholder art and reveals the real item from the claim response after a successful claim.
const CAMPAIGNS := {
	"MobilePet":
	{
		"campaign_id": "fcfd9412-38f7-4b44-92f6-17a0f10ae53b",
		"campaign_key":
		"Vn0WimL5RjKMpIeDoJIvHfz9lBI490tEkvYXoPEK5Ts=.3QEbxeMg9QXkY/vZrcZyDUYNL0MFrHFatF/KZJ1g7WA=",
		"urn": "",
	},
	# TODO(#3018): login-streak D3 item — not published yet, blocked on content. Empty
	# campaign_id/campaign_key surface reward_modal's existing "out of stock" claim error
	# until the real campaign exists; don't fill these in with placeholder credentials. `urn` is
	# ONLY a preview hint (no claim risk), so it's mocked with a real published on-chain wearable
	# (decentraland.org/marketplace/contracts/0x996b51131698ba70dcfb3fb5956e6816c5778eda/items/0)
	# to exercise the thumbnail-fetch path — swap for the real item's urn once it's published.
	"LoginStreakD3":
	{
		"campaign_id": "",
		"campaign_key": "",
		"urn": "urn:decentraland:matic:collections-v2:0x996b51131698ba70dcfb3fb5956e6816c5778eda:0",
	},
	# TODO(#3018): login-streak D7 reward ("Corona de Fuego" is a placeholder name) — same as
	# above; `urn` mocked with the same item for now (no second reference item on hand yet).
	"LoginStreakD7":
	{
		"campaign_id": "",
		"campaign_key": "",
		"urn": "urn:decentraland:matic:collections-v2:0x996b51131698ba70dcfb3fb5956e6816c5778eda:0",
	},
}


## The rewards host to claim against. The seeded campaigns are PRODUCTION campaigns
## (their keys only validate on rewards.decentraland.org), so we hit prod regardless of
## build environment — claiming from a dev build with a real wallet still grants the real
## wearable, which is exactly what we want to verify.
static func get_rewards_server() -> String:
	return REWARDS_SERVER_PROD
