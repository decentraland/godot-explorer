import { ReactElement } from 'react';
import { act, create, ReactTestInstance } from 'react-test-renderer';
import { StyleSheet, Text, TextInput } from 'react-native';
import {
  ButtonProfileLink,
  ButtonSeeMore,
  color,
  IconProfile,
  InputUsername,
  MenuNavItem,
  ModalSpecial,
  ProfileAttribute,
  SettingsRow,
  Switch,
} from '@dcl/ui-native';

function render(element: ReactElement) {
  let tree!: ReturnType<typeof create>;
  act(() => {
    tree = create(element);
  });
  return tree;
}

const texts = (root: ReactTestInstance) =>
  root
    .findAllByType(Text)
    .map(node => node.props.children)
    .flat()
    .filter(child => typeof child === 'string');

describe('Figma components added to the DS', () => {
  it('ButtonSeeMore toggles its label and fires onPress', () => {
    const onPress = jest.fn();
    const more = render(<ButtonSeeMore onPress={onPress} />);
    expect(texts(more.root)).toContain('See more');
    act(() =>
      more.root.findByProps({ accessibilityRole: 'button' }).props.onPress(),
    );
    expect(onPress).toHaveBeenCalled();
    const less = render(
      <ButtonSeeMore expanded lessLabel="Less" onPress={onPress} />,
    );
    expect(texts(less.root)).toContain('Less');
  });

  it('ButtonProfileLink shows the remove badge only in edit mode', () => {
    const onRemove = jest.fn();
    const plain = render(
      <ButtonProfileLink label="Site" onPress={jest.fn()} />,
    );
    expect(
      plain.root.findAllByProps({ accessibilityLabel: 'Remove' }),
    ).toHaveLength(0);
    const edit = render(
      <ButtonProfileLink
        label="Site"
        onPress={jest.fn()}
        onRemove={onRemove}
      />,
    );
    act(() =>
      edit.root
        .findAllByProps({ accessibilityLabel: 'Remove' })[0]
        .props.onPress(),
    );
    expect(onRemove).toHaveBeenCalled();
  });

  it('ProfileAttribute hides an empty field', () => {
    expect(
      render(
        <ProfileAttribute icon="Country" title="Country" value="" />,
      ).toJSON(),
    ).toBeNull();
    const shown = render(
      <ProfileAttribute icon="Country" title="Country" value="Argentina" />,
    );
    expect(texts(shown.root)).toEqual(['Country', 'Argentina']);
  });

  it('IconProfile renders every type', () => {
    for (const type of [
      'Gender',
      'Country',
      'Birthday',
      'Pronouns',
      'Relationship',
      'SexOrientation',
      'Language',
      'Profession',
      'Hobbies',
      'RealName',
      'EmploymentStatus',
    ] as const) {
      expect(render(<IconProfile type={type} />).toJSON()).not.toBeNull();
    }
  });

  it('MenuNavItem marks the selected item with the bold style', () => {
    const tree = render(
      <MenuNavItem label="Audio" selected onPress={jest.fn()} />,
    );
    const label = StyleSheet.flatten(tree.root.findByType(Text).props.style);
    expect(label.fontFamily).toBe('Inter-Bold');
  });

  it('SettingsRow dims its text when disabled', () => {
    const tree = render(
      <SettingsRow
        type="SwitchDescription"
        label="Dynamic graphics"
        description="Adjusts graphics"
        disabled
        control={
          <Switch label="x" value={false} onChange={jest.fn()} disabled />
        }
      />,
    );
    const label = StyleSheet.flatten(
      tree.root.findAllByType(Text)[0].props.style,
    );
    expect(label.color).toBe(color.text.disabled);
    expect(texts(tree.root)).toContain('Adjusts graphics');
  });

  it('InputUsername shows the free name field without a unique name', () => {
    const tree = render(
      <InputUsername
        label="Username"
        uniqueName={false}
        tab="Unique"
        onTabChange={jest.fn()}
        uniqueTabLabel="UNIQUE NAME"
        nameTabLabel="NON-UNIQUE USERNAME"
        names={[]}
        picked={null}
        onPick={jest.fn()}
        value="seba"
        onChangeText={jest.fn()}
        suffix="#1234"
        maxLength={15}
      />,
    );
    expect(tree.root.findAllByType(TextInput)).toHaveLength(1);
    expect(texts(tree.root)).not.toContain('UNIQUE NAME');
  });

  it('ModalSpecial wires both CTAs', () => {
    const onPrimary = jest.fn();
    const onSecondary = jest.fn();
    const tree = render(
      <ModalSpecial
        visible
        title="Unlock the full experience"
        body="Add email to claim an"
        highlight="exclusive Wearable."
        primaryLabel="Add email"
        onPrimary={onPrimary}
        secondaryLabel="Maybe later"
        onSecondary={onSecondary}
        onRequestClose={jest.fn()}
      />,
    );
    act(() => tree.root.findByProps({ label: 'Add email' }).props.onPress());
    act(() => tree.root.findByProps({ label: 'Maybe later' }).props.onPress());
    expect(onPrimary).toHaveBeenCalled();
    expect(onSecondary).toHaveBeenCalled();
  });
});
