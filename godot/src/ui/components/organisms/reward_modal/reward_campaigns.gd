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
const REWARDS_SERVER_ZONE := "https://rewards.decentraland.zone"

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
}


## The rewards host to claim against. The seeded campaigns are PRODUCTION campaigns
## (their keys only validate on rewards.decentraland.org), so we hit prod regardless of
## build environment — claiming from a dev build with a real wallet still grants the real
## wearable, which is exactly what we want to verify. REWARDS_SERVER_ZONE is kept for
## future test campaigns.
static func get_rewards_server() -> String:
	return REWARDS_SERVER_PROD
