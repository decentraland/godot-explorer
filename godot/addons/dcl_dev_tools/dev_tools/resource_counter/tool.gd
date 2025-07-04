extends "res://addons/dcl_dev_tools/dev_tools/dcl_dev_tool.gd"

const SINGLETON_NAME = "ResourceCounter"
const SETTINGS_PATH_KEY = "resource_counter/enable_counting"

# Repeated data from typical places tool
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


func _init(a_plugin: EditorPlugin):
	plugin = a_plugin
	if not ProjectSettings.has_setting(SETTINGS_PATH_KEY):
		ProjectSettings.set(SETTINGS_PATH_KEY, false)
		(
			ProjectSettings
			. add_property_info(
				{
					"type": TYPE_BOOL,
					"name": SETTINGS_PATH_KEY,
				}
			)
		)

	ProjectSettings.settings_changed.connect(
		func():
			var singleton_exists = ProjectSettings.has_setting("autoload/%s" % SINGLETON_NAME)
			var should_exist = ProjectSettings.get_setting(SETTINGS_PATH_KEY)
			if should_exist and !singleton_exists:
				plugin.add_autoload_singleton(
					SINGLETON_NAME,
					get_script().resource_path.get_base_dir().path_join("resource_counter.gd")
				)
			elif !should_exist and singleton_exists:
				plugin.remove_autoload_singleton(SINGLETON_NAME)
	)

func populate_menu(menu: PopupMenu, id: int):
	menu.add_item("Run Benchmark", id)

	var args := OS.get_cmdline_args()
	if args.has("--dcl-benchmark"):
		print("Running Benchmark...")
		execute()

func execute():
	var was_counting = ProjectSettings.get(SETTINGS_PATH_KEY)
	ProjectSettings.set(SETTINGS_PATH_KEY, true)
	var interface = plugin.get_editor_interface()

	var old_args = ProjectSettings.get("editor/run/main_run_args")
	for place in places.keys():
		print("Launching: %s" % place)
		var coord = places[place]
		ProjectSettings.set(
			"editor/run/main_run_args",
			(
				"--skip-lobby --realm https://realm-provider.decentraland.org/main --location %d,%d"
				% [coord.x, coord.y]
			)
		)
		plugin.get_editor_interface().play_main_scene()
		while interface.get_playing_scene() != "":
			await plugin.get_tree().create_timer(.1).timeout

	ProjectSettings.set("editor/run/main_run_args", old_args)
	ProjectSettings.set(SETTINGS_PATH_KEY, was_counting)
