import { hexToHsv, hsvToHex } from '../src/ds/colorMath';

describe('colorMath', () => {
  it('round-trips colors', () => {
    for (const hex of [
      '#ff0000',
      '#00ff00',
      '#3b2412',
      '#ffdcb6',
      '#000000',
      '#ffffff',
    ]) {
      expect(hsvToHex(hexToHsv(hex))).toBe(hex);
    }
  });

  it('treats invalid input as black and clamps components', () => {
    expect(hexToHsv('nope')).toEqual({ h: 0, s: 0, v: 0 });
    expect(hsvToHex({ h: 400, s: 2, v: -1 })).toBe('#000000');
  });
});
