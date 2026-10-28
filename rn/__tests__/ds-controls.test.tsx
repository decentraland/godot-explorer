import { ReactElement } from 'react';
import { act, create, ReactTestInstance } from 'react-test-renderer';
import {
  Dropdown,
  OutlinedButton,
  ReadOnlyField,
  SearchField,
  Slider,
  Switch,
  TextField,
  counterLabel,
  sliderValueFromX,
} from '../src/ds';

function render(element: ReactElement) {
  let tree!: ReturnType<typeof create>;
  act(() => {
    tree = create(element);
  });
  return tree;
}

const text = (tree: ReturnType<typeof create>) => JSON.stringify(tree.toJSON());

function pressable(tree: ReturnType<typeof create>, label: string) {
  const node = tree.root
    .findAllByProps({ accessibilityLabel: label })
    .find((n: ReactTestInstance) => typeof n.props.onPress === 'function');
  if (!node) {
    throw new Error(`no pressable "${label}"`);
  }
  return node;
}

describe('Switch', () => {
  it('reports its state and toggles', () => {
    const onChange = jest.fn();
    const tree = render(<Switch value label="Dynamic" onChange={onChange} />);
    const node = pressable(tree, 'Dynamic');
    expect(node.props.accessibilityRole).toBe('switch');
    expect(node.props.accessibilityState).toMatchObject({ checked: true });
    act(() => node.props.onPress());
    expect(onChange).toHaveBeenCalledWith(false);
  });

  it('ignores presses when disabled', () => {
    const onChange = jest.fn();
    const tree = render(
      <Switch value={false} label="A" disabled onChange={onChange} />,
    );
    const node = pressable(tree, 'A');
    expect(node.props.accessibilityState).toMatchObject({ disabled: true });
    expect(node.props.disabled).toBe(true);
  });
});

describe('sliderValueFromX', () => {
  it('maps position to a stepped value and clamps', () => {
    expect(sliderValueFromX(50, 100)).toBe(50);
    expect(sliderValueFromX(-20, 100)).toBe(0);
    expect(sliderValueFromX(250, 100)).toBe(100);
    expect(sliderValueFromX(33, 100, 0, 100, 10)).toBe(30);
    expect(sliderValueFromX(10, 0)).toBe(0);
  });
});

describe('Slider', () => {
  it('shows label and percentage', () => {
    const tree = render(
      <Slider label="General" value={42} onChange={() => {}} />,
    );
    expect(text(tree)).toContain('General');
    expect(text(tree)).toContain('42%');
  });

  it('exposes adjustable accessibility and steps via actions', () => {
    const onChange = jest.fn();
    const tree = render(
      <Slider label="General" value={42} onChange={onChange} />,
    );
    const node = tree.root.findAllByProps({
      accessibilityRole: 'adjustable',
    })[0];
    expect(node.props.accessibilityValue).toMatchObject({
      min: 0,
      max: 100,
      now: 42,
    });
    act(() =>
      node.props.onAccessibilityAction({
        nativeEvent: { actionName: 'increment' },
      }),
    );
    expect(onChange).toHaveBeenCalledWith(43);
  });
});

describe('Dropdown', () => {
  const options = [
    { value: 'en', label: 'English' },
    { value: 'es', label: 'Espanol' },
  ];

  it('shows the selected label or the placeholder', () => {
    const a = render(
      <Dropdown
        label="Lang"
        value="es"
        options={options}
        onChange={() => {}}
      />,
    );
    expect(text(a)).toContain('Espanol');
    const b = render(
      <Dropdown
        label="Lang"
        value={null}
        placeholder="Select"
        options={options}
        onChange={() => {}}
      />,
    );
    expect(text(b)).toContain('Select');
  });

  it('opens, selects and closes', () => {
    const onChange = jest.fn();
    const tree = render(
      <Dropdown
        label="Lang"
        value="en"
        options={options}
        onChange={onChange}
      />,
    );
    expect(text(tree)).not.toContain('Espanol');
    act(() => pressable(tree, 'Lang').props.onPress());
    expect(text(tree)).toContain('Espanol');
    act(() => pressable(tree, 'Espanol').props.onPress());
    expect(onChange).toHaveBeenCalledWith('es');
    expect(text(tree)).not.toContain('Espanol');
  });

  it('does not open when disabled', () => {
    const tree = render(
      <Dropdown
        label="Lang"
        value="en"
        disabled
        options={options}
        onChange={() => {}}
      />,
    );
    act(() => pressable(tree, 'Lang').props.onPress());
    expect(text(tree)).not.toContain('Espanol');
  });

  it('can open upwards', () => {
    const tree = render(
      <Dropdown
        label="Lang"
        value="en"
        direction="up"
        defaultOpen
        options={options}
        onChange={() => {}}
      />,
    );
    expect(text(tree)).toContain('Espanol');
  });
});

describe('counterLabel', () => {
  it('formats length over max', () => {
    expect(counterLabel(3, 15)).toBe('3/15');
  });
});

describe('TextField', () => {
  it('renders label, helper counter and error', () => {
    const tree = render(
      <TextField
        label="Title"
        value="abc"
        maxLength={15}
        showCounter
        error="Too short"
        onChangeText={() => {}}
      />,
    );
    const out = text(tree);
    expect(out).toContain('Title');
    expect(out).toContain('3/15');
    expect(out).toContain('Too short');
  });

  it('forwards text and disables editing', () => {
    const onChangeText = jest.fn();
    const tree = render(
      <TextField label="T" value="" disabled onChangeText={onChangeText} />,
    );
    const input = tree.root.findByProps({
      accessibilityLabel: 'T',
      editable: false,
    });
    expect(input).toBeTruthy();
  });

  it('shows a clear button while focused and calls onClear', () => {
    const onClear = jest.fn();
    const onChangeText = jest.fn();
    const tree = render(
      <TextField
        label="T"
        value="abc"
        onChangeText={onChangeText}
        onClear={onClear}
      />,
    );
    const input = tree.root
      .findAllByProps({ accessibilityLabel: 'T' })
      .find(n => n.props.onFocus)!;
    act(() => input.props.onFocus());
    act(() => pressable(tree, 'Clear').props.onPress());
    expect(onClear).toHaveBeenCalled();
  });

  it('supports multiline', () => {
    const tree = render(
      <TextField label="T" value="" multiline onChangeText={() => {}} />,
    );
    expect(
      tree.root.findAllByProps({ multiline: true }).length,
    ).toBeGreaterThan(0);
  });
});

describe('SearchField', () => {
  it('submits, clears and goes back', () => {
    const onSubmit = jest.fn();
    const onChangeText = jest.fn();
    const onBack = jest.fn();
    const tree = render(
      <SearchField
        value="casa"
        placeholder="Search"
        onChangeText={onChangeText}
        onSubmit={onSubmit}
        onBack={onBack}
      />,
    );
    const input = tree.root
      .findAllByProps({ placeholder: 'Search' })
      .find(n => n.props.onSubmitEditing)!;
    act(() => input.props.onSubmitEditing());
    expect(onSubmit).toHaveBeenCalledWith('casa');
    act(() => pressable(tree, 'Clear search').props.onPress());
    expect(onChangeText).toHaveBeenCalledWith('');
    act(() => pressable(tree, 'Back').props.onPress());
    expect(onBack).toHaveBeenCalled();
  });

  it('renders the framed variant', () => {
    const tree = render(
      <SearchField framed value="x" onChangeText={() => {}} />,
    );
    const flat = JSON.stringify(tree.toJSON());
    expect(flat).toContain('"borderWidth":1');
  });

  it('hides clear when empty', () => {
    const tree = render(<SearchField value="" onChangeText={() => {}} />);
    expect(
      tree.root.findAllByProps({ accessibilityLabel: 'Clear search' }),
    ).toHaveLength(0);
  });
});

describe('ReadOnlyField', () => {
  it('renders label and value', () => {
    const out = text(render(<ReadOnlyField label="Email" value="a@b.c" />));
    expect(out).toContain('Email');
    expect(out).toContain('a@b.c');
  });
});

describe('OutlinedButton', () => {
  it('presses and respects disabled', () => {
    const onPress = jest.fn();
    const tree = render(<OutlinedButton label="SIGN OUT" onPress={onPress} />);
    act(() => pressable(tree, 'SIGN OUT').props.onPress());
    expect(onPress).toHaveBeenCalledTimes(1);
    const off = render(
      <OutlinedButton label="CLEAN" disabled onPress={onPress} />,
    );
    expect(pressable(off, 'CLEAN').props.disabled).toBe(true);
  });
});
