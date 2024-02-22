class_name GdAnimationBlendBuilder
extends AnimationTree


class Item extends RefCounted:
	var weight: float = 1.0
	var speed: float = 1.0
	var loop: bool = false
	var playing: bool = false
	var should_reset: bool = false
	var index: int = -1
	
	func _init(_playing: bool, _weight: float, _loop: bool, _should_reset: bool, _speed: float):
		weight = _weight
		loop = _loop
		playing = _playing
		speed = _speed
		should_reset = _should_reset
		
	
var _current_anim_dict: Dictionary = {}
var _current_capacity = 0

var _animation_player: AnimationPlayer = null

func _on_animation_player_changed():
	_animation_player = get_node_or_null(self.anim_player)
	if _animation_player == null:
		return
	
	# Ensure Dummy animation
	var anim_lib = _animation_player.get_animation_library("")
	if not anim_lib.has_animation("__dummy__"):
		anim_lib.add_animation("__dummy__", Animation.new())
		
func _ready():
	self.animation_finished.connect(self._animation_finished)
	_on_animation_player_changed()
	

func _animation_finished(anim_name: StringName):
	if _current_anim_dict.has(anim_name):
		var cur_item: Item = _current_anim_dict[anim_name]
		if cur_item.loop or cur_item.should_reset:
			self.set("parameters/anim_" + str(cur_item.index) + "/time", 0.0)
		
		if not cur_item.loop:
			self.set("parameters/sanim_" + str(cur_item.index) + "/scale", 0.0)
			
	
func generate_animation_blend_tree( N: int) -> void:
	N = max(2, N)
	
	var tree: AnimationNodeBlendTree = AnimationNodeBlendTree.new()
	self.tree_root = tree
	
	for i in range(N):
		var blend_anim_node = AnimationNodeBlend2.new()
		var speed_anim_node = AnimationNodeTimeScale.new()
		var dummy_anim_node = AnimationNodeAnimation.new()
		var anim_node = AnimationNodeAnimation.new()
		
		anim_node.animation = "__dummy__"
		dummy_anim_node.animation = "__dummy__"
		
		tree.add_node("danim_" + str(i), dummy_anim_node)
		tree.add_node("sanim_" + str(i), speed_anim_node)
		tree.add_node("blend_" + str(i), blend_anim_node)
		tree.add_node("anim_" + str(i), anim_node)
		
		tree.connect_node("sanim_" + str(i), 0, "anim_" + str(i))
		tree.connect_node("blend_" + str(i), 0, "danim_" + str(i))
		tree.connect_node("blend_" + str(i), 1, "sanim_" + str(i))
		
		self.set("parameters/blend_" + str(i) +  "/blend_amount", 1.0)
		
		if i < N - 1:
			var add_node = AnimationNodeAdd2.new()
			tree.add_node("add_" + str(i), add_node)
			
			
	for i in range(N - 1):
		if i == 0:
			tree.connect_node("add_0", 0, "blend_0")
			tree.connect_node("add_0", 1, "blend_1")
		else:
			tree.connect_node("add_" + str(i), 0, "add_" + str(i - 1))
			tree.connect_node("add_" + str(i), 1, "blend_" + str(i + 1))
			
		self.set("parameters/add_" + str(i) +  "/add_amount", 1)

	tree.connect_node("output", 0, "add_" + str(N - 2))
	_current_capacity = N
	
func apply_anims(anim_dict: Dictionary):
	if _current_capacity < anim_dict.keys().size():
		generate_animation_blend_tree(anim_dict.values().size())
		
	if _animation_player == null:
		return
		
	var cur_item: Item
	var anim_keys = anim_dict.keys()
	for i in range(_current_capacity):
		if i < anim_keys.size() and _animation_player.has_animation(anim_keys[i]):
			cur_item = anim_dict[anim_keys[i]]
			cur_item.index = i
			self.tree_root.get_node("anim_" + str(i)).animation = anim_keys[i]
		else:
			self.tree_root.get_node("anim_" + str(i)).animation = "__dummy__"
			continue
			
		self.set("parameters/blend_" + str(i) + "/blend_amount", cur_item.weight)
		
		if cur_item.playing:
			self.set("parameters/sanim_" + str(i) + "/scale", cur_item.speed)
		else:
			self.set("parameters/sanim_" + str(i) + "/scale", 0.0)
		
		if cur_item.should_reset:
			self.set("parameters/anim_" + str(i) + "/time", 0.0)
			
			
	_current_anim_dict = anim_dict



