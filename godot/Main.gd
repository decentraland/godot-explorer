extends Node


# Called when the node enters the scene tree for the first time.
func _ready():
	pass

	var main_class: MainTestClass = MainTestClass.new()
	print('main class created')

	main_class.start_scene()
	self.add_child(main_class)

# Called every frame. 'delta' is the elapsed time since the previous frame.
func _process(delta):
	$Label.set_text("FPS " + str(Engine.get_frames_per_second()))
