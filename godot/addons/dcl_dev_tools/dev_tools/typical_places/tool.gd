extends "res://addons/dcl_dev_tools/dev_tools/dcl_dev_tool.gd"

var places = {
	"Genesis Plaza": Vector2(0, 0),
	"Soul Magic": Vector2(96, -110),
	"Tower of Hanoi": Vector2(61, -22),
	"Meta gamimall": Vector2(1, 95),
	"Wondermine": Vector2(-29, 55),
	"Exodus": Vector2(125, 41),
	"BitCinema": Vector2(-109, -93),
	"DAO HQ": Vector2(65, 13),
	"Casa Roustan": Vector2(37, -114),
	"Fashion Week Scene": Vector2(142, -78),
	"Game Night": Vector2(1, 81)
}


func populate_menu(menu: PopupMenu, id: int):
	var submenu = PopupMenu.new()
	for place_id in places.keys().size():
		var place = places.keys()[place_id]
		submenu.add_item(place, place_id)
	menu.add_submenu_node_item("Launch on", submenu, id)
	submenu.id_pressed.connect(_on_menu_item_selected)


func _on_menu_item_selected(id: int):
	var place = places.keys()[id]
	var coord = places[place]

	var old_args = ProjectSettings.get("editor/run/main_run_args")
	ProjectSettings.set(
		"editor/run/main_run_args",
		(
			"--skip-lobby --realm https://realm-provider.decentraland.org/main --location %d,%d"
			% [coord.x, coord.y]
		)
	)
	plugin.get_editor_interface().play_main_scene()
	ProjectSettings.set("editor/run/main_run_args", old_args)


# No-op. is a submenu.
func execute():
	pass
