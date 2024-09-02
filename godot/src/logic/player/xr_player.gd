class_name XRPlayer
extends XROrigin3D

## Curve for following the camera
@export var follow_curve : Curve
@export var follow_speed : float = 50.0

@onready var camera: Camera3D = $XRCamera3D
@onready var avatar := $Avatar

@onready var player_body := $PlayerBody

@onready var right_hand = %RightHand
@onready var left_hand = $LeftHand

@onready var vr_screen: Node3D = %VrScreen
@onready var ui_origin_3d = %UIOrigin3D

# Camera to track
@onready var xr_camera : XRCamera3D = %XRCamera3D

@onready var microphone_gltf = %MicrophoneGltf

var right_control_map_actions = {
	"ax_button": "ia_primary",
	"by_button": "ia_secondary",
	"trigger_click": "ia_pointer",
	"grip_click": "ia_record_mic",
	"primary_click": "ia_open_emote_wheel",
}

var left_control_map_actions = {
	"ax_button": "ia_action_3",
	"by_button": "ia_action_4",
	"grip_click": "ia_action_5",
	"primary_click": "ia_action_6",
}

func _ready():
	prints("Starts XRPlayer")
	
	var xr_interface = XRServer.find_interface('OpenXR')
	if xr_interface != null:
		xr_interface.pose_recentered.connect(self.pose_recentered)
		
	await get_tree().process_frame
	pose_recentered()

func pose_recentered():
	XRServer.center_on_hmd(XRServer.RESET_BUT_KEEP_TILT, true)

func _process(delta):
	# Skip if no camera to track
	if !xr_camera:
		return

		# Get the target Y rotation (camera's Y rotation)
	var target_rotation_y = xr_camera.rotation.y
	
	# Get the current Y rotation of the object
	var current_rotation_y = ui_origin_3d.rotation.y
	
	# Calculate the difference in Y rotation
	var difference = target_rotation_y - current_rotation_y
	
	# Wrap the angle difference to the range [-PI, PI] for smooth interpolation
	difference = fmod(difference + PI, 2 * PI) - PI
	
	# Calculate the interpolation factor
	var t = min(1, delta * follow_speed)
	
	# Interpolate based on the curve
	var interpolated_t = follow_curve.sample_baked(t)
	
	# Update the object's Y rotation
	ui_origin_3d.rotation.y = current_rotation_y + difference * interpolated_t


func _on_right_hand_button_pressed(xr_action_name):
	var action = right_control_map_actions.get(xr_action_name)
	if action != null:
		Input.action_press(action)
		
		if action == "ia_record_mic":
			if not Global.comms.is_voice_chat_enabled():
				Global.async_create_popup_warning(
					PopupWarning.WarningType.WARNING, "Voice Chat issue", "Realm doesn't support voice chat."
				)
				return
			microphone_gltf.show()


func _on_right_hand_button_released(xr_action_name):
	var action = right_control_map_actions.get(xr_action_name)
	if action != null:
		Input.action_release(action)
		
		if action == "ia_record_mic":
			microphone_gltf.hide()


func _on_left_hand_button_pressed(xr_action_name):
	var action = left_control_map_actions.get(xr_action_name)
	if action != null:
		Input.action_press(action)


func _on_left_hand_button_released(xr_action_name):
	var action = left_control_map_actions.get(xr_action_name)
	if action != null:
		Input.action_release(action)
