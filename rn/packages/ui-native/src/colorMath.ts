export type Hsv = { h: number; s: number; v: number };

const clamp = (value: number, min: number, max: number) =>
  Math.min(max, Math.max(min, value));

/** `#rrggbb` → HSV with h in [0,360), s and v in [0,1]. Invalid input yields black. */
export function hexToHsv(hex: string): Hsv {
  const match = /^#?([0-9a-f]{6})$/i.exec(hex.trim());
  if (!match) {
    return { h: 0, s: 0, v: 0 };
  }
  const [r, g, b] = [0, 2, 4].map(
    o => parseInt(match[1].slice(o, o + 2), 16) / 255,
  );
  const max = Math.max(r, g, b);
  const delta = max - Math.min(r, g, b);
  let h = 0;
  if (delta !== 0) {
    if (max === r) {
      h = ((g - b) / delta) % 6;
    } else if (max === g) {
      h = (b - r) / delta + 2;
    } else {
      h = (r - g) / delta + 4;
    }
    h = (h * 60 + 360) % 360;
  }
  return { h, s: max === 0 ? 0 : delta / max, v: max };
}

export function hsvToHex({ h, s, v }: Hsv): string {
  const hue = ((h % 360) + 360) % 360;
  const sat = clamp(s, 0, 1);
  const val = clamp(v, 0, 1);
  const c = val * sat;
  const x = c * (1 - Math.abs(((hue / 60) % 2) - 1));
  const m = val - c;
  const [r, g, b] =
    hue < 60
      ? [c, x, 0]
      : hue < 120
      ? [x, c, 0]
      : hue < 180
      ? [0, c, x]
      : hue < 240
      ? [0, x, c]
      : hue < 300
      ? [x, 0, c]
      : [c, 0, x];
  const byte = (channel: number) =>
    Math.round((channel + m) * 255)
      .toString(16)
      .padStart(2, '0');
  return `#${byte(r)}${byte(g)}${byte(b)}`;
}
