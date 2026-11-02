import { StyleSheet, View } from 'react-native';
import { ItemCard, u } from '@dcl/ui-native';
import { t } from '../i18n';
import { EmoteArt } from './ItemArt';
import { emoteStrip } from './emoteLogic';
import { Emote } from './emotes';
import { EmptyBlock } from './GridStates';
import { ItemGrid } from './ItemGrid';
import type { EmptyCopy } from './platformPolicy';

/** Grid of emotes; tile strips follow the selected slot (UNEQUIP / check / EQUIP). */
export function EmotesGrid({
  emotes,
  slots,
  selected,
  pending,
  columns,
  bottomInset,
  copy,
  onPress,
  onMarketplace,
}: {
  emotes: Emote[];
  slots: string[];
  selected: number | null;
  pending: string | null;
  columns?: number;
  bottomInset?: number;
  copy: EmptyCopy;
  onPress: (emote: Emote) => void;
  onMarketplace: () => void;
}) {
  return (
    <ItemGrid
      items={emotes}
      keyOf={emote => emote.id}
      columns={columns}
      bottomInset={bottomInset}
      header={<View style={styles.top} />}
      empty={
        <EmptyBlock icon="emotes" copy={copy} onMarketplace={onMarketplace} />
      }
      renderItem={emote => {
        const strip = emoteStrip(emote.id, slots, selected, pending);
        return (
          <View>
            <ItemCard
              label={emote.name}
              rarity={emote.rarity}
              thumbnail={emote.thumbnail}
              art={<EmoteArt id={emote.id} />}
              equipped={strip === 'unequip'}
              caption={t('BACKPACK_UNEQUIP')}
              outline={
                strip === 'equip'
                  ? 'orange'
                  : strip === 'check'
                  ? 'lavender'
                  : undefined
              }
              strip={
                strip === 'equip'
                  ? { label: t('BACKPACK_EQUIP'), tone: 'ruby' }
                  : strip === 'check'
                  ? { icon: 'check', tone: 'muted' }
                  : undefined
              }
              onPress={() => onPress(emote)}
            />
          </View>
        );
      }}
    />
  );
}

const styles = StyleSheet.create({ top: { height: u(12) } });
