import { StyleSheet, View } from 'react-native';
import { AppText, colors } from '../ds';
import { u } from '../ds/scale';
import { t } from '../i18n';
import { SheetShell } from './SheetShell';
import { ActionRow, Byline, Hero, JumpButton } from './SheetParts';
import type { Friend } from './types';

const FRACTION = 0.605;

/** A friend and the place they are in; JUMP IN is the only action. */
export function FriendSheet({
  friend,
  downloadMb,
  onClose,
  onJumpIn,
}: {
  friend: Friend;
  downloadMb: number | null;
  onClose: () => void;
  onJumpIn: () => void;
}) {
  const { place } = friend;
  return (
    <SheetShell
      fraction={FRACTION}
      expandable={false}
      onClose={onClose}
      footer={
        <ActionRow>
          <JumpButton glyph={false} onPress={onJumpIn} />
        </ActionRow>
      }
    >
      {() => (
        <View>
          <View style={styles.who}>
            <View style={[styles.avatar, { backgroundColor: friend.color }]}>
              <AppText variant="button" style={styles.initial}>
                {friend.name.charAt(0).toUpperCase()}
              </AppText>
            </View>
            <AppText variant="title" style={styles.name}>
              {friend.name}
              {friend.tag ? (
                <AppText
                  variant="caption"
                  style={styles.tag}
                >{` ${friend.tag}`}</AppText>
              ) : null}
            </AppText>
          </View>
          {place ? (
            <>
              <AppText variant="title" style={styles.title}>
                {place.title}
              </AppText>
              <Byline creator={place.creator} />
              <Hero image={place.image} downloadMb={downloadMb} />
            </>
          ) : (
            <AppText variant="body" style={styles.offline}>
              {t('DISCOVER_NOTHING_HERE')}
            </AppText>
          )}
        </View>
      )}
    </SheetShell>
  );
}

const styles = StyleSheet.create({
  who: {
    flexDirection: 'row',
    alignItems: 'center',
    justifyContent: 'center',
    gap: u(10),
    marginTop: u(6),
  },
  avatar: {
    width: u(40),
    height: u(40),
    borderRadius: u(20),
    borderWidth: 1.5,
    borderColor: colors.snow,
    alignItems: 'center',
    justifyContent: 'center',
  },
  initial: { color: colors.purple450 },
  name: { fontSize: u(17) },
  tag: { fontSize: u(12), color: colors.white70 },
  title: {
    marginTop: u(20),
    fontSize: u(20.5),
    lineHeight: u(25.5),
    fontFamily: 'Inter-SemiBold',
  },
  offline: { marginTop: u(20), color: colors.white70 },
});
