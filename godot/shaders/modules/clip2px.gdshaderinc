#ifndef INC_CLIP_TO_PIXEL
#define INC_CLIP_TO_PIXEL

vec2 clipToPixel(vec4 clip, vec2 view_size)
{
    vec2 ndc = vec2(clip.x, clip.y) / clip.w;
    ndc = (ndc + 1.0) / 2.0;
    return ndc * view_size; 
}

#endif
