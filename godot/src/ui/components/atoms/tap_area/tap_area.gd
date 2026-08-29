@tool
class_name TapArea
extends Control

## Padding extra de área tapeable por lado, en px. El tamaño visual del botón NO cambia;
## sólo crece la zona de hit. Seteás sólo los lados que necesites (top/bottom, o laterales).
## Sirve para cualquier botón basado en Control (Button, TextureButton, ...).
## En el editor dibuja un rectángulo amarillo mostrando el área resultante.
@export_group("Tap growth (px)")
@export var grow_left: float = 0.0:
	set(value):
		grow_left = value
		queue_redraw()
@export var grow_top: float = 0.0:
	set(value):
		grow_top = value
		queue_redraw()
@export var grow_right: float = 0.0:
	set(value):
		grow_right = value
		queue_redraw()
@export var grow_bottom: float = 0.0:
	set(value):
		grow_bottom = value
		queue_redraw()


func _ready() -> void:
	# Keep the editor overlay in sync when the button is resized.
	if Engine.is_editor_hint() and not resized.is_connected(queue_redraw):
		resized.connect(queue_redraw)


func _tap_rect() -> Rect2:
	return Rect2(Vector2.ZERO, size).grow_individual(grow_left, grow_top, grow_right, grow_bottom)


func _has_point(point: Vector2) -> bool:
	return _tap_rect().has_point(point)


func _draw() -> void:
	if not Engine.is_editor_hint():
		return
	var rect: Rect2 = _tap_rect()
	var yellow: Color = Color(1.0, 0.85, 0.0)
	draw_rect(rect, Color(yellow, 0.08), true)  # relleno tenue
	draw_rect(rect, Color(yellow, 0.9), false, 1.0)  # contorno
