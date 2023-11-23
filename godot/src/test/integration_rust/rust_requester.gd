extends Node

var http_requester = RustHttpRequester.new()


func _ready():
	do_test.call_deferred()


func do_test():
	http_requester.request_file(0, "https://httpbin.org/image/png", "algo0.png")
	http_requester.request_file(1, "https://httpbin.org/image/png", "algo1.png")
	http_requester.request_file(2, "https://httpbin.org/image/png", "algo2.png")


func _process(_delta):
	var some: RequestResponse = http_requester.poll()
	if some != null:
		print(
			"> reponse with id: ",
			some.id(),
			" error? ",
			some.is_error(),
			" status_code=",
			some.status_code()
		)
