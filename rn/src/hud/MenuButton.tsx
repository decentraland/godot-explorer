import { Image, Pressable, StyleSheet, View } from 'react-native';
import { Badge, colors, Icon, u } from '../ds';
import { t } from '../i18n';
import { HUD_SIZES } from './layout';

/** Avatar face in a ringed circle; opens the menu. */
export function MenuButton({
  face,
  badge,
  onPress,
}: {
  face?: string;
  badge?: boolean;
  onPress: () => void;
}) {
  const size = u(HUD_SIZES.menu / 2);
  return (
    <View>
      <Pressable
        accessibilityRole="button"
        accessibilityLabel={t('HUD_MENU')}
        onPress={onPress}
        style={({ pressed }) => [
          styles.ring,
          { width: size, height: size, borderRadius: size / 2 },
          pressed && styles.pressed,
        ]}
      >
        {face ? (
          <Image source={{ uri: face }} style={styles.face} />
        ) : (
          <Icon name="user" size={22} color={colors.silver} />
        )}
      </Pressable>
      {badge ? (
        <View style={styles.badge} pointerEvents="none">
          <Badge />
        </View>
      ) : null}
    </View>
  );
}

const styles = StyleSheet.create({
  ring: {
    alignItems: 'center',
    justifyContent: 'center',
    overflow: 'hidden',
    borderWidth: u(1.5),
    borderColor: colors.lavender,
    backgroundColor: colors.pearl,
  },
  face: { width: '100%', height: '100%' },
  pressed: { opacity: 0.8 },
  badge: { position: 'absolute', top: -u(2), right: -u(2) },
});
