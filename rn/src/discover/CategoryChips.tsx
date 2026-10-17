import { Pressable, ScrollView, StyleSheet, View } from 'react-native';
import { AppText, colors, LinearFill } from '../ds';
import { u } from '../ds/scale';
import { t } from '../i18n';
import { CATEGORIES } from './catalog';
import type { Category } from './types';

/** Row of category tiles (emoji on a violet gradient) that open the category page. */
export function CategoryChips({
  onOpen,
}: {
  onOpen: (category: Category) => void;
}) {
  return (
    <ScrollView
      horizontal
      showsHorizontalScrollIndicator={false}
      contentContainerStyle={styles.row}
      style={styles.scroll}
    >
      {CATEGORIES.map(category => (
        <Pressable
          key={category.id}
          accessibilityRole="button"
          accessibilityLabel={t(category.label)}
          onPress={() => onOpen(category)}
          style={({ pressed }) => [styles.chip, pressed && styles.pressed]}
        >
          <View style={styles.tile}>
            <LinearFill
              angle={180}
              stops={[
                { offset: 0, color: '#C03FD0' },
                { offset: 1, color: '#7B2BB0' },
              ]}
            />
            <AppText variant="heading" style={styles.emoji}>
              {category.emoji}
            </AppText>
          </View>
          <AppText variant="label" style={styles.label}>
            {t(category.label)}
          </AppText>
        </Pressable>
      ))}
    </ScrollView>
  );
}

const styles = StyleSheet.create({
  scroll: { marginTop: u(22) },
  row: { paddingHorizontal: u(24), gap: u(19) },
  chip: { width: u(60), alignItems: 'center' },
  tile: {
    width: u(60),
    height: u(60),
    borderRadius: u(7),
    borderWidth: 0.5,
    borderColor: colors.white30,
    overflow: 'hidden',
    alignItems: 'center',
    justifyContent: 'center',
  },
  emoji: { fontSize: u(30), lineHeight: u(38) },
  label: { marginTop: u(6), fontSize: u(10), fontFamily: 'Inter-SemiBold' },
  pressed: { opacity: 0.85 },
});
