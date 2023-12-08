class_name PlayerIdentity extends DclPlayerIdentity

var current_lambda_server_base_url: String = "https://peer.decentraland.org/lambdas/"

func _ready():
	wallet_connected.connect(self._on_wallet_connected)
	Global.realm.realm_changed.connect(self._on_realm_changed)

func _on_realm_changed():
	if Global.realm.lambda_server_base_url == current_lambda_server_base_url:
		return
	current_lambda_server_base_url = Global.realm.lambda_server_base_url
	if not get_address_str().is_empty() and not self.is_guest:
		async_fetch_profile(get_address_str(), current_lambda_server_base_url)
	
func async_fetch_profile(address: String, lambda_server_base_url :String) -> void:
	var url = lambda_server_base_url + "profiles/" + address
	var promise: Promise = Global.http_requester.request_json(
		url, HTTPClient.METHOD_GET, "", []
	)

	var res = await promise.async_awaiter()
	
	# Are we still needing to fetch this profile?
	if get_address_str() != address or current_lambda_server_base_url != lambda_server_base_url:
		print("fetc profile dismissed")
		return
			
	if res is Promise.Error:
		if res._error_description.find("404") != -1:
			# Deploy profile?
			update_profile(Global.config.default_profile())
			print("Profile not found " + url)
		else:
			update_profile(Global.config.default_profile())
			printerr("Error while fetching profile " + url, " reason: ", res._error_description)
			return
		
	prints(res)

	
func _on_wallet_connected(address: String, chain_id: int, is_guest: bool):
	if is_guest:
		update_profile(Global.config.default_profile())
		return
	
	async_fetch_profile(address, current_lambda_server_base_url)
	
