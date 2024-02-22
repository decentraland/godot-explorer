extends Control

const shark_gltf = preload("res://src/decentraland_components/animation/shark.glb")
@onready var sub_viewport = $SubViewportContainer/SubViewport

# Called when the node enters the scene tree for the first time.
func _ready():
	var with_animation_blend = false
	
	for i in range(1000):
		var shark = shark_gltf.instantiate()
		sub_viewport.add_child(shark)
		shark.position.x = 3 * i
		
		if with_animation_blend:
			var animation_blend = GdAnimationBlendBuilder.new()
			animation_blend.anim_player = "../AnimationPlayer"
			shark.add_child(animation_blend)
			
			animation_blend.generate_animation_blend_tree(10)	
			# Example
		# (_playing: bool, _weight: float, _loop: bool, _should_reset: bool, _speed: float)
			animation_blend.apply_anims({
				"bite": GdAnimationBlendBuilder.Item.new(true, 1.0, true, true, 1.0),
				"swim": GdAnimationBlendBuilder.Item.new(false, 0.2, true, true, 1.0),
			})
		else:
			var anim_player: AnimationPlayer = shark.get_node("AnimationPlayer")
			anim_player.get_animation("bite").loop_mode = Animation.LOOP_LINEAR
			shark.get_node("AnimationPlayer").play("bite")

# Called every frame. 'delta' is the elapsed time since the previous frame.
func _process(delta):
	pass
