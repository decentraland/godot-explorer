extends RefCounted
class_name RustHttpRequesterWrapper

var _requester: = RustHttpRequester.new()
var promises: Dictionary = {}

# Dictionary mapping HTTP status codes to their descriptions.
var http_status_descriptions = {
	100: "Continue",
	101: "Switching Protocols",
	200: "OK",
	201: "Created",
	202: "Accepted",
	203: "Non-Authoritative Information",
	204: "No Content",
	205: "Reset Content",
	206: "Partial Content",
	300: "Multiple Choices",
	301: "Moved Permanently",
	302: "Found",
	303: "See Other",
	304: "Not Modified",
	307: "Temporary Redirect",
	308: "Permanent Redirect",
	400: "Bad Request",
	401: "Unauthorized",
	403: "Forbidden",
	404: "Not Found",
	405: "Method Not Allowed",
	406: "Not Acceptable",
	407: "Proxy Authentication Required",
	408: "Request Timeout",
	409: "Conflict",
	410: "Gone",
	411: "Length Required",
	412: "Precondition Failed",
	413: "Payload Too Large",
	414: "URI Too Long",
	415: "Unsupported Media Type",
	416: "Range Not Satisfiable",
	417: "Expectation Failed",
	418: "I'm a teapot", # Easter egg status code
	426: "Upgrade Required",
	429: "Too Many Requests",
	451: "Unavailable For Legal Reasons",
	500: "Internal Server Error",
	501: "Not Implemented",
	502: "Bad Gateway",
	503: "Service Unavailable",
	504: "Gateway Timeout",
	505: "HTTP Version Not Supported"
}

func get_status_description(status_code: int) -> String:
	# Returns the description for the given HTTP status code.
	# If the status code is not found, returns an empty string or a default message.
	return "Status Code " + str(status_code) + " " + http_status_descriptions.get(status_code, "Unknown Status Code")

func is_success_status_code(status_code: int) -> bool:
	# Checks if the given status code is within the success range (200-299)
	return 200 <= status_code and status_code <= 299

func request_file(reference_id: int, url: String, absolute_path: String):
	var id = _requester.request_file(reference_id, url, absolute_path)

	var promise = Promise.new()
	promises[id] = promise
	return promise
	
func request_json(reference_id: int, url: String, method: int, body: String, headers: Array):
	var id = _requester.request_json(reference_id, url, method, body, headers)

	var promise = Promise.new()
	promises[id] = promise
	return promise

func _ready():
	self.on_completed.connect(self._on_completed)
	self.on_error.connect(self._on_error)
	
func poll():
	var res = _requester.poll()
	if res is RequestResponse:
		var response: RequestResponse = res
		var id = response.id()
		var promise: Promise = promises[id]
		if response.is_error():
			promise.reject(response.get_error())
		elif !is_success_status_code(response.status_code()):
			promise.reject(get_status_description(response.status_code()))
		else:
			promise.resolve_with_data(response.get_string_response_as_json())
		promises.erase(id)
	elif res is RequestResponseError:
		var error: RequestResponseError = res
		var id = error.id()
		var promise: Promise = promises[id]
		promise.reject(error.get_error_message())
		promises.erase(id)
