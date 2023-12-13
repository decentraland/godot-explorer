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


func async_fetch_profile(address: String, lambda_server_base_url: String) -> void:
	var url = lambda_server_base_url + "profiles/" + address
	var promise: Promise = Global.http_requester.request_json(url, HTTPClient.METHOD_GET, "", [])

	var response = await PromiseUtils.async_awaiter(promise)

	# Are we still needing to fetch this profile?
	if get_address_str() != address or current_lambda_server_base_url != lambda_server_base_url:
		print("fetc profile dismissed")
		return

	if response is PromiseError:
		if response.get_error().find("404") != -1:
			# Deploy profile?
			self.set_default_profile()
			print("Profile not found " + url)
		else:
			self.set_default_profile()
			printerr("Error while fetching profile " + url, " reason: ", response.get_error())
			return

	if not self._update_profile_from_lambda(response):
		self.set_default_profile()


func _on_wallet_connected(address: String, _chain_id: int, is_guest: bool):
	if is_guest:
		self.set_default_profile()
		return

	async_fetch_profile(address, current_lambda_server_base_url)


func async_deploy_profile(new_profile: Dictionary) -> void:
	var promise: Promise = self.async_prepare_deploy_profile(new_profile)
	var ret = await PromiseUtils.async_awaiter(promise)
	if ret is PromiseError:
		print(ret)
		return

	var headers := ["Content-Type: " + (ret as Dictionary).get("content_type")]
	var url := Global.realm.content_base_url + "entities/"
	var promise_req := Global.http_requester.request_json_bin(
		url, HTTPClient.METHOD_POST, (ret as Dictionary).get("body_payload"), headers
	)
	var response = await PromiseUtils.async_awaiter(promise_req)
	if response is PromiseError:
		print(response.get_error())

		var test_file = FileAccess.open("test.request.bin", FileAccess.WRITE)
		test_file.store_buffer((ret as Dictionary).get("body_payload"))
		test_file.close()

		return

	response = (response as RequestResponse).get_string_response_as_json()
	if response is Dictionary:
		if response.get("creationTimestamp") != null:
			self._update_profile_from_dictionary(new_profile)
