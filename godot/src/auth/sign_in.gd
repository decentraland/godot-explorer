extends Control

@onready var label = $Label

func _ready():
	var test = DclPlayerIdentity.new()
	add_child(test)
	test.need_open_url.connect(self._on_need_open_url)
	test.wallet_connected.connect(self._on_wallet_connected)
	test.try_connect_account()
	
func _on_need_open_url(url: String, description: String) -> void:
	OS.shell_open(url)
	prints("url ", url, "desc", description)

func _on_wallet_connected(address: String, chain_id: int) -> void:
	prints("wallet connected", address, "on chain", chain_id)
	label.text = "wallet connected " + address + " on chain_id=" + str(chain_id)
	
	
