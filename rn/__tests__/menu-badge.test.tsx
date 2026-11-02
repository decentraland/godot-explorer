import { act, create } from 'react-test-renderer';
import { Text } from 'react-native';
import { ArtFill } from '../src/menu/ArtFill';
import { setMenuApi } from '../src/menu/api';
import { resetDrawerForTests } from '../src/menu/drawerState';
import { createMockApi } from '../src/menu/mockApi';
import { useMenuUnread } from '../src/menu';

function Probe() {
  return <Text>{String(useMenuUnread())}</Text>;
}

describe('useMenuUnread', () => {
  beforeEach(() => {
    resetDrawerForTests();
    setMenuApi(createMockApi());
  });

  it('turns true once unread notifications load', async () => {
    let tree!: ReturnType<typeof create>;
    await act(async () => {
      tree = create(<Probe />);
    });
    await act(async () => {
      await new Promise(resolve => setTimeout(resolve, 20));
    });
    expect(tree.root.findByType(Text).props.children).toBe('true');
  });

  it('is false when every notification is read', async () => {
    const api = createMockApi();
    setMenuApi({
      ...api,
      notifications: async () => ({ status: 'ok', items: [] }),
    });
    let tree!: ReturnType<typeof create>;
    await act(async () => {
      tree = create(<Probe />);
    });
    expect(tree.root.findByType(Text).props.children).toBe('false');
  });
});

describe('ArtFill', () => {
  it('renders for any seed', () => {
    expect(() => create(<ArtFill seed="Genesis Plaza" />)).not.toThrow();
    expect(() => create(<ArtFill />)).not.toThrow();
  });
});
