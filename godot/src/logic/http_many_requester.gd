extends Node
class_name HTTPManyRequester

signal request_completed(reference_id: int, request_id: String, result: int, response_code: int, headers: PackedStringArray, body: PackedByteArray)

var pending_requests: Array = []
var requester: Array = []

const MAX_REQUESTER = 1000
var http_requester_count = 0
	
func request(reference_id: int, url: String, method: HTTPClient.Method = HTTPClient.METHOD_GET, body: String = "", headers: PackedStringArray = [], timeout: int = 0, file: String = "") -> String:
	var request_id = str(Time.get_ticks_usec())
	pending_requests.push_back({
		"request_id": request_id,
		"url": url,
		"method": method,
		"body": body,
		"headers": headers,
		"timeout": timeout,
		"file": file,
		"reference_id": reference_id
	})
	return request_id
			

func _process(_dt):
	if pending_requests.size() > 0  and requester.size() < MAX_REQUESTER:
		var pending_request = pending_requests.pop_front()
		
		http_requester_count += 1
		print("adding http requester ", http_requester_count)
		
		var new_http_request: HTTPRequest = HTTPRequest.new()
		new_http_request.name = "http_request_" + str(http_requester_count)
		new_http_request.timeout = 5
		new_http_request.use_threads = true
		new_http_request.request_completed.connect(self._http_request_completed.bind(new_http_request))
#		http_request.timeout = min(30.0, max(0.0, float(request["timeout"])))
		new_http_request.set_meta("request_id", pending_request["request_id"])
		new_http_request.set_meta("reference_id", pending_request["reference_id"])
		
		var file: String = pending_request["file"]
		if file.length() > 0:
			new_http_request.download_file = file
			
		self.add_child(new_http_request)
		
		var err = new_http_request.request(
			pending_request["url"], 
			pending_request["headers"], 
			pending_request["method"], 
			pending_request["body"]
		)
		
		if err != OK:
			printerr("many request fail ", err)
			emit_signal("request_completed", pending_request["reference_id"], pending_request["request_id"], err, 0, [], [])
		else:
			requester.push_back(new_http_request)

func _http_request_completed(result: int, response_code: int, headers: PackedStringArray, body: PackedByteArray, http_request: HTTPRequest):
	emit_signal.call_deferred("request_completed", http_request.get_meta("reference_id"), http_request.get_meta("request_id"), result, response_code, headers, body)
	clean_requester.call_deferred(http_request)
	
func clean_requester(http_request: HTTPRequest):
	requester.erase(http_request)
