import { ScrollView, StyleSheet, View } from 'react-native';
import { Icon, ProfileCard, u } from '@dcl/ui-native';
import type { EquippedItem } from './types';

/** Horizontal strip of equipped wearables and emotes as Figma `Card-Profile` cards. */
export function EquippedRow({
  items,
  onPress,
}: {
  items: EquippedItem[];
  onPress?: (item: EquippedItem) => void;
}) {
  return (
    <ScrollView
      horizontal
      showsHorizontalScrollIndicator={false}
      contentContainerStyle={styles.row}
    >
      {items.map(item => (
        <ProfileCard
          key={item.id}
          type={item.isEmote ? 'emote' : 'wearable'}
          title={item.name}
          thumbnail={item.thumbnail}
          rarity={item.rarity}
          art={
            item.isEmote ? (
              <View style={styles.silhouette}>
                <Icon
                  name="emotes"
                  size={40}
                  color="#0B0412"
                  strokeWidth={2.2}
                />
              </View>
            ) : undefined
          }
          onPress={onPress ? () => onPress(item) : undefined}
        />
      ))}
    </ScrollView>
  );
}

const styles = StyleSheet.create({
  silhouette: {
    ...StyleSheet.absoluteFill,
    alignItems: 'center',
    justifyContent: 'center',
  },
  row: { gap: u(12) },
});
