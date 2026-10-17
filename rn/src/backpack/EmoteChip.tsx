import { StyleSheet, View } from 'react-native';
import { AppText, colors, Icon } from '../ds';
import { t } from '../i18n';
import { u } from '../ds/scale';

/** Name of the previewed emote above the slot row; "Equipped" replaces it right after an equip. */
export function EmoteChip({
  name,
  equipped,
}: {
  name: string | null;
  equipped: boolean;
}) {
  if (!equipped && !name) {
    return <View style={styles.spacer} />;
  }
  return (
    <View style={styles.wrap} pointerEvents="none">
      <View style={styles.chip} accessibilityLiveRegion="polite">
        <AppText variant="body" style={styles.text}>
          {equipped ? t('BACKPACK_EQUIPPED') : name}
        </AppText>
        {equipped && <Icon name="check" size={13} color={colors.snow} />}
      </View>
    </View>
  );
}

const styles = StyleSheet.create({
  spacer: { height: u(22) },
  wrap: { alignItems: 'center' },
  chip: {
    height: u(22),
    paddingHorizontal: u(8),
    borderRadius: u(6),
    flexDirection: 'row',
    alignItems: 'center',
    gap: u(4),
    backgroundColor: 'rgba(36,3,59,0.8)',
    borderWidth: 1,
    borderColor: colors.white10,
  },
  text: { fontSize: u(11), color: colors.snow },
});
