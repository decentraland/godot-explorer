class_name ControlsFtueLabel
extends MarginContainer

enum PointerSide { LEFT, TOP, BOTTOM, NONE }

const FILL := Color(0.412, 0.122, 0.663, 0.8)
const CORNER_RADIUS := 8.0
const CORNER_STEPS := 4
const POINTER_LENGTH := 10.0
const POINTER_HALF_WIDTH := 10.0

@export var pointer_side := PointerSide.LEFT


func _ready() -> void:
	resized.connect(queue_redraw)


# Body and pointer are one polygon: two overlapping translucent shapes would blend darker
# where they meet.
func _draw() -> void:
	draw_colored_polygon(_outline(), FILL)


func _outline() -> PackedVector2Array:
	var w := size.x
	var h := size.y
	var r := CORNER_RADIUS
	var points := PackedVector2Array()
	_append_corner(points, Vector2(r, r), PI)
	if pointer_side == PointerSide.TOP:
		points.append(Vector2(w * 0.5 - POINTER_HALF_WIDTH, 0.0))
		points.append(Vector2(w * 0.5, -POINTER_LENGTH))
		points.append(Vector2(w * 0.5 + POINTER_HALF_WIDTH, 0.0))
	_append_corner(points, Vector2(w - r, r), PI * 1.5)
	_append_corner(points, Vector2(w - r, h - r), 0.0)
	if pointer_side == PointerSide.BOTTOM:
		points.append(Vector2(w * 0.5 + POINTER_HALF_WIDTH, h))
		points.append(Vector2(w * 0.5, h + POINTER_LENGTH))
		points.append(Vector2(w * 0.5 - POINTER_HALF_WIDTH, h))
	_append_corner(points, Vector2(r, h - r), PI * 0.5)
	if pointer_side == PointerSide.LEFT:
		points.append(Vector2(0.0, h * 0.5 + POINTER_HALF_WIDTH))
		points.append(Vector2(-POINTER_LENGTH, h * 0.5))
		points.append(Vector2(0.0, h * 0.5 - POINTER_HALF_WIDTH))
	return points


func _append_corner(points: PackedVector2Array, center: Vector2, from_angle: float) -> void:
	for i in range(CORNER_STEPS + 1):
		var angle := from_angle + PI * 0.5 * float(i) / float(CORNER_STEPS)
		points.append(center + Vector2(cos(angle), sin(angle)) * CORNER_RADIUS)
