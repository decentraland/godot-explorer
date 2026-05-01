class_name CategoriesBar
extends HBoxContainer

const CATEGORY_TAG_SCENE = preload("res://src/ui/components/discover/categories/category_tag.tscn")


func set_categories(categories: Array) -> void:
	for category in categories:
		if category is String:
			var category_tag: CategoryTag = CATEGORY_TAG_SCENE.instantiate()
			add_child(category_tag)
			category_tag.set_category(category)
