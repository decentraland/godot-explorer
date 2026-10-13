import { ReactElement } from 'react';
import { Text, TextInput } from 'react-native';
import {
  act,
  create,
  ReactTestInstance,
  ReactTestRenderer,
} from 'react-test-renderer';
import { SafeAreaProvider } from 'react-native-safe-area-context';
import { AddLinkDialog } from '../src/profile/AddLinkDialog';
import { EditProfile } from '../src/profile/EditProfile';
import { MOCK_NAMES, MOCK_PROFILE } from '../src/profile/mockData';
import { ProfileLandscape } from '../src/profile/ProfileLandscape';
import { ProfileSheet } from '../src/profile/ProfileSheet';
import type { Profile } from '../src/profile/types';

jest.mock('../src/GodotBridge', () => ({
  sendCommand: jest.fn(),
  startEngine: jest.fn(),
  setOrientation: jest.fn(),
  subscribe: jest.fn(() => () => {}),
}));

const metrics = {
  frame: { x: 0, y: 0, width: 400, height: 900 },
  insets: { top: 0, left: 0, right: 0, bottom: 0 },
};
const mounted: ReactTestRenderer[] = [];

function mount(node: ReactElement): ReactTestRenderer {
  let tree!: ReactTestRenderer;
  act(() => {
    tree = create(
      <SafeAreaProvider initialMetrics={metrics}>{node}</SafeAreaProvider>,
    );
  });
  mounted.push(tree);
  return tree;
}
afterEach(() => act(() => mounted.splice(0).forEach(tree => tree.unmount())));

const text = (tree: ReactTestRenderer) => JSON.stringify(tree.toJSON());
const press = (tree: ReactTestRenderer, label: string) => {
  const node = tree.root
    .findAllByProps({ accessibilityLabel: label })
    .find((n: ReactTestInstance) => typeof n.props.onPress === 'function');
  if (!node) {
    throw new Error(`no pressable "${label}"`);
  }
  act(() => node.props.onPress());
};
const pressLast = (tree: ReactTestRenderer, label: string) => {
  const nodes = tree.root
    .findAllByProps({ accessibilityLabel: label })
    .filter((n: ReactTestInstance) => typeof n.props.onPress === 'function');
  act(() => nodes[nodes.length - 1].props.onPress());
};
const pressTab = (tree: ReactTestRenderer, label: string) => {
  const tab = tree.root
    .findAllByProps({ accessibilityRole: 'tab' })
    .find(n => n.findAllByType(Text).some(x => x.props.children === label));
  act(() => tab?.props.onPress());
};
const inputs = (tree: ReactTestRenderer) => tree.root.findAllByType(TextInput);
const noop = () => {};

describe('ProfileSheet', () => {
  const base = {
    profile: MOCK_PROFILE as Profile | null,
    snap: 'half' as const,
    onSnapChange: noop,
    onEdit: noop,
    onCopyName: noop,
    onCopyAddress: noop,
  };

  it('shows header, about, fields, links and equipped', () => {
    const out = text(mount(<ProfileSheet {...base} />));
    for (const s of [
      'Namemaxcharacte',
      '#1234',
      '0x4e1...1c13B',
      'EDIT PROFILE',
      'Argentina',
      'He / Him',
      'Instagram',
      'LEGENDARY',
    ]) {
      expect(out).toContain(s);
    }
  });

  it('hides About me, links and empty fields when there is nothing to show', () => {
    const empty = {
      ...MOCK_PROFILE,
      description: '',
      country: '',
      language: '',
      pronouns: '',
      gender: '',
      sexualOrientation: '',
      relationshipStatus: '',
      employmentStatus: '',
      profession: '',
      realName: '',
      hobbies: '',
      links: [],
    };
    const out = text(mount(<ProfileSheet {...base} profile={empty} />));
    expect(out).not.toContain('About me');
    expect(out).not.toContain('Instagram');
    expect(out).toContain('EDIT PROFILE');
  });

  it('draws the claimed seal instead of the tag', () => {
    const out = text(
      mount(
        <ProfileSheet
          {...base}
          profile={{ ...MOCK_PROFILE, hasClaimedName: true }}
        />,
      ),
    );
    expect(out).not.toContain('#1234');
  });

  it('full snap has a close that returns to half; peek and loading render', () => {
    const onSnapChange = jest.fn();
    const full = mount(
      <ProfileSheet {...base} snap="full" onSnapChange={onSnapChange} />,
    );
    press(full, 'Close');
    expect(onSnapChange).toHaveBeenCalledWith('half');
    expect(text(mount(<ProfileSheet {...base} snap="peek" />))).toContain(
      'EDIT PROFILE',
    );
    expect(
      text(mount(<ProfileSheet {...base} profile={null} />)),
    ).not.toContain('Namemaxcharacte');
  });

  it('routes edit and copy presses', () => {
    const onEdit = jest.fn();
    const onCopyName = jest.fn();
    const onCopyAddress = jest.fn();
    const tree = mount(
      <ProfileSheet
        {...base}
        onEdit={onEdit}
        onCopyName={onCopyName}
        onCopyAddress={onCopyAddress}
      />,
    );
    press(tree, 'EDIT PROFILE');
    press(tree, 'Copy name');
    press(tree, 'Copy address');
    expect(
      [onEdit, onCopyName, onCopyAddress].map(f => f.mock.calls.length),
    ).toEqual([1, 1, 1]);
  });
});

describe('ProfileLandscape', () => {
  const props = {
    profile: MOCK_PROFILE,
    onClose: noop,
    onEdit: noop,
    onCopyAddress: noop,
  };

  it('toggles SEE MORE / SEE LESS and reveals the data grid', () => {
    const tree = mount(<ProfileLandscape {...props} />);
    expect(text(tree)).not.toContain('Argentina');
    press(tree, 'SEE MORE');
    expect(text(tree)).toContain('Argentina');
    press(tree, 'SEE LESS');
    expect(text(tree)).not.toContain('Argentina');
  });

  it('card has X, full screen has back; both close', () => {
    const onClose = jest.fn();
    press(mount(<ProfileLandscape {...props} onClose={onClose} />), 'Close');
    press(
      mount(<ProfileLandscape {...props} fullscreen onClose={onClose} />),
      'Back',
    );
    expect(onClose).toHaveBeenCalledTimes(2);
  });

  it('renders the skeleton while loading', () => {
    expect(
      text(mount(<ProfileLandscape {...props} profile={null} />)),
    ).not.toContain('Namemaxcharacte');
  });
});

describe('EditProfile', () => {
  const profile: Profile = {
    ...MOCK_PROFILE,
    hasClaimedName: false,
    links: [],
  };

  it('starts clean with SAVE disabled and leaves without asking', () => {
    const onClose = jest.fn();
    const tree = mount(
      <EditProfile
        profile={profile}
        names={[]}
        onClose={onClose}
        onSave={noop}
      />,
    );
    expect(
      tree.root.findByProps({ accessibilityLabel: 'SAVE' }).props
        .accessibilityState,
    ).toEqual({ disabled: true });
    press(tree, 'CANCEL');
    expect(onClose).toHaveBeenCalled();
  });

  it('saves only the changed fields', () => {
    const onSave = jest.fn();
    const tree = mount(
      <EditProfile
        profile={profile}
        names={[]}
        onClose={noop}
        onSave={onSave}
      />,
    );
    const description = inputs(tree).find(
      i => i.props.accessibilityLabel === 'Description',
    );
    act(() => description?.props.onChangeText('Hello'));
    press(tree, 'SAVE');
    expect(onSave).toHaveBeenCalledWith({ description: 'Hello' });
  });

  it('asks before leaving a dirty form; STAY keeps it, LEAVE closes', () => {
    const onClose = jest.fn();
    const tree = mount(
      <EditProfile
        profile={profile}
        names={[]}
        onClose={onClose}
        onSave={noop}
        initial={{ draft: { description: 'x' } }}
      />,
    );
    press(tree, 'CANCEL');
    expect(text(tree)).toContain('Leave without saving?');
    press(tree, 'STAY');
    expect(onClose).not.toHaveBeenCalled();
    press(tree, 'CANCEL');
    press(tree, 'LEAVE');
    expect(onClose).toHaveBeenCalled();
  });

  it('adds and removes links, hiding ADD LINK at five', () => {
    const tree = mount(
      <EditProfile profile={profile} names={[]} onClose={noop} onSave={noop} />,
    );
    press(tree, 'ADD LINK');
    const [title, url] = inputs(tree).filter(i =>
      ['Link title', 'Link'].includes(i.props.accessibilityLabel),
    );
    act(() => title.props.onChangeText('Web'));
    act(() => url.props.onChangeText('https://decentraland.org'));
    pressLast(tree, 'SAVE');
    expect(text(tree)).toContain('Web');
    press(tree, 'Remove Web');
    expect(text(tree)).not.toContain('Remove Web');
    const five = Array.from({ length: 5 }, (_, i) => ({
      title: `L${i}`,
      url: 'https://a.com',
    }));
    const full = mount(
      <EditProfile
        profile={profile}
        names={[]}
        onClose={noop}
        onSave={noop}
        initial={{ draft: { links: five } }}
      />,
    );
    expect(
      full.root.findAllByProps({ accessibilityLabel: 'ADD LINK' }),
    ).toHaveLength(0);
  });

  it('shows the tabs and the names dropdown for owners of names', () => {
    const tree = mount(
      <EditProfile
        profile={{ ...profile, hasClaimedName: true, name: 'Name' }}
        names={MOCK_NAMES}
        onClose={noop}
        onSave={noop}
        initial={{ open: 'username' }}
      />,
    );
    const out = text(tree);
    expect(out).toContain('UNIQUE NAME');
    expect(out).toContain('NON-UNIQUE USERNAME');
    expect(out).toContain('Name 02');
  });

  it('publishes the picked minted name as claimed', () => {
    const onSave = jest.fn();
    const tree = mount(
      <EditProfile
        profile={{ ...profile, hasClaimedName: true, name: 'Name' }}
        names={MOCK_NAMES}
        onClose={noop}
        onSave={onSave}
        initial={{ picked: 'Name 02' }}
      />,
    );
    press(tree, 'SAVE');
    expect(onSave).toHaveBeenCalledWith({ name: 'Name 02' });
  });

  it('switches to the non-unique input with the tag and blocks edge spaces', () => {
    const onSave = jest.fn();
    const tree = mount(
      <EditProfile
        profile={{ ...profile, hasClaimedName: true, name: 'Name' }}
        names={MOCK_NAMES}
        onClose={noop}
        onSave={onSave}
      />,
    );
    pressTab(tree, 'NON-UNIQUE USERNAME');
    expect(text(tree)).toContain('#1234');
    const input = inputs(tree).find(i => i.props.placeholder === 'username');
    act(() => input?.props.onChangeText(' bad'));
    expect(text(tree)).toContain('No leading or trailing spaces');
    expect(
      tree.root.findByProps({ accessibilityLabel: 'SAVE' }).props
        .accessibilityState,
    ).toEqual({ disabled: true });
  });

  it('disables the username for guests', () => {
    const tree = mount(
      <EditProfile
        profile={{ ...profile, guest: true }}
        names={[]}
        onClose={noop}
        onSave={noop}
      />,
    );
    const username = inputs(tree).find(
      i => i.props.accessibilityLabel === 'Username',
    );
    expect(username?.props.editable).toBe(false);
  });
});

describe('AddLinkDialog', () => {
  it('renders nothing while hidden and enables SAVE for valid input', () => {
    const onSave = jest.fn();
    const hidden = mount(
      <AddLinkDialog visible={false} onCancel={noop} onSave={onSave} />,
    );
    expect(
      hidden.root.findAllByProps({ accessibilityLabel: 'SAVE' }),
    ).toHaveLength(0);
    const tree = mount(
      <AddLinkDialog
        visible
        onCancel={noop}
        onSave={onSave}
        initial={{ title: ' Instagram ', url: 'https://www.instagram.com/ru' }}
      />,
    );
    press(tree, 'SAVE');
    expect(onSave).toHaveBeenCalledWith({
      title: 'Instagram',
      url: 'https://www.instagram.com/ru',
    });
  });

  it('keeps SAVE disabled for a bad URL and shows the error', () => {
    const tree = mount(
      <AddLinkDialog
        visible
        onCancel={noop}
        onSave={noop}
        initial={{ title: 'x', url: 'nope' }}
      />,
    );
    expect(text(tree)).toContain('Enter a valid URL');
    expect(
      tree.root.findByProps({ accessibilityLabel: 'SAVE' }).props
        .accessibilityState,
    ).toEqual({ disabled: true });
  });
});
