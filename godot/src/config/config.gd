class_name Config extends Object

signal config_changed

var file := ConfigFile.new()

class ConfigData:
	var gravity: float = 55.0
	var resolution: Vector2i = Vector2i()
	var jump_velocity: float = 12.0
	var walk_velocity: float = 2.0
	var run_velocity: float = 6.0
	var process_tick_quota_ms:int = 10
	var scene_radius:int = 1
	var show_fps: bool = true
	var limit_fps: int = 1 # 0 - Unlimited, 1 - VSync, Other-> Limit to that amount
	var skybox: int = 1 # 0- without, 1 - pretty, 2 -default env
	
	func default():
		self.gravity = 55.0
		self.resolution = Vector2i()
		self.jump_velocity = 12.0
		self.walk_velocity = 2.0
		self.run_velocity = 6.0
		self.process_tick_quota_ms = 10
		self.scene_radius = 1
		self.limit_fps = 1
		self.skybox = 1
		
	func from_file(file: ConfigFile):
		var default = ConfigData.new()
		default.default()
		
		self.gravity = file.get_value("gravity", "config", default.gravity) 
		self.resolution = file.get_value("resolution", "config", default.resolution) 
		self.jump_velocity = file.get_value("jump_velocity", "config", default.jump_velocity) 
		self.walk_velocity = file.get_value("walk_velocity", "config", default.walk_velocity) 
		self.run_velocity = file.get_value("run_velocity", "config", default.run_velocity) 
		self.process_tick_quota_ms = file.get_value("process_tick_quota_ms", "config", default.process_tick_quota_ms) 
		self.scene_radius = file.get_value("scene_radius", "config", default.scene_radius)
		
var default_data: ConfigData = ConfigData.new()
var current_data: ConfigData = ConfigData.new()

func init():
	default_data.default()
	
	var err = file.load("user://settings.cfg")
	if err == OK:
		current_data.from_file(self.file)
	
