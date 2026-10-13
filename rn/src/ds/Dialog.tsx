import { ReactNode, useEffect, useState } from 'react';
import { Keyboard, Modal, Pressable, StyleSheet, View } from 'react-native';
import { AppText } from './AppText';
import { LinearFill, RadialFill } from './Gradient';
import { Icon, IconName } from './Icon';
import { PrimaryButton, ButtonTone } from './PrimaryButton';
import { colors } from './tokens';
import { u } from './scale';
import { t } from '../i18n';

export type DialogButton = {
  label: string;
  onPress: () => void;
  tone?: ButtonTone;
  disabled?: boolean;
};

const CARD_STOPS = [
  { offset: 0, color: '#5A1A93' },
  { offset: 1, color: '#2E0750' },
];

const GLOW = [
  { offset: 0, color: '#B83CE0', opacity: 0.85 },
  { offset: 1, color: '#B83CE0', opacity: 0 },
];

/** Height of the on-screen keyboard while `active`, so the card can stay above it. */
function useKeyboardLift(active: boolean): number {
  const [height, setHeight] = useState(0);
  useEffect(() => {
    if (!active) {
      return;
    }
    const show = Keyboard.addListener('keyboardDidShow', e =>
      setHeight(e.endCoordinates.height),
    );
    const hide = Keyboard.addListener('keyboardDidHide', () => setHeight(0));
    return () => {
      show.remove();
      hide.remove();
      setHeight(0);
    };
  }, [active]);
  return height;
}

/** Centered modal card over a dark scrim (no blur library: the scrim is opaque enough to hide the page). */
export function Dialog({
  visible,
  title,
  body,
  icon,
  buttons,
  onRequestClose,
  rounded = false,
  width,
  children,
}: {
  visible: boolean;
  title: string;
  body?: string;
  icon?: IconName;
  buttons: DialogButton[];
  onRequestClose: () => void;
  /** Figma look: 8dp button corners and magenta glows on the card. */
  rounded?: boolean;
  /** Card width in dp (default 310). */
  width?: number;
  children?: ReactNode;
}) {
  const lift = useKeyboardLift(visible);
  return (
    <Modal
      visible={visible}
      transparent
      animationType="fade"
      statusBarTranslucent
      onRequestClose={onRequestClose}
    >
      <View
        style={[
          styles.scrim,
          rounded && styles.scrimLight,
          { paddingBottom: lift },
        ]}
      >
        <Pressable
          accessibilityLabel={t('APP_DISMISS_DIALOG')}
          style={StyleSheet.absoluteFill}
          onPress={onRequestClose}
        />
        <View
          style={[
            styles.card,
            width !== undefined && { maxWidth: u(width) },
            rounded && icon && styles.cardIcon,
          ]}
          accessibilityViewIsModal
        >
          <View style={styles.clip}>
            <LinearFill stops={CARD_STOPS} angle={150} />
            {rounded && (
              <>
                <RadialFill stops={GLOW} cx={1} cy={0} rx={0.7} ry={0.35} />
                <RadialFill
                  stops={GLOW}
                  cx={0.5}
                  cy={1.05}
                  rx={0.6}
                  ry={0.22}
                />
              </>
            )}
          </View>
          {icon && (
            <View style={rounded ? styles.iconRaised : styles.icon}>
              <Icon name={icon} size={28} color={colors.orange} />
            </View>
          )}
          <AppText variant="title" style={styles.title}>
            {title}
          </AppText>
          {body && (
            <AppText variant="body" style={styles.body}>
              {body}
            </AppText>
          )}
          {children}
          {buttons.length > 0 && (
            <View style={styles.buttons}>
              {buttons.map(button => (
                <View key={button.label} style={styles.button}>
                  <PrimaryButton
                    label={button.label}
                    tone={button.tone}
                    rounded={rounded}
                    disabled={button.disabled}
                    onPress={button.onPress}
                  />
                </View>
              ))}
            </View>
          )}
        </View>
      </View>
    </Modal>
  );
}

const styles = StyleSheet.create({
  scrim: {
    flex: 1,
    alignItems: 'center',
    justifyContent: 'center',
    paddingHorizontal: u(25),
    backgroundColor: 'rgba(22,21,24,0.78)',
  },
  scrimLight: { backgroundColor: 'rgba(22,21,24,0.6)' },
  card: {
    alignSelf: 'stretch',
    maxWidth: u(310),
    borderRadius: u(12),
    padding: u(20),
  },
  cardIcon: { paddingTop: u(30) },
  clip: {
    ...StyleSheet.absoluteFill,
    borderRadius: u(12),
    overflow: 'hidden',
  },
  iconRaised: {
    position: 'absolute',
    top: -u(14),
    alignSelf: 'center',
    padding: u(5),
    borderRadius: u(20),
    backgroundColor: colors.snow,
  },
  icon: {
    alignSelf: 'center',
    marginBottom: u(8),
  },
  title: { textAlign: 'center', fontSize: u(18), color: colors.snow },
  body: {
    textAlign: 'center',
    marginTop: u(8),
    fontSize: u(13),
    color: colors.pearl,
  },
  buttons: { flexDirection: 'row', marginTop: u(20), gap: u(8) },
  button: { flex: 1 },
});
