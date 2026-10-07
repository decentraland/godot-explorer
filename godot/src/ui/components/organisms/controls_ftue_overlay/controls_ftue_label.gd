class_name ControlsFtueLabel
extends MarginContainer

enum PointerSide { LEFT, TOP, BOTTOM }

const FILL := Color(0.412, 0.122, 0.663, 0.8)
const CORNER_RADIUS := 8.0
const CORNER_STEPS := 4
const POINTER_LENGTH := 10.0
const POINTER_HALF_WIDTH := 10.0

@export var pointer_side := PointerSide.LEFT

## Slides the pointer along its side, away from the middle, so the body can sit off-centre
## from what it points at.
var pointer_offset := 0.0:
	set(value):
		pointer_offset = value
		queue_redraw()


func _ready() -> void:
	resized.connect(queue_redraw)


# Body and pointer are one polygon: two overlapping translucent shapes would blend darker
# where they meet.
func _draw() -> void:
	draw_colored_polygon(_outline(), FILL)


## How far the pointer can slide before it runs into a rounded corner.
func max_pointer_offset() -> float:
	var side_length := size.y if pointer_side == PointerSide.LEFT else size.x
	return maxf(0.0, side_length * 0.5 - CORNER_RADIUS - POINTER_HALF_WIDTH)


func _outline() -> PackedVector2Array:
	var w := size.x
	var h := size.y
	var along_x := w * 0.5 + pointer_offset
	var along_y := h * 0.5 + pointer_offset
	var r := CORNER_RADIUS
	var points := PackedVector2Array()
	_append_corner(points, Vector2(r, r), PI)
	if pointer_side == PointerSide.TOP:
		points.append(Vector2(along_x - POINTER_HALF_WIDTH, 0.0))
		points.append(Vector2(along_x, -POINTER_LENGTH))
		points.append(Vector2(along_x + POINTER_HALF_WIDTH, 0.0))
	_append_corner(points, Vector2(w - r, r), PI * 1.5)
	_append_corner(points, Vector2(w - r, h - r), 0.0)
	if pointer_side == PointerSide.BOTTOM:
		points.append(Vector2(along_x + POINTER_HALF_WIDTH, h))
		points.append(Vector2(along_x, h + POINTER_LENGTH))
		points.append(Vector2(along_x - POINTER_HALF_WIDTH, h))
	_append_corner(points, Vector2(r, h - r), PI * 0.5)
	if pointer_side == PointerSide.LEFT:
		points.append(Vector2(0.0, along_y + POINTER_HALF_WIDTH))
		points.append(Vector2(-POINTER_LENGTH, along_y))
		points.append(Vector2(0.0, along_y - POINTER_HALF_WIDTH))
	return points


func _append_corner(points: PackedVector2Array, center: Vector2, from_angle: float) -> void:
	for i in range(CORNER_STEPS + 1):
		var angle := from_angle + PI * 0.5 * float(i) / float(CORNER_STEPS)
		points.append(center + Vector2(cos(angle), sin(angle)) * CORNER_RADIUS)
