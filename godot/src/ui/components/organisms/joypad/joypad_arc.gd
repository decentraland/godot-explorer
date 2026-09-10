@tool
class_name JoypadArc
extends Control

## Procedural arc placement for the joypad's satellite buttons, driven by diameters (the values
## the designer controls) instead of hand-authored per-count offsets.
##
## The operation reverse-engineered from the old hardcoded LAYOUTS is a polar arc around the
## corner-anchored Button_Press: the satellites orbit its center at a fixed radius, the FIRST
## sits tangent to the joypad's bottom edge and the LAST tangent to the right edge, with the
## rest spread evenly between. Because Button_Press is anchored to the bottom-right corner, its
## center is always `press_diameter / 2` from both edges, so the endpoint angles fall out of the
## three diameters alone — no resolution dependence, nothing to tune per button count.
##
## Runtime: joypad.gd calls `arrange(visible_arc_nodes)` (satellites first, "+" last). In-editor
## (@tool) it previews with its own visible children so the arc updates live as you edit the
## exports.

## Diameter of the central Button_Press (its center sits this/2 from the joypad's corner edges).
@export var press_diameter: float = 156.0:
	set(value):
		press_diameter = value
		_request_preview()
## Diameter of each satellite orb button.
@export var satellite_diameter: float = 80.0:
	set(value):
		satellite_diameter = value
		_request_preview()
## Center-to-center distance from a satellite to Button_Press (the orbit radius). The gap between
## the two circles is therefore center_distance - (press_diameter + satellite_diameter) / 2.
@export var center_distance: float = 167.0:
	set(value):
		center_distance = value
		_request_preview()
## Where a lone satellite sits along the arc: 0 = bottom endpoint, 1 = right endpoint, 0.5 = the
## midpoint (up-left diagonal). Only used when exactly one node is placed.
@export_range(0.0, 1.0) var single_node_t: float = 0.5:
	set(value):
		single_node_t = value
		_request_preview()
## Button count at which the arc reaches the right edge. With fewer buttons they keep this same
## angular spacing anchored at the bottom (e.g. with 2 the second lands mid-arc, not at the right
## edge); with more they compress evenly to fit between the two edges. Matches the old feel where
## 1 -> bottom, 2 -> bottom + ~diagonal, 3 -> bottom + diagonal + right.
@export var full_arc_count: int = 3:
	set(value):
		full_arc_count = max(value, 2)
		_request_preview()
## Live editor preview. Turn off to freeze positions while hand-nudging in the editor.
@export var preview_in_editor: bool = true


func _process(_delta: float) -> void:
	if Engine.is_editor_hint() and preview_in_editor:
		arrange(_visible_arc_children())


## Positions `nodes` (ordered: first -> bottom edge, last -> right edge) on the arc.
func arrange(nodes: Array) -> void:
	var count: int = nodes.size()
	if count == 0:
		return
	var press: Control = get_parent() as Control
	if press == null:
		return

	var press_r: float = press_diameter * 0.5
	var sat_r: float = satellite_diameter * 0.5
	# Offset of an endpoint node's center toward its tangent edge: the node touches the edge
	# that Button_Press already touches, so its center is press_r - sat_r inside that edge.
	var edge: float = press_r - sat_r
	var span: float = sqrt(max(center_distance * center_distance - edge * edge, 0.0))
	var theta_first: float = atan2(edge, -span)  # down and to the left  (bottom edge)
	var theta_last: float = atan2(-span, edge)  # up and to the right   (right edge)
	# Sweep counter-clockwise (through the upper-left), never the short way round.
	if theta_last < theta_first:
		theta_last += TAU

	# Fixed angular step anchored at the bottom: the arc is "full" (last node on the right edge)
	# at full_arc_count. Fewer nodes keep this step (so they don't stretch across the whole arc);
	# more nodes shrink the step to stay within the two edges.
	var slots: int = max(count, full_arc_count)
	var step: float = (theta_last - theta_first) / float(slots - 1)

	var center: Vector2 = press.global_position + press.size * 0.5 - global_position
	for i in count:
		var node: Control = nodes[i] as Control
		if node == null:
			continue
		var theta: float = (
			lerp(theta_first, theta_last, single_node_t)
			if count == 1
			else theta_first + step * float(i)
		)
		var node_center: Vector2 = center + Vector2(cos(theta), sin(theta)) * center_distance
		node.position = node_center - node.size * 0.5


## Visible, non-"+" children in tree order, with Button_Combo forced last (it's the topmost /
## right-edge arc element whenever it's shown).
func _visible_arc_children() -> Array:
	var nodes: Array = []
	var combo: Control = null
	for child in get_children():
		var control: Control = child as Control
		if control == null or not control.visible:
			continue
		if control.name == "Button_Combo":
			combo = control
		else:
			nodes.append(control)
	if combo != null:
		nodes.append(combo)
	return nodes


func _request_preview() -> void:
	if Engine.is_editor_hint() and is_node_ready() and preview_in_editor:
		arrange(_visible_arc_children())
