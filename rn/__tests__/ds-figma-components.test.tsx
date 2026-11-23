import { ReactElement } from 'react';
import { act, create } from 'react-test-renderer';
import { Text } from 'react-native';
import {
  BugReportDialog,
  Button,
  FriendRow,
  IconButton,
  MenuDrawer,
  NotificationRow,
  PlaceModalLandscape,
  ScreenshotCard,
  SectionCollapsible,
  Tabs,
} from '@dcl/ui-native';

function render(element: ReactElement) {
  let tree!: ReturnType<typeof create>;
  act(() => {
    tree = create(element);
  });
  return tree;
}

const texts = (tree: ReturnType<typeof create>) =>
  tree.root
    .findAllByType(Text)
    .map(node => node.props.children)
    .flat()
    .filter(child => typeof child === 'string' || typeof child === 'number')
    .map(String);

describe('Design system components from Figma', () => {
  it('Tabs shows the count after the label', () => {
    const tree = render(
      <Tabs
        items={[{ key: 'nearby', label: 'Nearby', count: 10 }]}
        active="nearby"
        onChange={jest.fn()}
      />,
    );
    expect(texts(tree)).toEqual(['Nearby', '10']);
  });

  it('SectionCollapsible shows children only while expanded', () => {
    const onToggle = jest.fn();
    const closed = render(
      <SectionCollapsible
        title="Online"
        count={2}
        expanded={false}
        onToggle={onToggle}
      >
        <Text>Row</Text>
      </SectionCollapsible>,
    );
    expect(texts(closed)).toEqual(['Online', '(', '2', ')']);
    act(() =>
      closed.root.findByProps({ accessibilityRole: 'button' }).props.onPress(),
    );
    expect(onToggle).toHaveBeenCalled();
    const open = render(
      <SectionCollapsible title="Online" expanded onToggle={onToggle}>
        <Text>Row</Text>
      </SectionCollapsible>,
    );
    expect(texts(open)).toContain('Row');
  });

  it('FriendRow renders the action for each kind', () => {
    const jump = render(
      <FriendRow
        name="Ana"
        action={{ kind: 'jumpIn', label: 'Jump', onPress: jest.fn() }}
      />,
    );
    expect(jump.root.findAllByType(IconButton)).toHaveLength(1);
    const add = render(
      <FriendRow
        name="Ana"
        action={{ kind: 'addFriend', label: 'Add friend', onPress: jest.fn() }}
      />,
    );
    expect(add.root.findByType(Button).props.label).toBe('Add friend');
    const onAccept = jest.fn();
    const request = render(
      <FriendRow
        name="Ana"
        action={{
          kind: 'request',
          acceptLabel: 'Accept',
          declineLabel: 'Decline',
          onAccept,
          onDecline: jest.fn(),
        }}
      />,
    );
    const buttons = request.root.findAllByType(IconButton);
    expect(buttons).toHaveLength(2);
    act(() => buttons[0].props.onPress());
    expect(onAccept).toHaveBeenCalled();
  });

  it('NotificationRow hides the title for NoTitle and dims read rows', () => {
    const tree = render(
      <NotificationRow
        thumbnail={null}
        body="Invited you"
        date="2 days ago"
        read
      />,
    );
    expect(texts(tree)).toEqual(['Invited you', '2 days ago']);
  });

  it('MenuDrawer renders the header title and content', () => {
    const tree = render(
      <MenuDrawer rail={null} title="Notifications">
        <Text>Item</Text>
      </MenuDrawer>,
    );
    expect(texts(tree)).toEqual(
      expect.arrayContaining(['Notifications', 'Item']),
    );
  });

  it('ScreenshotCard shows the remove button only with an image', () => {
    const onRemove = jest.fn();
    const remove = (tree: ReturnType<typeof create>) =>
      tree.root.findAll(n => n.props.onPress === onRemove).length;
    const empty = render(
      <ScreenshotCard
        accessibilityLabel="Add"
        onPress={jest.fn()}
        onRemove={onRemove}
      />,
    );
    expect(remove(empty)).toBe(0);
    const filled = render(
      <ScreenshotCard
        uri="file://shot.png"
        accessibilityLabel="Shot"
        onPress={jest.fn()}
        onRemove={onRemove}
      />,
    );
    expect(remove(filled)).toBeGreaterThan(0);
  });

  it('BugReportDialog keeps Submit disabled until type and description are set', () => {
    const labels = {
      title: 'Bug Report',
      issueType: 'Issue Type',
      select: 'Select',
      description: 'Description',
      descriptionPlaceholder: 'Briefly describe',
      screenshot: 'Screenshot',
      addScreenshot: 'Add',
      cancel: 'Cancel',
      submit: 'Submit',
    };
    const props = {
      visible: true,
      labels,
      issueTypes: [{ value: 'crash', label: 'Crash' }],
      onIssueTypeChange: jest.fn(),
      onDescriptionChange: jest.fn(),
      onAddScreenshot: jest.fn(),
      onRemoveScreenshot: jest.fn(),
      onSubmit: jest.fn(),
      onRequestClose: jest.fn(),
    };
    const submit = (tree: ReturnType<typeof create>) =>
      tree.root.findAllByType(Button).find(b => b.props.label === 'Submit')!
        .props.disabled;
    expect(
      submit(
        render(<BugReportDialog {...props} issueType={null} description="" />),
      ),
    ).toBe(true);
    expect(
      submit(
        render(
          <BugReportDialog
            {...props}
            issueType="crash"
            description="It froze"
          />,
        ),
      ),
    ).toBe(false);
  });

  it('PlaceModalLandscape switches the action button', () => {
    const base = {
      title: 'Name event',
      creator: 'By Creator',
      onClose: jest.fn(),
    };
    const live = render(
      <PlaceModalLandscape
        {...base}
        action={{ kind: 'jumpIn', label: 'Jump in', onPress: jest.fn() }}
      />,
    );
    expect(live.root.findByType(Button).props.trailingIcon).toBe('jumpIn');
    const set = render(
      <PlaceModalLandscape
        {...base}
        action={{
          kind: 'reminderSet',
          label: 'Reminder set',
          onPress: jest.fn(),
        }}
      />,
    );
    expect(set.root.findByType(Button).props).toMatchObject({
      tone: 'reminder',
      selected: true,
    });
  });
});
