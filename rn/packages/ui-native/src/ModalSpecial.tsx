import { ReactNode } from 'react';
import { Modal, Pressable, StyleSheet, View } from 'react-native';
import { AppText } from './AppText';
import { Button } from './Button';
import { RadialFill } from './Gradient';
import { t } from './i18n';
import { color, colors, radius, space } from './tokens';
import { u } from './scale';

type ModalSpecialBase = { visible: boolean; onRequestClose: () => void };

export type ModalSpecialProps = ModalSpecialBase &
  (
    | {
        type?: 'default';
        media?: ReactNode;
        title: string;
        body: string;
        highlight?: string;
        primaryLabel: string;
        onPrimary: () => void;
        secondaryButton?: boolean;
        secondaryLabel?: string;
        onSecondary?: () => void;
      }
    | {
        /** Figma `Type=Custom`: only the background, border and glow; `children` fill the card. */
        type: 'custom';
        children: ReactNode;
        /** Card height in dp; Figma's frame is 503. */
        height?: number;
      }
  );

/** Marketing modal for campaigns, rewards and launches, over the scrim. `Default`: the campaign
 * `media` on top (312x225, may bleed under the text), the title and body (`highlight` in bold
 * Lavender Light at the end), Button/Primary and, unless `secondaryButton` is off,
 * Button/Secondary to dismiss. `Custom`: the same card with the caller's own layout.
 * Figma Modal/Special: https://www.figma.com/design/Jis8xv58qaDNy53OCGLeoh/Design-System-Mobile?node-id=1122-421
 */
export function ModalSpecial(props: ModalSpecialProps) {
  const { visible, onRequestClose } = props;
  return (
    <Modal
      visible={visible}
      transparent
      animationType="fade"
      statusBarTranslucent
      // iOS defaults to portrait and aborts when the screen is locked to landscape (in world).
      supportedOrientations={['portrait', 'landscape']}
      onRequestClose={onRequestClose}
    >
      <View style={styles.scrim}>
        <Pressable
          accessibilityLabel={t('APP_DISMISS_DIALOG')}
          style={StyleSheet.absoluteFill}
          onPress={onRequestClose}
        />
        <View style={styles.card} accessibilityViewIsModal>
          <View style={styles.glow} pointerEvents="none">
            <RadialFill
              stops={[
                {
                  offset: 0,
                  color: color.surface['modal-special-glow'],
                  opacity: 0.55,
                },
                {
                  offset: 0.35,
                  color: color.surface['modal-special-glow'],
                  opacity: 0.3,
                },
                {
                  offset: 0.7,
                  color: color.surface['modal-special-glow'],
                  opacity: 0.08,
                },
                {
                  offset: 1,
                  color: color.surface['modal-special-glow'],
                  opacity: 0,
                },
              ]}
            />
          </View>
          {props.type === 'custom' ? (
            <View style={{ height: props.height ?? u(503) }}>
              {props.children}
            </View>
          ) : (
            <DefaultContent {...props} />
          )}
        </View>
      </View>
    </Modal>
  );
}

function DefaultContent({
  media,
  title,
  body,
  highlight,
  primaryLabel,
  onPrimary,
  secondaryButton = true,
  secondaryLabel,
  onSecondary,
}: Extract<ModalSpecialProps, { title: string }>) {
  return (
    <>
      <View style={styles.media}>{media}</View>
      <View style={styles.content}>
        <View style={styles.texts}>
          <AppText variant="Modal/Special Title" style={styles.title}>
            {title}
          </AppText>
          <AppText variant="Modal/Special Body" style={styles.body}>
            {body}
            {highlight ? (
              <AppText variant="Modal/Special Body" style={styles.highlight}>
                {` ${highlight}`}
              </AppText>
            ) : null}
          </AppText>
        </View>
        <View style={styles.ctas}>
          <Button label={primaryLabel} onPress={onPrimary} fullWidth />
          {secondaryButton && secondaryLabel && onSecondary ? (
            <Button
              label={secondaryLabel}
              onPress={onSecondary}
              tone="secondary"
              fullWidth
            />
          ) : null}
        </View>
      </View>
    </>
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
    width: u(312),
    overflow: 'hidden',
    borderRadius: radius.xl,
    borderWidth: u(0.5),
    borderColor: colors.purple350,
    backgroundColor: color.surface['modal-special'],
  },
  // Figma's 333x76 ellipse blurred by ~97dp, centred 5dp under the card's bottom edge.
  glow: {
    position: 'absolute',
    left: u(-60),
    right: u(-60),
    bottom: u(-125),
    height: u(240),
  },
  media: { height: u(225), overflow: 'hidden' },
  content: {
    alignItems: 'center',
    gap: space['54'],
    paddingTop: space.xl,
    paddingBottom: space['40'],
    paddingHorizontal: space['50'],
  },
  texts: { alignSelf: 'stretch', gap: space['14'] },
  title: { color: colors.white, textAlign: 'center' },
  body: { color: color.text.primary, textAlign: 'center' },
  highlight: { fontFamily: 'Inter-Bold', color: colors.lavender },
  ctas: { alignSelf: 'stretch', gap: space.xl },
});
