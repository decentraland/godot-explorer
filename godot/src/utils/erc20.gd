class_name Erc20
var ethers: float = 0.0
var dollar_value: float = 0.0
var ether_value: float = 0.0
var token = "ETH"


func _init(
	_ethers: float, _token: String, token_to_dollars: float = 0.0, token_to_ethers: float = 0.0
):
	ethers = _ethers
	token = _token
	dollar_value = ethers * token_to_dollars
	ether_value = ethers * token_to_ethers


func _to_string():
	return token + " " + str(ethers)


func dollar_to_string():
	return "US$" + str(snappedf(dollar_value, 0.01))


func ether_to_string():
	return "ETH " + str(snappedf(ether_value, 0.0001))
