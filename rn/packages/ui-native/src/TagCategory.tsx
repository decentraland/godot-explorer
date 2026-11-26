import { StyleSheet, View } from 'react-native';
import { AppText } from './AppText';
import { Icon } from './Icon';
import { categoryPaths } from './iconSvgs';
import { alpha, color, control, radius, space } from './tokens';

export type CategoryType = keyof typeof categoryPaths.paths;

/** Discover category pill: the category glyph and its name in `Tags`. `icon={false}` is the
 * text-only variant. The caller passes the translated label.
 * Figma Tag/Category: https://www.figma.com/design/Jis8xv58qaDNy53OCGLeoh/Design-System-Mobile?node-id=35-591
 */
export function TagCategory({
  type,
  label,
  icon = true,
}: {
  type: CategoryType;
  label: string;
  icon?: boolean;
}) {
  return (
    <View style={styles.tag}>
      {icon && (
        <Icon
          name={
            `category${type[0].toUpperCase()}${type.slice(
              1,
            )}` as `category${Capitalize<CategoryType>}`
          }
          size={13}
          color={color.text.primary}
        />
      )}
      <AppText variant="Tags">{label}</AppText>
    </View>
  );
}

const styles = StyleSheet.create({
  tag: {
    flexDirection: 'row',
    alignItems: 'center',
    alignSelf: 'flex-start',
    height: control.tag,
    gap: space['2xs'],
    paddingVertical: space.xs,
    paddingHorizontal: space['10'],
    borderRadius: radius.pill,
    backgroundColor: alpha['white-10'],
  },
});
