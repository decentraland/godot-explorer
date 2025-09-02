extends PanelContainer

signal submit_message(message: String)
signal hide_parcel_info
signal show_parcel_info
signal release_mouse

const EMOTE: String = "␐"
const REQUEST_PING: String = "␑"
const ACK: String = "␆"
const CHAT_MESSAGE = preload("res://src/ui/components/chat/chat_message.tscn")

var hide_tween = null
var open_tween = null
var close_tween = null
var nearby_avatars = null
var is_open: bool = false

@onready var h_box_container_line_edit = %HBoxContainer_LineEdit
@onready var line_edit_command = %LineEdit_Command
@onready var button_nearby_users: Button = %Button_NearbyUsers
@onready var label_members_quantity: Label = %Label_MembersQuantity
@onready var margin_container_chat: MarginContainer = %MarginContainer_Chat
@onready var button_back: Button = %Button_Back
@onready var texture_rect_logo: TextureRect = %TextureRect_Logo
@onready var h_box_container_nearby_users: HBoxContainer = %HBoxContainer_NearbyUsers
@onready var timer_hide = %Timer_Hide
@onready var v_box_container_chat: VBoxContainer = %VBoxContainerChat
@onready var scroll_container_chats_list: ScrollContainer = %ScrollContainer_ChatsList
@onready var avatars_list: Control = %AvatarsList
@onready var panel_container_navbar: PanelContainer = %PanelContainer_Navbar
@onready var v_box_container_content: VBoxContainer = %VBoxContainer_Content
@onready var panel_container_notification: PanelContainer = %PanelContainer_Notification
@onready var v_box_container_notifications: VBoxContainer = %VBoxContainerNotifications
@onready var timer_delete_notifications: Timer = %Timer_DeleteNotifications


func _ready():
	_on_button_back_pressed()
	avatars_list.async_update_nearby_users(Global.avatars.get_avatars())

	# Connect to avatar scene changed signal instead of using timer
	Global.avatars.avatar_scene_changed.connect(avatars_list.async_update_nearby_users)
	avatars_list.size_changed.connect(self.update_nearby_quantity)

	Global.comms.chat_message.connect(self.on_chats_arrived)
	submit_message.connect(self._on_submit_message)
	
	show_notification()
	# Esperar a que el chat esté completamente inicializado antes de crear mensajes
	await_chat_ready.call_deferred()


func await_chat_ready():
	# Esperar múltiples frames para asegurar que el layout esté completamente listo
	await get_tree().process_frame
	await get_tree().process_frame
	await get_tree().process_frame
	
	# Verificar que todos los componentes necesarios estén listos
	if not v_box_container_chat or not scroll_container_chats_list:
		return
	
	# Forzar actualización del layout del contenedor
	v_box_container_chat.queue_redraw()
	scroll_container_chats_list.queue_redraw()
	await get_tree().process_frame
	
	# Crear el mensaje del sistema sin afectar el estado de notificación
	var system_message = ["system", Time.get_unix_time_from_system(), "Welcome to the Godot Client! Navigate to Advanced Settings > Realm tab to change the realm. Press Enter or click in the Talk button to say something to nearby."]
	
	# Crear solo el mensaje en el chat, sin notificación y sin cambiar la vista
	var new_chat = CHAT_MESSAGE.instantiate()
	v_box_container_chat.add_child(new_chat)
	new_chat.compact_view = Global.is_chat_compact
	new_chat.set_chat(system_message)
	
	# Ajustar el tamaño del mensaje del sistema
	await get_tree().process_frame
	await get_tree().process_frame
	if new_chat.is_inside_tree() and new_chat.get_parent():
		new_chat.async_adjust_panel_size.call_deferred()


func _on_submit_message(_message: String):
	UiSounds.play_sound("widget_chat_message_private_send")


func on_chats_arrived(chats: Array):
	var should_show_notification = not v_box_container_content.visible
	
	for i in range(chats.size()):
		var chat = chats[i]
		var is_last_message = (i == chats.size() - 1)
		create_chat(chat, should_show_notification and is_last_message)

	# Scroll to bottom after frame processing
	async_scroll_to_bottom.call_deferred()


func async_scroll_to_bottom() -> void:
	# Ensure scroll is at maximum (bottom)
	await get_tree().process_frame
	scroll_container_chats_list.scroll_vertical = (
		scroll_container_chats_list.get_v_scroll_bar().max_value
	)


func _on_button_send_pressed():
	submit_message.emit(line_edit_command.text)
	line_edit_command.text = ""


func _on_line_edit_command_text_submitted(new_text):
	submit_message.emit(new_text)
	line_edit_command.text = ""


func finish():
	if line_edit_command.text.size() > 0:
		submit_message.emit(line_edit_command.text)
		line_edit_command.text = ""


func toggle_chat_visibility(visibility:bool):
	_on_button_back_pressed()
	if visibility:
		UiSounds.play_sound("widget_chat_open")
		_tween_open()
	else:
		Global.explorer_grab_focus()
		UiSounds.play_sound("widget_chat_close")
		_tween_close()


func _tween_open() -> void:
	if open_tween != null:
		open_tween.stop()
	open_tween = get_tree().create_tween()
	v_box_container_content.show()
	open_tween.tween_property(self, "modulate", Color.WHITE, 0.5)
	is_open = true


func _tween_close() -> void:
	if close_tween != null:
		close_tween.stop()
	close_tween = get_tree().create_tween()
	close_tween.tween_property(self, "modulate", Color.TRANSPARENT, 0.5)
	v_box_container_content.hide()
	is_open = false


func update_nearby_quantity() -> void:
	button_nearby_users.text = str(avatars_list.list_size)
	label_members_quantity.text = str(avatars_list.list_size)


func _on_button_nearby_users_pressed() -> void:
	show_nearby_players()


func _on_button_back_pressed() -> void:
	show_chat()


func _on_line_edit_command_focus_entered() -> void:
	panel_container_navbar.show()
	emit_signal("hide_parcel_info")
	timer_hide.stop()


func _on_line_edit_command_focus_exited():
	emit_signal("show_parcel_info")
	timer_hide.start()


func _on_timer_hide_timeout() -> void:
	panel_container_navbar.hide()
	h_box_container_line_edit.hide()
	self_modulate = "#00000010"


func _on_gui_input(event: InputEvent) -> void:
	if event is InputEventScreenTouch or event is InputEventMouseButton:
		if margin_container_chat.visible:
			show_chat()


func show_chat() -> void:
	v_box_container_content.show()
	panel_container_notification.hide()
	self_modulate = "#00000040"
	avatars_list.hide()
	button_back.hide()
	h_box_container_line_edit.show()
	h_box_container_nearby_users.hide()
	margin_container_chat.show()
	panel_container_navbar.show()
	texture_rect_logo.show()
	button_nearby_users.show()
	timer_hide.start()
	
	# Ajustar el tamaño de todos los mensajes existentes cuando se muestra el chat por primera vez
	adjust_existing_messages.call_deferred()


func adjust_existing_messages() -> void:
	# Esperar a que el layout se actualice
	await get_tree().process_frame
	await get_tree().process_frame
	
	# Ajustar el tamaño de todos los mensajes de chat existentes
	for child in v_box_container_chat.get_children():
		if child.has_method("async_adjust_panel_size"):
			child.async_adjust_panel_size.call_deferred()
	
	
func show_nearby_players() -> void:
	v_box_container_content.show()
	panel_container_notification.hide()
	self_modulate = "#00000080"
	avatars_list.show()
	button_back.show()
	h_box_container_nearby_users.show()
	margin_container_chat.hide()
	texture_rect_logo.hide()
	button_nearby_users.hide()
	timer_hide.stop()


func show_notification() -> void:
	# Detener cualquier timer activo
	timer_hide.stop()
	
	# Asegurar que el panel de notificaciones esté completamente visible
	panel_container_notification.modulate = Color.WHITE
	panel_container_notification.show()
	
	# Ocultar el contenido del chat
	v_box_container_content.hide()
	
	self_modulate = "#00000000"
	
func create_chat(chat, should_create_notification = false) -> void:
	# Verificar que el contenedor esté listo
	if not v_box_container_chat or not is_inside_tree():
		print("Warning: Chat container not ready, deferring message creation")
		create_chat.call_deferred(chat, should_create_notification)
		return
	
	# No verificar el tamaño del contenedor si está oculto - simplemente crear el mensaje
	# El ajuste de tamaño se hará cuando sea necesario
	
	# Crear el mensaje principal en el chat
	var new_chat = CHAT_MESSAGE.instantiate()
	v_box_container_chat.add_child(new_chat)
	new_chat.compact_view = Global.is_chat_compact
	new_chat.set_chat(chat)
	
	# Crear notificación si es necesario
	if should_create_notification:
		create_notification(chat)
	
	# Esperar a que el mensaje esté completamente agregado al árbol
	await get_tree().process_frame
	await get_tree().process_frame
	
	# Solo ajustar el panel si el contenido del chat es visible
	# Si está oculto, el ajuste se hará cuando se muestre
	if new_chat.is_inside_tree() and v_box_container_content.visible:
		new_chat.async_adjust_panel_size.call_deferred()


func create_notification(chat) -> void:
	# Limpiar notificaciones anteriores
	clear_notifications()
	
	# Mostrar la vista de notificaciones ANTES de crear la notificación
	show_notification()
	
	# Esperar un frame para que el layout se estabilice
	await get_tree().process_frame
	
	# Crear nueva notificación
	var new_notification = CHAT_MESSAGE.instantiate()
	v_box_container_notifications.add_child(new_notification)
	new_notification.compact_view = true
	new_notification.set_chat(chat)
	
	# Esperar a que la notificación esté completamente agregada
	await get_tree().process_frame
	await get_tree().process_frame
	
	# Ajustar el tamaño de la notificación después de que esté estable
	if new_notification.is_inside_tree() and new_notification.get_parent():
		new_notification.async_adjust_panel_size.call_deferred()
	
	# Iniciar timer para ocultar la notificación
	timer_delete_notifications.start()


func clear_notifications() -> void:
	for child in v_box_container_notifications.get_children():
		child.queue_free()


func _on_timer_delete_notifications_timeout() -> void:
	# Animar fade out de la notificación
	var hide_notification_tween = get_tree().create_tween()
	hide_notification_tween.tween_property(panel_container_notification, "modulate", Color.TRANSPARENT, 0.5)
	
	# Después del fade out, limpiar notificaciones y ocultar el chat
	hide_notification_tween.tween_callback(func():
		clear_notifications()
		panel_container_notification.modulate = Color.WHITE  # Restaurar opacidad para próxima vez
		panel_container_notification.hide()  # Ocultar panel de notificaciones
		# No llamar a show_chat() - el chat permanece cerrado
	)
		
