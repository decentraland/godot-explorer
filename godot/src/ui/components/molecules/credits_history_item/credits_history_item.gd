extends PanelContainer

@onready var label_date: Label = %Label_Date
@onready var label_detail: Label = %Label_Detail
@onready var label_amount: Label = %Label_Amount


func setup(credits: int, is_refund: bool, timestamp: String) -> void:
	label_date.text = timestamp
	if is_refund:
		label_detail.text = "Refund Deduction"
		label_amount.text = "-%d" % credits
	else:
		label_detail.text = "Purchased %d CREDITS" % credits
		label_amount.text = str(credits)
