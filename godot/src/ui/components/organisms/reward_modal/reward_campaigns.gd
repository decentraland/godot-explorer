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
	# login-streak D3 reward, "Mobile Pants" (decentraland.org/rewards/campaign/?id=4237615f-99b6-
	# 46a0-8816-b2073a4f2780), item contract 0xca1e4214acd9ba53bda189639878214ecff4de5f.
	"LoginStreakD3":
	{
		"campaign_id": "4237615f-99b6-46a0-8816-b2073a4f2780",
		"campaign_key":
		"g8k3ewmNR5mxNWqW2ELAQkI3YV+ZtkagiBayBzpPJ4A=.Yp941hk9ViLVjnIhQ+lsI9Ro2Z4qCZwOFC1LQzKz1rw=",
		"urn": "urn:decentraland:matic:collections-v2:0xca1e4214acd9ba53bda189639878214ecff4de5f:0",
	},
	# login-streak D7 reward, "Mobile Jacket" (decentraland.org/rewards/campaign/?id=ec79d7a9-566e-
	# 4acc-8c48-40bf5390788b), item contract 0x914c9cb1e619f24734b61c345108fd69401f96ec.
	"LoginStreakD7":
	{
		"campaign_id": "ec79d7a9-566e-4acc-8c48-40bf5390788b",
		"campaign_key":
		"vseXLBe9SjyLN5QqHgShjex516lWbkrMjEhAv1OQeIs=.kguLt2+PLXHlMyA0avLtUnGtnJkeJ3ZqzwxYTguokpU=",
		"urn": "urn:decentraland:matic:collections-v2:0x914c9cb1e619f24734b61c345108fd69401f96ec:0",
	},
}


## The rewards host to claim against. The seeded campaigns are PRODUCTION campaigns
## (their keys only validate on rewards.decentraland.org), so we hit prod regardless of
## build environment — claiming from a dev build with a real wallet still grants the real
## wearable, which is exactly what we want to verify.
static func get_rewards_server() -> String:
	return REWARDS_SERVER_PROD
