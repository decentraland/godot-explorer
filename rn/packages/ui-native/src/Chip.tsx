import { Pressable, StyleSheet, View } from 'react-native';
import { AppText } from './AppText';
import { Icon, IconName } from './Icon';
import { colors } from './tokens';
import { u } from './scale';

export function Chip({
  label,
  icon,
  onPress,
  onRemove,
}: {
  label: string;
  icon?: IconName;
  onPress?: () => void;
  onRemove?: () => void;
}) {
  return (
    <View style={styles.chip}>
      <Pressable
        accessibilityRole="button"
        accessibilityLabel={label}
        disabled={!onPress}
        onPress={onPress}
        style={styles.chipMain}
      >
        {icon && <Icon name={icon} size={16} color={colors.periwinkle} />}
        <AppText variant="caption" style={styles.chipText}>
          {label}
        </AppText>
      </Pressable>
      {onRemove && (
        <Pressable
          accessibilityRole="button"
          accessibilityLabel={`Remove ${label}`}
          hitSlop={8}
          onPress={onRemove}
        >
          <Icon name="close" size={14} color={colors.periwinkle} />
        </Pressable>
      )}
    </View>
  );
}

const styles = StyleSheet.create({
  chip: {
    height: u(32),
    paddingHorizontal: u(12),
    borderRadius: u(8),
    flexDirection: 'row',
    alignItems: 'center',
    alignSelf: 'flex-start',
    backgroundColor: colors.black40,
    gap: u(8),
  },
  chipMain: { flexDirection: 'row', alignItems: 'center', gap: u(6) },
  chipText: { fontSize: u(14), color: colors.periwinkle },
});
