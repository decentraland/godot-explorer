# Bell Animation Spritesheet Migration Guide

This document describes how we migrated the notification bell toggle animation from Unity to Godot using spritesheets.

## Overview

Unity uses a SpriteAtlas system for frame-by-frame animations. We converted this to Godot's SpriteFrames resource system, consolidating multiple PNG frames into a single optimized spritesheet.

## Source Materials (Unity)

**Location in Unity project:**
```
~/Projects/decentraland/unity-explorer/Explorer/Assets/Textures/ExplorePanel/
├── NotificationsHover/              # Folder with 63 animation frames
│   ├── NotificationIcon_00000.png   # Frame 0 (first animation frame)
│   ├── NotificationIcon_00001.png
│   ├── ...
│   └── NotificationIcon_00062.png   # Frame 62 (last animation frame)
├── NotificationsHoverSpriteAtlas.spriteatlasv2  # Unity's sprite atlas config
└── NotificationBell.png             # Static white bell icon (inactive state)
```

**Frame specifications:**
- **Dimensions:** 70x70 pixels each
- **Format:** PNG with RGBA (transparency)
- **Total frames:** 63 animation frames
- **Animation duration:** 0.5 seconds

## Migration Process

### Step 1: Extract and Prepare Frames

1. **Resize the static white bell icon to match frame size:**
   ```bash
   cd godot/assets/ui/notifications
   cp ../NotificationBell.png bell_icon_white.png
   magick bell_icon_white.png -resize 70x70 -background transparent \
     -gravity center -extent 70x70 bell_icon_white_70.png
   ```

2. **Copy animation frames from Unity:**
   ```bash
   cp ~/Projects/decentraland/unity-explorer/Explorer/Assets/Textures/ExplorePanel/NotificationsHover/NotificationIcon_*.png ./
   ```

### Step 2: Create Optimized Spritesheet

Combine all frames into a single spritesheet using ImageMagick:

```bash
magick montage bell_icon_white_70.png \
  ~/Projects/decentraland/unity-explorer/Explorer/Assets/Textures/ExplorePanel/NotificationsHover/NotificationIcon_*.png \
  -tile 8x8 -geometry 70x70+0+0 -background transparent \
  bell_animation_spritesheet.png
```

**Result:**
- **Dimensions:** 560x560 pixels (8 columns × 8 rows)
- **Total frames:** 64 (1 white bell + 63 animation frames)
- **File size:** ~74KB (compressed from multiple individual files)

**Frame layout:**
```
Frame 0: White bell icon (inactive state)
Frames 1-63: Animation sequence (shake and fill effect)
```

### Step 3: Create SpriteFrames Resource

Create a `.tres` resource file that defines all frames as AtlasTextures:

```gdscript
[gd_resource type="SpriteFrames" format=3]

[ext_resource type="Texture2D" path="res://assets/ui/notifications/bell_animation_spritesheet.png" id="1"]

# Define 64 AtlasTexture subresources (one per frame)
[sub_resource type="AtlasTexture" id="atlas_0"]
atlas = ExtResource("1")
region = Rect2(0, 0, 70, 70)

[sub_resource type="AtlasTexture" id="atlas_1"]
atlas = ExtResource("1")
region = Rect2(70, 0, 70, 70)

# ... (repeat for all 64 frames) ...

[resource]
animations = [{
"frames": [
  {"duration": 1.0, "texture": SubResource("atlas_0")},
  {"duration": 1.0, "texture": SubResource("atlas_1")},
  # ... (all 64 frames) ...
],
"loop": false,
"name": &"toggle",
"speed": 128.0  # 64 frames / 0.5 seconds = 128 FPS
}]
```

**Animation settings:**
- **Speed:** 128 FPS (makes 64 frames play in exactly 0.5 seconds)
- **Loop:** false (animation stops at last frame)
- **Name:** "toggle"

### Step 4: Implement in Scene

**Scene file (`notification_bell_button.tscn`):**
```gdscript
[ext_resource type="SpriteFrames" path="res://assets/ui/notifications/bell_animation.tres" id="2_bell_animation"]

[node name="BellAnimatedSprite" type="AnimatedSprite2D" parent="."]
unique_name_in_owner = true
position = Vector2(30, 30)
scale = Vector2(0.7, 0.7)
sprite_frames = ExtResource("2_bell_animation")
animation = &"toggle"
```

**Script file (`notification_bell_button.gd`):**
```gdscript
@onready var bell_sprite: AnimatedSprite2D = %BellAnimatedSprite

func _update_button_state() -> void:
	if bell_sprite == null:
		return

	if _is_panel_open:
		# Play animation forward (inactive -> active)
		bell_sprite.play("toggle")
	else:
		# Play animation backward (active -> inactive)
		bell_sprite.play_backwards("toggle")
```

## Animation Behavior

- **Inactive state:** Shows frame 0 (white bell icon)
- **Click to open:** Plays frames 0→63 in 0.5 seconds (shake and fill effect)
- **Click to close:** Plays frames 63→0 in 0.5 seconds (reverse animation)

## Future Migrations

To migrate other Unity sprite animations (chat button, sidebar icons, etc.):

1. **Locate the Unity SpriteAtlas:**
   - Look in `unity-explorer/Explorer/Assets/Textures/` for `.spriteatlasv2` files
   - Find the corresponding folder with individual frame PNGs

2. **Identify the static icon:**
   - Determine which icon should be the "inactive" or default state
   - Resize it to match the animation frame dimensions

3. **Create spritesheet:**
   ```bash
   magick montage <static_icon> <animation_frames>/*.png \
     -tile <cols>x<rows> -geometry <width>x<height>+0+0 \
     -background transparent output_spritesheet.png
   ```

   **Tip:** Calculate tile size based on total frames:
   - 64 frames = 8×8 grid
   - 100 frames = 10×10 grid
   - 128 frames = 11×12 or 12×11 grid

4. **Calculate animation speed:**
   ```
   FPS = total_frames / desired_duration_in_seconds
   ```
   Example: 64 frames / 0.5s = 128 FPS

5. **Generate `.tres` resource:**
   - Create AtlasTexture subresources for each frame
   - Set proper region rectangles: `Rect2(col * width, row * height, width, height)`
   - Configure animation speed and loop settings

6. **Implement in scene:**
   - Use `AnimatedSprite2D` node
   - Reference the `.tres` resource
   - Control playback with `play()` and `play_backwards()`

## Benefits of This Approach

✅ **Performance:** Single texture lookup instead of loading multiple files
✅ **Memory:** Reduced overhead from texture management
✅ **Maintainability:** Animation settings stored in resource file, not code
✅ **Flexibility:** Easy to adjust FPS, add frames, or create variants
✅ **Hot-reload friendly:** Changes to `.tres` reload instantly in editor

## Tools Used

- **ImageMagick:** For creating spritesheets (`magick montage`)
- **Godot 4.x:** SpriteFrames resource and AnimatedSprite2D node
- **Text editor:** For generating `.tres` resource files

## Notes

- The Unity animation may use different FPS settings - test in Unity to match timing
- Some animations might need easing or different playback modes
- For very large spritesheets (>2048px), consider splitting into multiple atlases
- Always use transparent background for proper blending
