#ifndef INC_NUMBER_PX
#define INC_NUMBER_PX

const int DATA_BITS[10] = {
	0xEBFB,0xFFFC,
	0xA89B,0x21B4,
	0xABBF,0xF9FC,
	0xAA12,0x69A4,
	0xEBF3,0xF9E5
};

const int po10[] = {1, 10, 100, 1000, 10000, 100000, 1000000, 10000000, 100000000, 1000000000, 10000000000};

float DrawDigit(ivec2 px,  int digit)
{
    if (px.x < 0 || px.x > 2 || px.y < 0 || px.y > 4)
        return 0.0; // pixel out of bounds
    int xId = (digit == -1) ? 18 : 31 - (3 * digit + px.x);
    return float(0 != (DATA_BITS[(4 - px.y) * 2 + int(xId < 16)] & (1 << (xId % 16))));
}

// indexed like: XXX.0123
void GetDecimalSymbolAt( float v, const int i, const int decimalCount, out int symbol, out float opacity)
{
    // hide if outside the decimal range
    if (i > min(decimalCount - 1, 6))
    {
        symbol = 0;
        opacity = 0.0;
        return;
    }
    // get the i:th decimal
    float scale = float(po10[i + 1]);
    float scaledF = abs(v) * scale;
    symbol = int(scaledF) % 10;

    // fade trailing zeroes
    opacity = (fract(scaledF / 10.0) != 0.0) ? 1.0 : 0.5;
}

// indexed like: 210.XXX
void GetIntSymbolAt( float v, int i, out int symbol, out float opacity)
{
    // don't render more than 9 digits
    if (i <= 9)
    {
         int scale = po10[i];
         float vAbs = abs(v);
        // digits
        if (vAbs >= float(scale))
        {
             int it = int(floor(vAbs));
             int rem = it / scale;
            symbol = rem % 10;
            opacity = 1.0;
            return;
        }
        // minus symbol
        if ((v < 0.0) && (vAbs * 10.0 >= float(scale)))
        {
            symbol = -1;
            opacity = 1.0;
            return;
        }
    }
    // leading zeroes
    symbol = 0;
    opacity = 0.0;
}

// Get the digit at the given index of a floating point number
// with -45.78, then with a given dIndex:
// [-3] = - (digit -1)
// [-2] = 4
// [-1] = 5
// [ 0] = . (digit 10)
// [ 1] = 7
// [ 2] = 8
void GetSymbolAtPositionInFloat(float number, int dIndex, int decimalCount, out int symbol, out float opacity)
{
    opacity = 1.0;
    if (dIndex == 0)
        symbol = 10; // period
    else if (dIndex > 0)
        GetDecimalSymbolAt(number, dIndex - 1, decimalCount, symbol, opacity);
    else
        GetIntSymbolAt(number, -dIndex - 1, symbol, opacity);
}

// Given a pixel coordinate pxCoord, draws a number at pxPos
float DrawNumberAtPxPos(vec2 pxCoord, vec2 pxPos, float number, float fontScale, int decimalCount)
{
    ivec2 p = ivec2(floor((pxCoord - pxPos) / fontScale));
    // p.y += 0; // 0 = bottom aligned, 2 = vert. center aligned, 5 = top aligned
    // p.x += 0; // 0 = integers are directly to the left, decimal separator and decimals, to the right
    if (p.y < 0 || p.y > 4)
        return 0.0; // out of bounds vertically
    // shift placement to make it tighter around the decimal separator
    int shift = 0;
    if (p.x > 1) // decimal digits
        p.x += 1;
    else if (p.x < 0) // integer digits
    {
        p.x += -1;
    }

    int SEP = 4; // separation between characters
    int dIndex = int(floor(float(p.x) / float(SEP))); // the digit index to read
    float opacity;
    int digit;
    GetSymbolAtPositionInFloat(number, dIndex, decimalCount, /*out*/ digit, /*out*/ opacity);

    ivec2 pos = ivec2(dIndex * SEP + shift, 0);
    return opacity * DrawDigit(p - pos, digit);
}

/* // btw this might not work on all platforms, it might be Y-flipped or whatever! */
/* vec2 ClipToPixel(vec4 clip) */
/* { */
/*     vec2 ndc = vec2(clip.x, -clip.y) / clip.w; */
/*     ndc = (ndc + 1.0) / 2.0; */
/*     return ndc * _ScreenParams.xy; */
/* } */

/* vec2 LocalToPixel(vec3 locPos) { return ClipToPixel(UnityObjectToClipPos(vec4(locPos, 1))); } */
/* vec2 WorldToPixel(vec3 wPos) { return ClipToPixel(UnityWorldToClipPos(vec4(wPos, 1))); } */

/* float DrawNumberAtLocalPos(vec2 pxCoord, vec3 localPos, float number, float scale = 2, int decimalCount = 3) */
/* { */
/*      vec2 pxPos = LocalToPixel(localPos); */
/*     return DrawNumberAtPxPos(pxCoord, pxPos, number, scale, decimalCount); */
/* } */

/* float DrawNumberAtWorldPos(vec2 pxCoord, vec3 worldPos, float number, float scale = 2, int decimalCount = 3) */
/* { */
/*      vec2 pxPos = WorldToPixel(worldPos); */
/*     return DrawNumberAtPxPos(pxCoord, pxPos, number, scale, decimalCount); */
/* } */



#endif
