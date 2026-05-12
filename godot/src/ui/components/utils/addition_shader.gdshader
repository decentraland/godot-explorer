// Additive blend shader for canvas items.
// Brightens the node's own texture and adds it on top of whatever is behind it,
// producing a glow/bloom effect: bright areas of the node light up the background
// rather than replacing it.
shader_type canvas_item;

// Grab a copy of the screen content behind this node (with mipmapping for smooth sampling).
uniform sampler2D screen_texture : hint_screen_texture, filter_linear_mipmap;

void fragment() {
	// Sample the screen pixels visible behind this node.
	vec4 color = texture(screen_texture, SCREEN_UV);
	// Double the node's own colour so the additive contribution is stronger.
	COLOR.rgb *= 2.0;
	// Add the brightened node colour on top of the background — pure additive blending.
	// Black pixels (0,0,0) from the node contribute nothing; bright pixels push the
	// result toward white, giving a natural glow without darkening anything.
	COLOR.rgb += color.rgb;
}