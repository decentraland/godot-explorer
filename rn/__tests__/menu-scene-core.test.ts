import { setCoreForTests } from '../src/core';
import { createMockCore } from '../src/core/mock';
import type { CurrentScene } from '../src/core/native';
import { createCoreMenuApi } from '../src/menu/coreMenuApi';
import { createMockApi } from '../src/menu/mockApi';

const currentScene = jest.fn();
const captureScreenshot = jest.fn();

beforeEach(() => {
  currentScene.mockReset();
  setCoreForTests({ ...createMockCore(), currentScene, captureScreenshot });
});
afterEach(() => setCoreForTests(undefined));

const scene = (over: Partial<CurrentScene> = {}): CurrentScene => ({
  title: 'Plaza',
  creator: 'Ana',
  parcel: { x: 3, y: -4 },
  realm: 'https://realm.test/main',
  genesis: true,
  ...over,
});

describe('menu scene from the core', () => {
  it('builds the scene header from the current scene', async () => {
    currentScene.mockResolvedValue(scene());
    expect(await createCoreMenuApi(createMockApi()).scene()).toEqual({
      title: 'Plaza',
      creator: 'Ana',
      linkQuery: 'position=3,-4',
    });
  });

  it('links a world by its realm name', async () => {
    currentScene.mockResolvedValue(
      scene({
        title: 'Mine',
        creator: '',
        parcel: { x: 0, y: 0 },
        realm: 'https://worlds.test/world/foo.dcl.eth',
        genesis: false,
      }),
    );
    const header = await createCoreMenuApi(createMockApi()).scene();
    expect(header.linkQuery).toBe('realm=foo.dcl.eth');
  });

  it('falls back to a neutral header when the core cannot say', async () => {
    currentScene.mockRejectedValue(new Error('no scene'));
    expect(await createCoreMenuApi(createMockApi()).scene()).toEqual({
      title: 'Decentraland',
      creator: '',
      linkQuery: 'position=0,0',
    });
  });

  it('captures the screenshot as a file uri', async () => {
    captureScreenshot.mockResolvedValue('/data/cache/shot.png');
    expect(await createCoreMenuApi(createMockApi()).captureScreenshot()).toBe(
      'file:///data/cache/shot.png',
    );
  });
});
