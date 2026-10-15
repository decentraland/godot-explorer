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
## - assign_target / assign_value: OPTIONAL. Some campaigns grant from a pool of possible items and
##   the claim POST must say which one — the rewards server rejects the request with "Missing
##   parameters \"assign_target\" and \"assign_value\" or \"assign_group\"" otherwise. When the
##   campaign only ever grants a single fixed item (e.g. MobilePet) these stay empty and are
##   omitted from the POST body. assign_target is the item's collection contract address,
##   assign_value its blockchain/token id — the same "Collection Address" / "Blockchain ID" shown
##   on the item's row in the rewards dashboard.
## - urn: OPTIONAL wearable/emote urn for the pre-claim preview, built from assign_target/value
##   when known. When empty the modal shows its placeholder art and reveals the real item from the
##   claim response after a successful claim.
const CAMPAIGNS := {
	"MobilePet":
	{
		"campaign_id": "fcfd9412-38f7-4b44-92f6-17a0f10ae53b",
		"campaign_key":
		"Vn0WimL5RjKMpIeDoJIvHfz9lBI490tEkvYXoPEK5Ts=.3QEbxeMg9QXkY/vZrcZyDUYNL0MFrHFatF/KZJ1g7WA=",
		"assign_target": "",
		"assign_value": "",
		"urn": "",
	},
	# login-streak D3 reward, "Rad TV Chill Mode" (an emote) — campaign
	# decentraland.org/rewards/campaign/?id=4237615f-99b6-46a0-8816-b2073a4f2780, item collection
	# 0x701d9caddafdd6944a7d3821720953300e1d0fe0 (blockchain id 0, confirmed resolvable against the
	# catalyst). Not the campaign's own on-chain address (0xca1e4214acd9ba53bda189639878214ecff4de5f)
	# — that one isn't a collection contract and doesn't resolve as an item.
	"LoginStreakD3":
	{
		"campaign_id": "4237615f-99b6-46a0-8816-b2073a4f2780",
		"campaign_key":
		"g8k3ewmNR5mxNWqW2ELAQkI3YV+ZtkagiBayBzpPJ4A=.Yp941hk9ViLVjnIhQ+lsI9Ro2Z4qCZwOFC1LQzKz1rw=",
		"assign_target": "0x701d9caddafdd6944a7d3821720953300e1d0fe0",
		"assign_value": "0",
		"urn": "urn:decentraland:matic:collections-v2:0x701d9caddafdd6944a7d3821720953300e1d0fe0:0",
	},
	# login-streak D7 reward, "RegenesisLabs Community Tall Tee" (a wearable) — campaign
	# decentraland.org/rewards/campaign/?id=ec79d7a9-566e-4acc-8c48-40bf5390788b, item collection
	# 0x996b51131698ba70dcfb3fb5956e6816c5778eda (blockchain id 0, confirmed resolvable against the
	# catalyst). Not the campaign's own on-chain address (0x914c9cb1e619f24734b61c345108fd69401f96ec)
	# — same mixup D3 initially had, that one isn't a collection contract either.
	"LoginStreakD7":
	{
		"campaign_id": "ec79d7a9-566e-4acc-8c48-40bf5390788b",
		"campaign_key":
		"vseXLBe9SjyLN5QqHgShjex516lWbkrMjEhAv1OQeIs=.kguLt2+PLXHlMyA0avLtUnGtnJkeJ3ZqzwxYTguokpU=",
		"assign_target": "0x996b51131698ba70dcfb3fb5956e6816c5778eda",
		"assign_value": "0",
		"urn": "urn:decentraland:matic:collections-v2:0x996b51131698ba70dcfb3fb5956e6816c5778eda:0",
	},
}


## The rewards host to claim against. The seeded campaigns are PRODUCTION campaigns
## (their keys only validate on rewards.decentraland.org), so we hit prod regardless of
## build environment — claiming from a dev build with a real wallet still grants the real
## wearable, which is exactly what we want to verify.
static func get_rewards_server() -> String:
	return REWARDS_SERVER_PROD
