import { Image, StyleSheet, View } from 'react-native';
import { AppText } from './AppText';
import { Button } from './Button';
import { Icon } from './Icon';
import { IconButton } from './IconButton';
import { alpha, color, colors, space } from './tokens';
import { u } from './scale';

export type FriendAction =
  | { kind: 'jumpIn'; label: string; onPress: () => void }
  | { kind: 'addFriend'; label: string; onPress: () => void }
  | {
      kind: 'request';
      acceptLabel: string;
      declineLabel: string;
      onAccept: () => void;
      onDecline: () => void;
    };

/** Friends list row: avatar, name (+ verified), place and one action. `offline` dims the avatar,
 * `blocked` draws the red ring.
 * Figma List/Friend: https://www.figma.com/design/Jis8xv58qaDNy53OCGLeoh/Design-System-Mobile?node-id=1086-184
 */
export function FriendRow({
  name,
  place,
  avatarUri,
  avatarBackdrop = colors.exoticTint,
  nameColor = color.text.primary,
  verified = false,
  offline = false,
  blocked = false,
  action,
}: {
  name: string;
  place?: string;
  avatarUri?: string;
  avatarBackdrop?: string;
  /** The player's own name colour. */
  nameColor?: string;
  verified?: boolean;
  offline?: boolean;
  blocked?: boolean;
  action?: FriendAction;
}) {
  return (
    <View style={styles.row}>
      <View style={styles.player}>
        <View style={[styles.avatar, { backgroundColor: avatarBackdrop }]}>
          {avatarUri && (
            <Image source={{ uri: avatarUri }} style={styles.avatarImage} />
          )}
          {offline && <View style={styles.offline} />}
          {blocked && (
            <View style={styles.blocked}>
              <View style={styles.slash} />
            </View>
          )}
        </View>
        <View style={styles.text}>
          <View style={styles.name}>
            <AppText
              variant="B4"
              style={{ color: nameColor }}
              numberOfLines={1}
            >
              {name}
            </AppText>
            {verified && <Icon name="nameVerified" size={12} />}
          </View>
          {place !== undefined && (
            <AppText variant="Caption3" style={styles.place} numberOfLines={1}>
              {place}
            </AppText>
          )}
        </View>
      </View>
      {action?.kind === 'jumpIn' && (
        <IconButton
          icon="jumpIn"
          variant="primary"
          accessibilityLabel={action.label}
          onPress={action.onPress}
        />
      )}
      {action?.kind === 'addFriend' && (
        <Button
          small
          label={action.label}
          icon="menuAddFriend"
          onPress={action.onPress}
        />
      )}
      {action?.kind === 'request' && (
        <View style={styles.actions}>
          <IconButton
            icon="check"
            variant="outline"
            accessibilityLabel={action.acceptLabel}
            onPress={action.onAccept}
          />
          <IconButton
            icon="close"
            variant="outline"
            accentColor={colors.ruby}
            accessibilityLabel={action.declineLabel}
            onPress={action.onDecline}
          />
        </View>
      )}
    </View>
  );
}

const AVATAR = u(30);

const styles = StyleSheet.create({
  row: {
    flexDirection: 'row',
    alignItems: 'center',
    justifyContent: 'space-between',
    paddingVertical: u(12.5),
  },
  player: {
    flexDirection: 'row',
    alignItems: 'center',
    gap: space.md,
    flexShrink: 1,
  },
  avatar: {
    width: AVATAR,
    height: AVATAR,
    borderRadius: AVATAR / 2,
    borderWidth: 1,
    borderColor: colors.white50,
    alignItems: 'center',
    justifyContent: 'center',
    overflow: 'hidden',
  },
  avatarImage: { width: u(27), height: u(27) },
  offline: {
    ...StyleSheet.absoluteFill,
    backgroundColor: alpha['black-60'],
  },
  blocked: {
    ...StyleSheet.absoluteFill,
    borderRadius: AVATAR / 2,
    borderWidth: u(1.6),
    borderColor: colors.danger,
    alignItems: 'center',
    justifyContent: 'center',
  },
  slash: {
    width: u(27),
    height: u(1.6),
    backgroundColor: colors.danger,
    transform: [{ rotate: '45deg' }],
  },
  text: { gap: u(1), flexShrink: 1 },
  name: { flexDirection: 'row', alignItems: 'center', gap: space['2xs'] },
  place: { color: color.text.secondary },
  actions: { flexDirection: 'row', gap: space.lg },
});
