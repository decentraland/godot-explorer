import { Image, Pressable, StyleSheet, Text, View } from 'react-native';
import { colors, fonts, Icon, u } from '../ds';
import type { ChatProfile } from './types';
import { fullName } from './useChatFeed';

/** Up to three mention suggestions above the chat input. */
export function MentionList({
  profiles,
  onPick,
}: {
  profiles: ChatProfile[];
  onPick: (fullName: string) => void;
}) {
  if (!profiles.length) {
    return null;
  }
  return (
    <View style={styles.list}>
      {profiles.map(p => (
        <Pressable
          key={p.address}
          accessibilityRole="button"
          accessibilityLabel={fullName(p)}
          onPress={() => onPick(fullName(p))}
          style={styles.item}
        >
          {p.face ? (
            <Image source={{ uri: p.face }} style={styles.face} />
          ) : (
            <View style={[styles.face, styles.facePlaceholder]}>
              <Icon name="user" size={9} color={colors.silver} />
            </View>
          )}
          <Text style={[styles.name, { color: p.color }]}>{p.name}</Text>
          {p.tag ? <Text style={styles.tag}>{p.tag}</Text> : null}
          {p.claimed ? (
            <Icon name="seal" size={9} color={colors.mythic} />
          ) : null}
        </Pressable>
      ))}
    </View>
  );
}

const styles = StyleSheet.create({
  list: {
    backgroundColor: 'rgba(0,0,0,0.85)',
    borderRadius: u(4),
    marginBottom: u(3),
  },
  item: {
    flexDirection: 'row',
    alignItems: 'center',
    gap: u(4),
    padding: u(4),
  },
  face: { width: u(14), height: u(14), borderRadius: u(7) },
  facePlaceholder: {
    backgroundColor: colors.pearl,
    alignItems: 'center',
    justifyContent: 'center',
  },
  name: { fontFamily: fonts.semibold, fontSize: u(9.33) },
  tag: {
    fontFamily: fonts.regular,
    fontSize: u(9.33),
    color: 'rgba(255,255,255,0.6)',
  },
});
