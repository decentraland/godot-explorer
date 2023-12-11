class_name RustHttpRequesterWrapper
extends RefCounted

# Dictionary mapping HTTP status codes to their descriptions.
const HTTP_STATUS_DESCRIPTIONS = {
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
	418: "I'm a teapot",  # Easter egg status code
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

var promises: Dictionary = {}

var _requester := RustHttpRequester.new()


func get_status_description(status_code: int) -> String:
	# Returns the description for the given HTTP status code.
	# If the status code is not found, returns an empty string or a default message.
	return (
		"Status Code "
		+ str(status_code)
		+ " "
		+ HTTP_STATUS_DESCRIPTIONS.get(status_code, "Unknown Status Code")
	)


func is_success_status_code(status_code: int) -> bool:
	# Checks if the given status code is within the success range (200-299)
	return 200 <= status_code and status_code <= 299


func request_file(url: String, absolute_path: String) -> Promise:
	var id = _requester.request_file(0, url, absolute_path)

	var promise = Promise.new()
	promises[id] = promise
	return promise


func request_json(url: String, method: int, body: String, headers: Array) -> Promise:
	var id = _requester.request_json(0, url, method, body, headers)

	var promise = Promise.new()
	promises[id] = promise
	return promise

func request_json_bin(url: String, method: int, body: PackedByteArray, headers: Array) -> Promise:
	var id = _requester.request_json_bin(0, url, method, body, headers)

	var promise = Promise.new()
	promises[id] = promise
	return promise

func poll():
	var res = _requester.poll()
	if res is RequestResponse:
		var response: RequestResponse = res
		var id = response.id()
		var promise: Promise = promises[id]
		if response.is_error():
			promise.reject(response.get_error())
		elif !is_success_status_code(response.status_code()):
			var payload = response.get_response_as_string()
			if payload != null:
				promise.reject(payload)
			else:
				promise.reject(get_status_description(response.status_code()))
		else:
			promise.resolve_with_data(response)
		promises.erase(id)
	elif res is RequestResponseError:
		var error: RequestResponseError = res
		var id = error.id()
		var promise: Promise = promises[id]
		promise.reject(error.get_error_message())
		promises.erase(id)

	if res != null:
		poll()
