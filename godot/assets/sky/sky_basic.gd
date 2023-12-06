extends WorldEnvironment

var sun_light: DirectionalLight3D


func _ready():
	sun_light = DirectionalLight3D.new()
	sun_light.light_color = Color("fffcc4")
	sun_light.rotate_x(-PI / 3)
	sun_light.name = "DirectionalLight3D_SunBasic"
	add_sibling(sun_light)
