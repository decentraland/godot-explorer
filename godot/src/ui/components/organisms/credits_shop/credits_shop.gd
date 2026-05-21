extends VBoxContainer

@onready var credits_options_skeleton: VBoxContainer = %CreditsOptions_Skeleton
@onready var credits_faq_skeleton: VBoxContainer = %CreditsFaq_Skeleton
@onready var credits_option_inner = %CreditsOptionInner
@onready var credits_faq: VBoxContainer = %CreditsFaq


func _ready() -> void:
	if Iap.get_products().size() > 0:
		_show_content()
	else:
		Iap.products_ready.connect(_on_products_ready)


func _on_products_ready(_products: Array) -> void:
	_show_content()


func _show_content() -> void:
	credits_options_skeleton.hide()
	credits_faq_skeleton.hide()
	credits_option_inner.show()
	credits_faq.show()


func reset() -> void:
	credits_options_skeleton.show()
	credits_faq_skeleton.show()
	credits_option_inner.hide()
	credits_faq.hide()
