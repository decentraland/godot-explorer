import { showsFps } from '../src/hud/VersionFps';

describe('version line', () => {
  it('shows the frame rate only for builds that are not a plain release', () => {
    expect(showsFps('2.0.0')).toBe(false);
    expect(showsFps('1.15.0-cb9ce77-debug-dev')).toBe(true);
  });
});
