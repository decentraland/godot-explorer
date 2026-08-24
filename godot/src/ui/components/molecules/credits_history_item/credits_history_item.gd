extends PanelContainer

@onready var label_date: Label = %Label_Date
@onready var label_detail: Label = %Label_Detail
@onready var label_amount: Label = %Label_Amount


func setup(credits: int, is_refund: bool, timestamp: String) -> void:
	label_date.text = timestamp
	if is_refund:
		label_detail.text = tr("CREDITS_HISTORY_ITEM_REFUND_DEDUCTION")
		label_amount.text = tr("CREDITS_AMOUNT_DEDUCTED") % credits
	else:
		label_detail.text = (TranslationKey.new("CREDITS_PURCHASED").plural(credits))
		label_amount.text = str(credits)
