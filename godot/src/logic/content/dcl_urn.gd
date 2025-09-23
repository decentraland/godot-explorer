class_name DclUrn

const SEPARATOR: String = ":"

var valid: bool = false
var chain: String = ""
var contract_address: String = ""
var token_id: String = ""
var urn: String = ""


func _init(_urn):
	var urn_parts: PackedStringArray = _urn.split(SEPARATOR)

	# 0: "urn"
	if urn_parts[0] != "urn":
		return

	# 1: "decentraland"
	if urn_parts[1] != "decentraland":
		return

	self.urn = _urn

	# 2: "chain"
	self.chain = urn_parts[2]

	# 3: contract standard (not used, but we skip it)
	# 4: contract address
	self.contract_address = urn_parts[4]

	# 5: token id
	self.token_id = urn_parts[5]

	self.valid = true


func get_hash() -> String:
	return contract_address + "_" + token_id
