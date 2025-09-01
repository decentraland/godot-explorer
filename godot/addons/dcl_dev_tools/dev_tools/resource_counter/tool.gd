extends "res://addons/dcl_dev_tools/dev_tools/dcl_dev_tool.gd"

const SINGLETON_NAME = "ResourceCounter"

var places = {
	"Genesis Plaza": Vector2i(0, 0),
	"Soul Magic": Vector2i(96, -110),
	"Tower of Hanoi": Vector2i(61, -22),
	"Meta gamimall": Vector2i(1, 95),
	"Wondermine": Vector2i(-29, 55),
	"Exodus": Vector2i(125, 41),
	"BitCinema": Vector2i(-109, -93),
	"DAO HQ": Vector2i(65, 13),
	"Casa Roustan": Vector2i(37, -114),
	"Fashion Week Scene": Vector2i(142, -78),
	"Game Night": Vector2i(1, 81)
}


func populate_menu(menu: PopupMenu, id: int):
	menu.add_item("Run Benchmark", id)

	if Global.cli.dcl_benchmark:
		print("Running Benchmark...")
		await execute()
		plugin.get_editor_interface().get_editor_main_screen().get_tree().quit()


func execute():
	var interface = plugin.get_editor_interface()

	var old_args = ProjectSettings.get("editor/run/main_run_args")
	for place in places.keys():
		print("Launching: %s" % place)
		var coord = places[place]
		(
			ProjectSettings
			. set(
				"editor/run/main_run_args",
				(
					"--measure-perf --skip-lobby --realm https://realm-provider.decentraland.org/main --location %d,%d"
					% [coord.x, coord.y]
				)
			)
		)

		# If i don't wait settings changes may not impact
		await plugin.get_tree().create_timer(.1).timeout

		plugin.get_editor_interface().play_main_scene()
		while interface.get_playing_scene() != "":
			await plugin.get_tree().create_timer(.1).timeout

	ProjectSettings.set("editor/run/main_run_args", old_args)
