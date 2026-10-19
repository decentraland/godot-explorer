import { Linking, StyleSheet, View } from 'react-native';
import { Chip } from '../ds';
import { u } from '../ds/scale';
import type { ProfileLink } from './types';

/** Wrapping link chips; tapping opens the URL, `onRemove` adds the delete X (Edit Profile). */
export function LinkChips({
  links,
  onRemove,
}: {
  links: ProfileLink[];
  onRemove?: (index: number) => void;
}) {
  return (
    <View style={styles.wrap}>
      {links.map((link, index) => (
        <Chip
          key={`${link.title}:${link.url}`}
          label={link.title}
          icon="link"
          onPress={
            onRemove
              ? undefined
              : () => {
                  Linking.openURL(link.url);
                }
          }
          onRemove={onRemove ? () => onRemove(index) : undefined}
        />
      ))}
    </View>
  );
}

const styles = StyleSheet.create({
  wrap: { flexDirection: 'row', flexWrap: 'wrap', gap: u(12) },
});
