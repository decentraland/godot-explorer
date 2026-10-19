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
	# The guided tutorial's reward (#2767): the "Vibe Coding" emote.
	"FtueTutorial":
	{
		"campaign_id": "34923698-3107-4b0c-8968-2f0d90bcea63",
		"campaign_key":
		"tH95hT6uQCWCITnSBLljxDSSNpgxB0sMiWgvDZC86mM=.VLoVqyGRwMLooMysTEz5b1EAe173cCG5i0naKcpzdFA=",
		"urn": "urn:decentraland:matic:collections-v2:0xcb911d51f7d9c470262461935636bd173f1061f5:0",
	},
}


## The rewards host to claim against. The seeded campaigns are PRODUCTION campaigns
## (their keys only validate on rewards.decentraland.org), so we hit prod regardless of
## build environment — claiming from a dev build with a real wallet still grants the real
## wearable, which is exactly what we want to verify.
static func get_rewards_server() -> String:
	return REWARDS_SERVER_PROD
