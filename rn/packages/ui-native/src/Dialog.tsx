import { ReactNode, useEffect, useState } from 'react';
import {
  Keyboard,
  Modal,
  Pressable,
  StyleSheet,
  useWindowDimensions,
  View,
} from 'react-native';
import { AppText } from './AppText';
import { RadialFill } from './Gradient';
import { Icon } from './Icon';
import { Button, ButtonTone } from './Button';
import { colors, fonts } from './tokens';
import { u } from './scale';
import { t } from './i18n';

export type DialogButton = {
  label: string;
  onPress: () => void;
  tone?: ButtonTone;
  disabled?: boolean;
};

/** The `Icon/Modal` illustration shown on top of the card. */
export type DialogIcon =
  | 'alert'
  | 'banned'
  | 'block'
  | 'bug'
  | 'connection'
  | 'password'
  | 'report'
  | 'success';

const MODAL_ICON = {
  alert: 'modalAlert',
  banned: 'modalBanned',
  block: 'modalBlock',
  bug: 'modalBug',
  connection: 'modalConnection',
  password: 'modalPassword',
  report: 'modalReport',
  success: 'modalSuccess',
} as const;

/** `surface/modal-glow`: two blurred Mythic blobs, the top one opaque and the bottom one at 50%. */
const GLOW_TOP = [
  { offset: 0, color: colors.mythic, opacity: 0.85 },
  { offset: 1, color: colors.mythic, opacity: 0 },
];
const GLOW_BOTTOM = [
  { offset: 0, color: colors.mythic, opacity: 0.5 },
  { offset: 1, color: colors.mythic, opacity: 0 },
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

/** Figma `Modal/Dialog` over `Modal/Scrim`: Portrait card 520 wide, Landscape 720, `surface/modal`
 * with the glow, an optional `Icon/Modal` overlapping the top edge, and a Secondary (hug) +
 * Primary (fill) CTA row. The scrim is Black 60% (the no-blur fallback); tapping it closes. */
export function Dialog({
  visible,
  title,
  body,
  link,
  icon,
  buttons,
  onRequestClose,
  children,
}: {
  visible: boolean;
  title: string;
  body?: string;
  /** Figma `Link=True`: an underlined sky-blue line under the body (two lines at most). */
  link?: { label: string; onPress: () => void };
  icon?: DialogIcon;
  buttons: DialogButton[];
  onRequestClose: () => void;
  children?: ReactNode;
}) {
  const lift = useKeyboardLift(visible);
  const { width, height } = useWindowDimensions();
  const landscape = width > height;
  return (
    <Modal
      visible={visible}
      transparent
      animationType="fade"
      statusBarTranslucent
      onRequestClose={onRequestClose}
    >
      <View style={[styles.scrim, { paddingBottom: lift }]}>
        <Pressable
          accessibilityLabel={t('APP_DISMISS_DIALOG')}
          style={StyleSheet.absoluteFill}
          onPress={onRequestClose}
        />
        <View
          style={[
            styles.card,
            landscape ? styles.cardLandscape : styles.cardPortrait,
          ]}
          accessibilityViewIsModal
        >
          <View style={styles.clip}>
            <RadialFill stops={GLOW_TOP} cx={1} cy={0} rx={0.7} ry={0.35} />
            <RadialFill
              stops={GLOW_BOTTOM}
              cx={0.5}
              cy={1.05}
              rx={0.6}
              ry={0.22}
            />
          </View>
          <View style={[styles.content, landscape && styles.contentLandscape]}>
            <View style={styles.text}>
              <AppText style={styles.title}>{title}</AppText>
              {body && <AppText style={styles.body}>{body}</AppText>}
              {link && (
                <Pressable accessibilityRole="link" onPress={link.onPress}>
                  <AppText numberOfLines={2} style={styles.link}>
                    {link.label}
                  </AppText>
                </Pressable>
              )}
            </View>
            {children}
            {buttons.length > 0 && (
              <View
                style={[styles.buttons, landscape && styles.buttonsLandscape]}
              >
                {buttons.map((button, i) => (
                  <View
                    key={button.label}
                    style={i === buttons.length - 1 && styles.grow}
                  >
                    <Button
                      label={button.label}
                      tone={button.tone}
                      disabled={button.disabled}
                      fullWidth={i === buttons.length - 1}
                      onPress={button.onPress}
                    />
                  </View>
                ))}
              </View>
            )}
          </View>
          {icon && (
            <View style={styles.icon}>
              <Icon name={MODAL_ICON[icon]} size={45} />
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
    backgroundColor: colors.black60,
  },
  card: {
    borderRadius: u(8),
    backgroundColor: colors.modal,
  },
  cardPortrait: {
    width: u(260),
    paddingTop: u(32),
    paddingBottom: u(30),
    paddingHorizontal: u(28),
  },
  cardLandscape: {
    width: u(360),
    paddingTop: u(27),
    paddingBottom: u(24),
    paddingHorizontal: u(27),
  },
  clip: {
    ...StyleSheet.absoluteFill,
    borderRadius: u(8),
    overflow: 'hidden',
  },
  content: { gap: u(27) },
  contentLandscape: { gap: u(24) },
  text: { gap: u(8) },
  title: {
    fontFamily: fonts.bold,
    fontSize: u(19),
    color: colors.snow,
    textAlign: 'center',
  },
  body: {
    fontSize: u(13),
    lineHeight: u(18),
    color: colors.snow,
    textAlign: 'center',
  },
  link: {
    marginTop: u(5),
    fontSize: u(10),
    color: colors.sky,
    textAlign: 'center',
    textDecorationLine: 'underline',
  },
  buttons: { flexDirection: 'row', alignItems: 'center', gap: u(16) },
  buttonsLandscape: { gap: u(20) },
  grow: { flex: 1 },
  icon: {
    position: 'absolute',
    top: -u(22.5),
    left: 0,
    right: 0,
    alignItems: 'center',
  },
});
