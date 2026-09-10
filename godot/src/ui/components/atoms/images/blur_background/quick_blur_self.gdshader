shader_type canvas_item;

// Twin of quick_blur.gdshader, but it blurs ONLY this node's own texture instead of the screen
// backbuffer: it samples TEXTURE/UV (the element itself) rather than a hint_screen_texture, so
// whatever is drawn behind the element stays sharp.
//
// Same cheap mipmap-LOD trick: a higher blur_amount reads a smaller (more downsampled) mip.
// For that to actually blur, the element's texture needs mipmaps:
//   - Texture import:  Mipmaps > Generate = On
//   - Node (e.g. TextureRect):  Texture Filter = "Linear Mipmap"
// Without both, textureLod clamps to LOD 0 and nothing blurs — if the texture can't have mipmaps,
// blur it instead with an explicit neighbour-sampling kernel (more GPU, no mipmaps needed).

uniform float blur_amount : hint_range(0.0, 8.0) = 2.0;

void fragment() {
	COLOR = textureLod(TEXTURE, UV, blur_amount);
}
