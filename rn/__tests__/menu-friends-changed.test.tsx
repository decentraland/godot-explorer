import { act, create } from 'react-test-renderer';
import { SafeAreaProvider } from 'react-native-safe-area-context';
import { setCoreForTests } from '../src/core';
import { resetCoreEventsForTests } from '../src/core/events';
import { createMockCore } from '../src/core/mock';
import type { AppListener } from '../src/core/native';
import { setMenuApi } from '../src/menu/api';
import { openMenu, resetDrawerForTests } from '../src/menu/drawerState';
import { MenuDrawer } from '../src/menu/MenuDrawer';
import { createMockApi } from '../src/menu/mockApi';
import { mockSocial } from '../src/menu/mockData';

const metrics = {
  frame: { x: 0, y: 0, width: 800, height: 360 },
  insets: { top: 0, left: 0, right: 0, bottom: 0 },
};

const settle = () =>
  act(async () => {
    await new Promise(resolve => setTimeout(resolve, 20));
  });

describe('MenuDrawer friendsChanged', () => {
  afterEach(() => {
    setMenuApi(null);
    setCoreForTests(undefined);
    resetCoreEventsForTests();
  });

  it('reloads the social lists without a loading flash', async () => {
    resetDrawerForTests();
    resetCoreEventsForTests();
    let listener: AppListener | undefined;
    setCoreForTests({
      ...createMockCore(),
      addListener: l => {
        listener = l;
        return 0n;
      },
    });
    const social = jest
      .fn()
      .mockResolvedValueOnce(mockSocial)
      .mockResolvedValueOnce({ ...mockSocial, friends: [] });
    setMenuApi({ ...createMockApi(), social });

    let tree!: ReturnType<typeof create>;
    act(() => {
      tree = create(
        <SafeAreaProvider initialMetrics={metrics}>
          <MenuDrawer navigate={jest.fn()} />
        </SafeAreaProvider>,
      );
    });
    act(() => openMenu('social'));
    await settle();
    expect(JSON.stringify(tree.toJSON())).toContain('ONLINE (2)');

    await act(async () => {
      listener?.friendsChanged();
    });
    await settle();
    expect(social).toHaveBeenCalledTimes(2);
    expect(JSON.stringify(tree.toJSON())).toContain('No friends yet');
  });
});
