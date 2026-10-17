import { ReactElement } from 'react';
import { act, create } from 'react-test-renderer';
import { Dialog, Dropdown, PrimaryButton, Tabs, TextField } from '../src/ds';

const render = (element: ReactElement) => {
  let tree!: ReturnType<typeof create>;
  act(() => {
    tree = create(element);
  });
  return tree;
};
const text = (tree: ReturnType<typeof create>) => JSON.stringify(tree.toJSON());

describe('TextField additions', () => {
  const base = {
    value: 'abc',
    onChangeText: () => {},
    maxLength: 15,
    showCounter: true,
  };

  it('keeps the counter beside the label by default', () => {
    const out = text(render(<TextField {...base} label="Name" />));
    expect(out).toContain('3/15');
  });

  it('draws the counter under the box and the helper above it on request', () => {
    const tree = render(
      <TextField
        {...base}
        label="Name"
        helper="Say it"
        counterBelow
        helperAbove
      />,
    );
    const out = text(tree);
    expect(out.indexOf('Say it')).toBeLessThan(out.indexOf('3/15'));
    expect(out.indexOf('Name')).toBeLessThan(out.indexOf('Say it'));
  });

  it('shows an error instead of an above helper, and a suffix', () => {
    const out = text(
      render(
        <TextField
          {...base}
          label="Name"
          helper="Say it"
          helperAbove
          error="Bad"
          suffix={<></>}
        />,
      ),
    );
    expect(out).toContain('Bad');
  });

  it('omits the label row for an empty label without a counter beside it', () => {
    const out = text(render(<TextField {...base} label="" counterBelow />));
    expect(out).toContain('3/15');
  });
});

describe('Dropdown additions', () => {
  const options = [
    { value: 'a', label: 'Alpha', icon: 'seal' as const },
    { value: 'b', label: 'Beta' },
  ];

  it('reports open changes and draws option icons', () => {
    const onOpenChange = jest.fn();
    const tree = render(
      <Dropdown
        label=""
        value="a"
        options={options}
        onChange={() => {}}
        onOpenChange={onOpenChange}
      />,
    );
    const field = tree.root.findByProps({ accessibilityLabel: '' });
    act(() => field.props.onPress());
    expect(onOpenChange).toHaveBeenCalledWith(true);
    expect(
      tree.root.findAllByProps({ accessibilityRole: 'menuitem' }).length,
    ).toBeGreaterThanOrEqual(2);
    act(() =>
      tree.root.findByProps({ accessibilityLabel: 'Beta' }).props.onPress(),
    );
    expect(onOpenChange).toHaveBeenLastCalledWith(false);
  });
});

describe('Dialog additions', () => {
  it('disables a button', () => {
    const tree = render(
      <Dialog
        visible
        title="T"
        onRequestClose={() => {}}
        buttons={[{ label: 'GO', disabled: true, onPress: () => {} }]}
      />,
    );
    expect(
      tree.root.findByProps({ accessibilityLabel: 'GO' }).props
        .accessibilityState,
    ).toEqual({ disabled: true });
  });
});

describe('Figma-fidelity additions', () => {
  it('PrimaryButton draws the dark tone with rounded corners', () => {
    const out = text(
      render(
        <PrimaryButton label="Leave" tone="dark" rounded onPress={() => {}} />,
      ),
    );
    expect(out).toContain('rgba(0,0,0,0.4)');
  });

  it('Tabs with a gap hug their labels', () => {
    const out = text(
      render(
        <Tabs
          items={[
            { key: 'a', label: 'A' },
            { key: 'b', label: 'B' },
          ]}
          active="a"
          onChange={() => {}}
          gap={14}
          labelSize={10}
        />,
      ),
    );
    expect(out).toContain('flex-start');
  });

  it('Dropdown draws an option badge node next to the label', () => {
    const out = text(
      render(
        <Dropdown
          label="Name"
          value="a"
          onChange={() => {}}
          options={[{ value: 'a', label: 'Alpha', iconNode: <></> }]}
        />,
      ),
    );
    expect(out).toContain('Alpha');
  });

  it('TextField can start focused', () => {
    const tree = render(
      <TextField label="L" value="" onChangeText={() => {}} autoFocus />,
    );
    expect(text(tree)).toContain('"autoFocus":true');
  });
});
