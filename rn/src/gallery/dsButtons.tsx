// The Figma "Design System Mobile" Buttons page (0:1), section by section, drawn with the real
// `@dcl/ui-native` components. Titles are the repo names with the Figma set in parentheses.
import { ReactNode } from 'react';
import { StyleSheet, View } from 'react-native';
import {
  AppText,
  Button,
  IconButton,
  LinkButton,
  LoginButton,
  PillButton,
  colors,
  rarityColors,
  u,
} from '@dcl/ui-native';
import { registerPreview } from './registry';

const noop = () => {};

type State = 'Default / Pressed' | 'Selected' | 'Disabled' | 'Notifications';
type Row = { label: string; render: (state: State) => ReactNode };

function Section({
  title,
  figma,
  children,
}: {
  title: string;
  figma: string;
  children: ReactNode;
}) {
  return (
    <View style={styles.section}>
      <AppText variant="heading" style={styles.sectionTitle}>
        {title} <AppText style={styles.sectionFigma}>({figma})</AppText>
      </AppText>
      {children}
    </View>
  );
}

/** One Figma block: a size label and its spec string on the left, the state columns on the right. */
function Block({
  label,
  spec,
  columns,
  rows,
}: {
  label: string;
  spec?: string;
  columns: State[];
  rows: Row[];
}) {
  return (
    <View style={styles.block}>
      <View style={styles.aside}>
        <AppText variant="title" style={styles.asideLabel}>
          {label}
        </AppText>
        {spec && <AppText style={styles.asideSpec}>{spec}</AppText>}
      </View>
      <View style={styles.grid}>
        <View style={styles.gridRow}>
          {columns.map(column => (
            <AppText key={column} variant="caption" style={styles.column}>
              {column}
            </AppText>
          ))}
        </View>
        {rows.map(row => (
          <View key={row.label || 'row'}>
            {row.label !== '' && (
              <AppText variant="caption" style={styles.rowLabel}>
                {row.label}
              </AppText>
            )}
            <View style={styles.gridRow}>
              {columns.map(column => (
                <View key={column} style={styles.cell}>
                  {row.render(column)}
                </View>
              ))}
            </View>
          </View>
        ))}
      </View>
    </View>
  );
}

const STATES: State[] = ['Default / Pressed', 'Disabled'];
const off = (s: State) => s === 'Disabled';
const on = (s: State) => s === 'Selected';

const PRIMARY_SPEC = {
  big: 'Height 96px · Corner 16px · Font 30pt Bold Uppercase · Icon 32x32',
  medium: 'Height 74px · Corner 12px · Font 24pt Bold Uppercase · Icon 28x28',
  small: 'Height 54px · Corner 8px · Font 22pt Bold Uppercase · Icon 24x24',
};

const primaryRows = (size: { big?: boolean; small?: boolean }): Row[] => [
  {
    label: 'Type=Primary · Icon=False',
    render: s => (
      <Button
        {...size}
        label="Button"
        disabled={off(s)}
        fullWidth
        onPress={noop}
      />
    ),
  },
  {
    label: 'Type=Primary · Icon=True',
    render: s => (
      <Button
        {...size}
        label="Button"
        icon="check"
        disabled={off(s)}
        fullWidth
        onPress={noop}
      />
    ),
  },
  {
    label: 'Type=JumpIn · Icon=True',
    render: s => (
      <Button
        {...size}
        label="Jump in"
        trailingIcon="jumpIn"
        disabled={off(s)}
        fullWidth
        onPress={noop}
      />
    ),
  },
];

const secondaryRows = (size: { big?: boolean; small?: boolean }): Row[] => [
  {
    label: 'Outline=False · Icon=False',
    render: s => (
      <Button
        {...size}
        tone="secondary"
        label="Button"
        disabled={off(s)}
        fullWidth
        onPress={noop}
      />
    ),
  },
  {
    label: 'Outline=False · Icon=True',
    render: s => (
      <Button
        {...size}
        tone="secondary"
        label="Button"
        icon="check"
        disabled={off(s)}
        fullWidth
        onPress={noop}
      />
    ),
  },
  {
    label: 'Outline=True (2pt) · Icon=False',
    render: s => (
      <Button
        {...size}
        tone="outline"
        label="Button"
        disabled={off(s)}
        fullWidth
        onPress={noop}
      />
    ),
  },
  {
    label: 'Outline=True (2pt) · Icon=True',
    render: s => (
      <Button
        {...size}
        tone="outline"
        label="Button"
        icon="check"
        disabled={off(s)}
        fullWidth
        onPress={noop}
      />
    ),
  },
];

const ENTRY: {
  label: string;
  provider: 'google' | 'apple' | 'wallet' | 'email';
  text: string;
}[] = [
  { label: 'Type=Google', provider: 'google', text: 'Google' },
  { label: 'Type=Apple', provider: 'apple', text: 'Apple' },
  { label: 'Type=Wallet', provider: 'wallet', text: 'Connect wallet' },
  { label: 'Type=Mail', provider: 'email', text: 'Email' },
];

function DsButtons() {
  return (
    <View style={styles.content}>
      <AppText variant="heading" style={styles.pageTitle}>
        Buttons
      </AppText>

      <Section title="Button" figma="Button/Primary">
        <Block
          label="Size Big"
          spec={PRIMARY_SPEC.big}
          columns={STATES}
          rows={primaryRows({ big: true })}
        />
        <Block
          label="Medium"
          spec={PRIMARY_SPEC.medium}
          columns={STATES}
          rows={[
            ...primaryRows({}),
            {
              label: 'Type=Claim',
              render: s => (
                <Button
                  tone="claim"
                  label={off(s) ? 'Claimed' : 'Claim'}
                  disabled={off(s)}
                  onPress={noop}
                />
              ),
            },
            {
              label: 'Icon=True · Text=False (Dice)',
              render: s => (
                <IconButton
                  variant="primary"
                  big
                  icon="dice"
                  disabled={off(s)}
                  accessibilityLabel="Random"
                  onPress={noop}
                />
              ),
            },
          ]}
        />
        <Block
          label="Small"
          spec={PRIMARY_SPEC.small}
          columns={STATES}
          rows={[
            ...primaryRows({ small: true }),
            {
              label: 'Type=JumpIn · Text=False',
              render: s => (
                <IconButton
                  variant="primary"
                  icon="jumpIn"
                  disabled={off(s)}
                  accessibilityLabel="Jump in"
                  onPress={noop}
                />
              ),
            },
          ]}
        />
      </Section>

      <Section title="Button tone=secondary | outline" figma="Button/Secondary">
        <Block
          label="Size Big"
          spec="Height 96px · Corner 16px · Font 30pt · Icon 32x32"
          columns={STATES}
          rows={secondaryRows({ big: true })}
        />
        <Block
          label="Medium"
          spec="Height 74px · Corner 10px · Font 24pt · Icon 28x28"
          columns={STATES}
          rows={secondaryRows({})}
        />
        <Block
          label="Small"
          spec="Height 54px · Corner 8px · Font 22pt · Icon 24x24"
          columns={STATES}
          rows={secondaryRows({ small: true })}
        />
      </Section>

      <Section title="Button tone=reminder" figma="Button/Reminder">
        <Block
          label="Size Big"
          spec="Height 96px · Corner 12px · Icon 54x54 · click to toggle Selected"
          columns={['Default / Pressed', 'Selected']}
          rows={[
            {
              label: 'Reminder',
              render: s => (
                <Button
                  tone="reminder"
                  icon={on(s) ? 'check' : 'notification'}
                  label="Remind me"
                  selected={on(s)}
                  fullWidth
                  onPress={noop}
                />
              ),
            },
          ]}
        />
      </Section>

      <Section title="LoginButton" figma="Button/Entry">
        <Block
          label="Big"
          spec="Height 96px · Corner 16px · Font 30pt Bold Uppercase · Logo 36x36"
          columns={STATES}
          rows={ENTRY.map(e => ({
            label: e.label,
            render: s => (
              <LoginButton
                provider={e.provider}
                label={e.text}
                disabled={off(s)}
                fullWidth
                onPress={noop}
              />
            ),
          }))}
        />
      </Section>

      <Section
        title="IconButton"
        figma="Button/Discover · Button/Filter · Button/Social"
      >
        <Block
          label="Discover"
          spec="Height 96px · Corner 16px · Icon 52x52 · variant=filled big"
          columns={['Default / Pressed', 'Selected', 'Disabled']}
          rows={[
            {
              label: '',
              render: s => (
                <IconButton
                  big
                  icon="world"
                  selected={on(s)}
                  disabled={off(s)}
                  accessibilityLabel="Discover"
                  onPress={noop}
                />
              ),
            },
          ]}
        />
        <Block
          label="Menu"
          spec="54x54 px · Corner 8px · Icon 40x40 · variant=filled"
          columns={['Default / Pressed', 'Selected', 'Disabled']}
          rows={[
            {
              label: 'Background=On',
              render: s => (
                <IconButton
                  icon="menuDot"
                  selected={on(s)}
                  disabled={off(s)}
                  accessibilityLabel="Menu"
                  onPress={noop}
                />
              ),
            },
            {
              label: 'Background=Off',
              render: s => (
                <IconButton
                  icon="menuDot"
                  background={false}
                  selected={on(s)}
                  disabled={off(s)}
                  accessibilityLabel="Menu"
                  onPress={noop}
                />
              ),
            },
          ]}
        />
        <Block
          label="Social"
          spec="54x54 px · Corner 8px · Icon 40x40 · variant=outline accentColor"
          columns={STATES}
          rows={[
            {
              label: 'Icon=Cancel',
              render: s => (
                <IconButton
                  variant="outline"
                  icon="close"
                  accentColor={colors.ruby}
                  disabled={off(s)}
                  accessibilityLabel="Cancel"
                  onPress={noop}
                />
              ),
            },
            {
              label: 'Icon=Accept',
              render: s => (
                <IconButton
                  variant="outline"
                  icon="check"
                  accentColor={rarityColors.rare.light}
                  disabled={off(s)}
                  accessibilityLabel="Accept"
                  onPress={noop}
                />
              ),
            },
          ]}
        />
      </Section>

      <Section title="PillButton" figma="Button/Pill">
        <Block
          label="Size Big"
          spec="Height 60px · Corner 100 · Font 26pt · Icon 32x32"
          columns={['Default / Pressed', 'Selected']}
          rows={[
            {
              label: 'Icon=True',
              render: s => (
                <PillButton
                  big
                  icon="fav"
                  label="Pill"
                  selected={on(s)}
                  onPress={noop}
                />
              ),
            },
            {
              label: 'Icon=False',
              render: s => (
                <PillButton big label="Pill" selected={on(s)} onPress={noop} />
              ),
            },
          ]}
        />
        <Block
          label="Size Medium"
          spec="Height 56px · Corner 100 · Font 24pt · Icon 32x32"
          columns={['Default / Pressed', 'Selected']}
          rows={[
            {
              label: 'Icon=True',
              render: s => (
                <PillButton
                  icon="fav"
                  label="Pill"
                  selected={on(s)}
                  onPress={noop}
                />
              ),
            },
            {
              label: 'Icon=False',
              render: s => (
                <PillButton label="Pill" selected={on(s)} onPress={noop} />
              ),
            },
            {
              label: 'Notification badge',
              render: s => (
                <PillButton
                  label="Pill"
                  badge
                  selected={on(s)}
                  onPress={noop}
                />
              ),
            },
          ]}
        />
      </Section>

      <Section title="LinkButton" figma="Button/Link">
        <Block
          label="Link"
          spec="Height 54px · Font 24pt Medium Underline · text/link"
          columns={STATES}
          rows={[
            {
              label: '',
              render: s => (
                <View>
                  <LinkButton
                    label="Resend code"
                    disabled={off(s)}
                    onPress={noop}
                  />
                </View>
              ),
            },
          ]}
        />
      </Section>

      <Section title="Button fullWidth" figma="Width — Hug vs Fill">
        <View style={styles.widthDemo}>
          <AppText variant="title">Fill container</AppText>
          <AppText style={styles.asideSpec}>
            The button takes the full available width: fullWidth.
          </AppText>
          <Button big fullWidth label="Button" onPress={noop} />
          <Button fullWidth tone="secondary" label="Button" onPress={noop} />
          <AppText variant="title" style={styles.widthTitle}>
            Hug contents (default)
          </AppText>
          <AppText style={styles.asideSpec}>
            Width fits the text + 24px padding per side.
          </AppText>
          <View style={styles.hugRow}>
            <Button label="Button" icon="check" onPress={noop} />
            <Button tone="secondary" label="Button" onPress={noop} />
          </View>
        </View>
      </Section>
    </View>
  );
}

registerPreview('extra/ds-buttons', {
  title: 'Design system: Buttons page',
  orientation: 'portrait',
  render: () => <DsButtons />,
});

const styles = StyleSheet.create({
  content: {
    paddingHorizontal: u(32),
    paddingVertical: u(48),
    gap: u(120),
    width: '100%',
    maxWidth: u(1260),
    alignSelf: 'center',
  },
  pageTitle: { fontSize: u(36) },
  section: { gap: u(40) },
  sectionTitle: { fontSize: u(24) },
  sectionFigma: { color: colors.lavender70, fontSize: u(16) },
  block: { flexDirection: 'row', gap: u(32), alignItems: 'flex-start' },
  aside: { width: u(170), gap: u(6) },
  asideLabel: { color: colors.snow },
  asideSpec: { color: colors.gray, fontSize: u(11), lineHeight: u(16) },
  grid: { flex: 1, gap: u(12) },
  gridRow: { flexDirection: 'row', gap: u(32), alignItems: 'center' },
  column: { flex: 1, color: colors.lavender, textAlign: 'center' },
  rowLabel: { marginBottom: u(6) },
  cell: {
    flex: 1,
    minWidth: 0,
    alignItems: 'center',
    justifyContent: 'center',
  },
  widthDemo: { gap: u(10), maxWidth: u(360) },
  widthTitle: { marginTop: u(12) },
  hugRow: { flexDirection: 'row', gap: u(8) },
});
