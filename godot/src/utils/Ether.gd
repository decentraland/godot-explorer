class_name Ether


static func is_valid_ethereum_address(address: String) -> bool:
	var regex = RegEx.new()
	regex.compile("^0x[a-fA-F0-9]{40}$")
	return regex.search(address) != null


static func shorten_eth_address(eth_address: String) -> String:
	if eth_address.length() <= 10:
		# Si la dirección es demasiado corta para ser válida o para aplicar el formato, devuelve la dirección original.
		return eth_address

	# Extrae los primeros 6 caracteres y los últimos 4 caracteres.
	var start: String = eth_address.substr(0, 6)
	var end: String = eth_address.substr(eth_address.length() - 4, 4)

	# Combina las partes con puntos suspensivos en el medio.
	return start + "..." + end
