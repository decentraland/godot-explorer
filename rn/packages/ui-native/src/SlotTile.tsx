import { ReactNode } from 'react';
import { StyleSheet, View } from 'react-native';
import { AppText } from './AppText';
import { ItemCard } from './ItemCard';
import { color, fonts, space } from './tokens';
import { u } from './scale';

/** `Cards-Wearables` is drawn at 96 dp; the slot rows scale it to 62. */
const CARD = 96;
/** The slot rows' selected outline: 3 px, centred on the card edge. */
const SLOT_RING = u(1.5);

/** Backpack category slot: a `Cards-Wearables` at 124 px (what is worn, or the empty "+") and
 * its label 4 px below, which may run wider than the card. */
export function SlotTile({
  label,
  thumbnail,
  selected,
  size: designSize = 62,
  art,
  onPress,
}: {
  label: string;
  thumbnail?: string;
  selected: boolean;
  size?: number;
  /** Drawn in the tile when there is no thumbnail but the slot is worn. */
  art?: ReactNode;
  onPress: () => void;
}) {
  const size = u(designSize);
  return (
    <View style={[styles.slot, { width: size }]}>
      <ItemCard
        label={label}
        thumbnail={thumbnail}
        art={art}
        empty={!thumbnail && !art}
        state={selected ? 'selected' : 'default'}
        ringWidth={SLOT_RING}
        scale={designSize / CARD}
        onPress={onPress}
        style={{ width: size }}
      />
      <AppText numberOfLines={1} style={styles.label}>
        {label}
      </AppText>
    </View>
  );
}

const styles = StyleSheet.create({
  slot: { alignItems: 'center', overflow: 'visible' },
  /** `SecondaryCTA-03`: Inter Medium 24 px, Snow, py 8 px. */
  label: {
    marginTop: space['3xs'],
    paddingVertical: space.xs,
    fontFamily: fonts.medium,
    fontSize: u(12),
    color: color.text.primary,
    textAlign: 'center',
    width: u(88),
  },
});
