extends MarginContainer

signal link_clicked(url: String)

@onready var profile_about: MarginContainer = %ProfileAbout
@onready var profile_links: VBoxContainer = %ProfileLinks


func _ready() -> void:
	profile_links.link_clicked.connect(func(url): link_clicked.emit(url))


func refresh(profile: DclUserProfile) -> void:
	profile_about.refresh(profile)
	profile_links.refresh(profile)
