import { ReactNode } from 'react';
import { ImageBackground, Pressable, StyleSheet, View } from 'react-native';
import { useSafeAreaInsets } from 'react-native-safe-area-context';
import { BrandBackground, colors, Icon, u } from '@dcl/ui-native';
import { t } from '../i18n';

const photoBackground = require('../assets/photo-background.png');

/** Full-screen backdrop (the brand `Background` with its glows, plain when `flat`, the lobby's photo background
 * when `photo`) with safe-area padding and an optional back button. `edgeToEdge` skips the
 * safe-area padding: the Figma margin is already generous enough to clear the notch/status bar
 * on its own, so adding insets on top double-pads — for screens with their own exact Figma
 * vertical margins. */
export function FlowFrame({
  children,
  onBack,
  flat = false,
  photo = false,
  edgeToEdge = false,
}: {
  children: ReactNode;
  onBack?: () => void;
  flat?: boolean;
  photo?: boolean;
  edgeToEdge?: boolean;
}) {
  const insets = useSafeAreaInsets();
  return (
    <View style={styles.root}>
      {photo ? (
        <ImageBackground
          source={photoBackground}
          style={StyleSheet.absoluteFill}
          resizeMode="cover"
        />
      ) : flat ? (
        <View style={styles.flat} />
      ) : (
        <BrandBackground glow />
      )}
      <View
        style={[
          styles.content,
          edgeToEdge
            ? null
            : { paddingTop: insets.top, paddingBottom: insets.bottom },
        ]}
      >
        {children}
      </View>
      {onBack ? (
        <Pressable
          accessibilityRole="button"
          accessibilityLabel={t('APP_BACK')}
          onPress={onBack}
          hitSlop={12}
          style={[styles.back, { top: Math.max(insets.top + u(4), u(43)) }]}
        >
          <Icon name="back" size={30} />
        </Pressable>
      ) : null}
    </View>
  );
}

const styles = StyleSheet.create({
  root: { ...StyleSheet.absoluteFill, backgroundColor: colors.purple350 },
  flat: { ...StyleSheet.absoluteFill, backgroundColor: colors.brand },
  content: { flex: 1, paddingHorizontal: u(32) },
  /** The same arrow, size and place as on the avatar screens (Figma Header: 60 px at 48, 86). */
  back: { position: 'absolute', left: u(24) },
});
