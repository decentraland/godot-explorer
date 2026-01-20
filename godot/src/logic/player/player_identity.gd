class_name PlayerIdentity extends DclPlayerIdentity

var _current_lambda_server_base_url: String = DclUrls.peer_lambdas()


func _ready():
	wallet_connected.connect(self._on_wallet_connected)
	Global.realm.realm_changed.connect(self._on_realm_changed)


func _on_realm_changed():
	if Global.realm.get_lambda_server_base_url() == _current_lambda_server_base_url:
		return
	_current_lambda_server_base_url = Global.realm.get_lambda_server_base_url()
	if not get_address_str().is_empty() and not self.is_guest:
		async_fetch_profile(get_address_str(), _current_lambda_server_base_url)


func async_fetch_profile(address: String, requested_lambda_server_base_url: String) -> void:
	var promise = ProfileService.async_fetch_profile(address, requested_lambda_server_base_url)
	var response = await PromiseUtils.async_awaiter(promise)

	# Are we still needing to fetch this profile?
	if (
		get_address_str() != address
		or _current_lambda_server_base_url != requested_lambda_server_base_url
	):
		print("fetch profile dismissed")
		return

	if response is PromiseError:
		if response.get_error().find("404") != -1:
			# Deploy profile?
			self.set_default_profile_or_guest_profile()
			print("Profile not found for address " + address)
		else:
			self.set_default_profile_or_guest_profile()
			printerr(
				"Error while fetching profile for " + address, " reason: ", response.get_error()
			)
			return

	if not self._update_profile_from_lambda(response):
		self.set_default_profile_or_guest_profile()


func set_default_profile_or_guest_profile():
	if Global.get_config().guest_profile.is_empty():
		self.set_default_profile()
	else:
		var guest_profile := DclUserProfile.from_godot_dictionary(Global.get_config().guest_profile)
		self.set_profile(guest_profile)


func _on_wallet_connected(address: String, _chain_id: int, is_guest_value: bool):
	if is_guest_value:
		set_default_profile_or_guest_profile()
		return

	async_fetch_profile(address, _current_lambda_server_base_url)
