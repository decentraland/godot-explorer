class_name PlayerIdentity extends DclPlayerIdentity

var _current_lambda_server_base_url: String = ""


func _ready():
	wallet_connected.connect(self._on_wallet_connected)
	Global.realm.realm_changed.connect(self._on_realm_changed)


func _on_realm_changed():
	var new_url = Global.realm.get_lambda_server_base_url()
	if new_url == _current_lambda_server_base_url:
		return
	_current_lambda_server_base_url = new_url
	if not get_address_str().is_empty() and not self.is_guest:
		async_fetch_profile(get_address_str(), _current_lambda_server_base_url)


func async_fetch_profile(address: String, lambda_server_url: String) -> void:
	var promise = ProfileService.async_fetch_profile(address, lambda_server_url)
	var response = await PromiseUtils.async_awaiter(promise)

	# Are we still needing to fetch this profile?
	if get_address_str() != address or _current_lambda_server_base_url != lambda_server_url:
		print("fetch profile dismissed")
		return

	if response is PromiseError:
		# Profile not found or error - clear saved guest profile and start fresh
		Global.get_config().guest_profile = {}
		Global.get_config().save_to_settings_file()
		self.set_default_profile()

		if response.get_error().find("404") != -1:
			print("Profile not found for address " + address + " - starting with default profile")
		else:
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
		# Update the address to match the current wallet (guest profile may have old guest address)
		var current_address = get_address_str()
		if not current_address.is_empty():
			guest_profile.set_ethereum_address(current_address)
		self.set_profile(guest_profile)


func _on_wallet_connected(address: String, _chain_id: int, is_guest_value: bool):
	if is_guest_value:
		set_default_profile_or_guest_profile()
		return

	# Get the current lambda URL from realm
	_current_lambda_server_base_url = Global.realm.get_lambda_server_base_url()
	async_fetch_profile(address, _current_lambda_server_base_url)
