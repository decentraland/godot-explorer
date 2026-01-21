class_name PlayerIdentity extends DclPlayerIdentity

var _mutable_avatar: DclAvatarWireFormat
var _mutable_profile: DclUserProfile
var _current_profile: DclUserProfile

var _current_lambda_server_base_url: String = "https://peer.decentraland.org/lambdas/"


func _ready():
	wallet_connected.connect(self._on_wallet_connected)
	Global.realm.realm_changed.connect(self._on_realm_changed)

	_mutable_profile = DclUserProfile.new()
	_current_profile = DclUserProfile.new()
	_mutable_avatar = _mutable_profile.get_avatar()

	profile_changed.connect(_on_profile_changed)


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


## Save profile (ADR-290: snapshots are no longer uploaded)
func async_save_profile_metadata() -> void:
	await ProfileService.async_deploy_profile(_mutable_profile)


## ADR-290: Snapshots are no longer uploaded to the server.
## Profile images are served on-demand by the profile-images service.
func async_save_profile() -> void:
	_mutable_profile.set_has_connected_web3(!Global.player_identity.is_guest)

	# Generate local snapshots for immediate UI display (not uploaded)
	await Global.snapshot.async_generate_for_avatar(_mutable_avatar, _mutable_profile)

	_mutable_profile.set_avatar(_mutable_avatar)

	# Update blocked and muted lists from social_blacklist
	_mutable_profile.set_blocked(Global.social_blacklist.get_blocked_list())
	_mutable_profile.set_muted(Global.social_blacklist.get_muted_list())

	# Deploy profile to server (ADR-290: no snapshots in deployment)
	await ProfileService.async_deploy_profile(_mutable_profile)


## Check both profile changes AND avatar changes
## Avatar is a clone, so changes to mutable_avatar don't affect mutable_profile directly
func has_changes():
	var original_avatar = _current_profile.get_avatar()
	if not original_avatar.equal(_mutable_avatar):
		return true
	return not _current_profile.equal(_mutable_profile)


func _on_profile_changed(new_profile: DclUserProfile):
	_mutable_profile = new_profile.duplicated()
	_current_profile = new_profile.duplicated()
	_mutable_avatar = _mutable_profile.get_avatar()

	# Update social blacklist from the profile
	Global.social_blacklist.init_from_profile(new_profile)


func get_mutable_avatar() -> DclAvatarWireFormat:
	return _mutable_avatar


func get_mutable_profile() -> DclUserProfile:
	return _mutable_profile
