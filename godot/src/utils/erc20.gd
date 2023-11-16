class_name ERC20
var wei: BigNumber = null
var value: float = 0.0
var dollar_value: float = 0.0
var ether_value: float = 0.0
var token = "ETH"


func _init(
	wei: BigNumber, token: String, token_to_dollars: float = 0.0, token_to_ethers: float = 0.0
):
	self.wei = wei
	self.token = token
	value = self.wei.divide(BigNumber.new(10, 18)).to_float()
	dollar_value = value * token_to_dollars
	ether_value = value * token_to_ethers


func _to_string():
	return token + " " + str(value)


func dollar_to_string():
	return "US$" + str(snappedf(dollar_value, 0.01))


func ether_to_string():
	return "ETH " + str(snappedf(ether_value, 0.0001))
