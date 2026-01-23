extends PanelContainer

@onready var label_download_warning: Label = %Label_DownloadWarning


func set_warning_text(text: String):
	label_download_warning.text = text
