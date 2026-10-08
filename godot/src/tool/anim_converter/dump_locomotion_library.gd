extends SceneTree

# Dump the art-pipeline locomotion GLB (no-export source) as a shippable
# AnimationLibrary .tres. Run:
#   .bin/godot/Godot.app/Contents/MacOS/Godot --headless --path godot \
#       --script src/tool/anim_converter/dump_locomotion_library.gd

const SRC := "res://assets/no-export/locomotion/Avatar_Locomotion_Fix.glb"
const OUT := "res://assets/animations/locomotion_full.tres"
# The GLB import loses loop flags; gait clips must loop (they don't here, and
# a frozen last frame is the "patinando" bug). Slide loops too.
# Jump_Mid/Run_Jump_Mid loop so their auto-advance never fires — the apex
# holds until fall_fast (vy<-3, Unity AnimationFallSpeed) triggers (#1553).
const FORCE_LOOP := ["Idle", "Walk", "Jog", "Run", "Slide", "Jump_Mid", "Run_Jump_Mid"]


func _initialize() -> void:
	var lib: AnimationLibrary = load(SRC)
	if lib == null:
		printerr("GLB did not load as AnimationLibrary — reimport first")
		quit(1)
		return
	for clip_name in FORCE_LOOP:
		if lib.has_animation(clip_name):
			lib.get_animation(clip_name).loop_mode = Animation.LOOP_LINEAR
	var err := ResourceSaver.save(lib, OUT)
	prints("clips:", lib.get_animation_list().size(), "saved:", OUT, "err:", err)
	quit(0 if err == OK else 1)
