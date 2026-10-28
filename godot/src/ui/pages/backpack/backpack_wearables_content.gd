class_name BackpackWearablesContent
extends MarginContainer

## WearableEditor + EmoteEditor, split out of backpack_portrait.tscn into their own scene
## (#1935) so Godot can edit them normally — they used to live as raw instance overrides
## reparented inside the draggable sheet, which the editor's "Editable Children" couldn't
## reliably round-trip. backpack.gd can't reach these via %unique-name (a nested scene
## instance's internal nodes keep their own owner, not the outer scene's), so it reads them
## through this script instead — see backpack.gd's _ready().

@onready var wearable_editor: PanelContainer = %WearableEditor
@onready var emote_editor = %EmoteEditor
@onready var color_carrousel = %ColorCarrousel
@onready var carrousel_separator: HSeparator = %CarrouselSeparator
@onready var grid_container_wearables_list: GridContainer = %GridContainer_WearablesList
@onready var container_main_categories: HBoxContainer = %HBoxContainer_MainCategories
@onready var container_sub_categories: HBoxContainer = %HBoxContainer_SubCategories
@onready var vboxcontainer_wearable_selector: VBoxContainer = %VBoxContainer_WearableSelector
@onready var subcategories_container: HBoxContainer = %SubcategoriesContainer
@onready var subcategories_separator: HSeparator = %SubcategoriesSeparator
@onready var maincategories_container: MarginContainer = %MainCategoriesContainer
@onready var scroll_container_items: ScrollContainer = %ScrollContainer_Items
@onready var hseparator_extra_space: HSeparator = %HSeparator_ExtraSpace
@onready var hseparator_extra_space_b: HSeparator = %HSeparator_ExtraSpaceB
@onready var hseparator_size_maintainer: HSeparator = get_node_or_null("%HSeparator_SizeMaintainer")
@onready var margin_container_no_items: MarginContainer = %MarginContainer_NoItems
