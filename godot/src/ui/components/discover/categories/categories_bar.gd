class_name CategoriesBar
extends HBoxContainer

const category_tag_scene = preload("res://src/ui/components/discover/categories/category_tag.tscn")


func set_categories(categories:Array) -> void:
	for category in categories:
		if category is String:
			var category_tag:CategoryTag = category_tag_scene.instantiate()
			add_child(category_tag)
			category_tag.set_category(category)
