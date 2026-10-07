extends SceneTree

# Dump the art-pipeline locomotion GLB (no-export source) as a shippable
# AnimationLibrary .tres. Run:
#   .bin/godot/Godot.app/Contents/MacOS/Godot --headless --path godot \
#       --script src/tool/anim_converter/dump_locomotion_library.gd

const SRC := "res://assets/no-export/locomotion/Avatar_Locomotion_Fix.glb"
const OUT := "res://assets/animations/locomotion_full.tres"


func _initialize() -> void:
	var lib: AnimationLibrary = load(SRC)
	if lib == null:
		printerr("GLB did not load as AnimationLibrary — reimport first")
		quit(1)
		return
	var err := ResourceSaver.save(lib, OUT)
	prints("clips:", lib.get_animation_list().size(), "saved:", OUT, "err:", err)
	quit(0 if err == OK else 1)
