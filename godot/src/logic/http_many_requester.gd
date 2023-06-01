extends Node
class_name HTTPManyRequester

signal request_completed(hash: String, result: int, response_code: int, headers: PackedStringArray, body: PackedByteArray)

var pending_requests: Array = []
var available_requests: Array = []

@export var use_threads: bool = false
@export var requester_count: int = 10

func _ready():
	for index in range(requester_count):
		var http_request_child: HTTPRequest = HTTPRequest.new()
		self.add_child(http_request_child)
		http_request_child.request_completed.connect(self._http_request_completed.bind(http_request_child))
		http_request_child.use_threads = self.use_threads
		available_requests.append(http_request_child)
		
	
func do_request_raw(url: String, method: HTTPClient.Method, body: String, headers: PackedStringArray, timeout: int = 0, file: String = ""):
	var time_hash = str(Time.get_ticks_usec())
	pending_requests.push_back({
		"hash": time_hash,
		"url": url,
		"method": method,
		"body": body,
		"headers": headers,
		"timeout": timeout,
		"file": file
	})
	
	while true:
		var signal_result = await self.request_completed
		if signal_result[0] == time_hash:
			return signal_result
			
# If the request was OK, the returned value is not null
func do_request_text(url: String, method: HTTPClient.Method, body: String = "", headers: PackedStringArray = [], timeout: int = 0) -> Variant:
	var result = await do_request_raw(url, method, body, headers, timeout)
	var result_code = result[1]
	var http_response_code = result[2]
	
	if result_code != OK or http_response_code < 200 or http_response_code > 299:
		return null
	
	var response_body: PackedByteArray = result[4] as PackedByteArray
	return response_body.get_string_from_utf8()
	
# If the request was OK, the returned value is not null
func do_request_file(url: String, file_hash: String, timeout: int = -1) -> Variant:
	var local_file = "user://content/" + file_hash 
	DirAccess.make_dir_recursive_absolute("user://content/")
	
	if FileAccess.file_exists(local_file):
		DirAccess.remove_absolute(local_file)
	
	var result = await do_request_raw(url, HTTPClient.METHOD_GET, "", [], timeout, local_file)
	var result_code = result[1]
	var http_response_code = result[2]
	
	if result_code != OK or http_response_code < 200 or http_response_code > 299:
		return null
		
	var response_body: PackedByteArray = result[4] as PackedByteArray
	return response_body
	
# If the request was OK, the returned value is not null
func do_request_json(url: String, method: HTTPClient.Method, body: String = "", headers: PackedStringArray = [], timeout: int = 0) -> Variant:
	var result = await do_request_raw(url, method, body, headers, timeout)
	var result_code = result[1]
	var http_response_code = result[2]
	
	if result_code != OK or http_response_code < 200 or http_response_code > 299:
		return null
		
	var json_str: String = (result[4] as PackedByteArray).get_string_from_utf8()
	
	if json_str.length() > 0:
		var json = JSON.parse_string(json_str)
		if json == null:
			printerr("do_request_json failed because json_string is not a valid json with length ", json_str.length(), " ", url, " ", method, " ", body, " ", headers, " ", timeout)
		return json
	else:
		return null
		
func _process(_dt):
	if pending_requests.size() > 0 and available_requests.size() > 0:
		var request = pending_requests.pop_front()
#		print("getting new request", request)
		var http_request: HTTPRequest = available_requests.pop_front()
		http_request.timeout = min(30.0, max(0.0, float(request["timeout"])))
		http_request.set_meta("hash", request["hash"])
		
		var file: String = request["file"]
		if file.length() > 0:
			http_request.download_file = file
		
		var err = http_request.request(
			request["url"], 
			request["headers"], 
			request["method"], 
			request["body"]
		)
		
		if err != OK:
			emit_signal("request_completed", request["hash"], err, 0, [], [])
			available_requests.append(http_request)

func _http_request_completed(result: int, response_code: int, headers: PackedStringArray, body: PackedByteArray, http_request: HTTPRequest):
	available_requests.append(http_request)
	emit_signal("request_completed", http_request.get_meta("hash"), result, response_code, headers, body)
