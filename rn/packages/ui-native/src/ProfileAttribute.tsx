import { StyleSheet, View } from 'react-native';
import { AppText } from './AppText';
import { IconProfile, ProfileIconType } from './IconProfile';
import { color, colors, space } from './tokens';
import { u } from './scale';

/** A Profile About field: icon and title (Tags, Lavender Light) over the value (Modal/Body), 150dp
 * wide so two share a row. Renders nothing when the value is empty.
 * Figma Profile/Attribute: https://www.figma.com/design/Jis8xv58qaDNy53OCGLeoh/Design-System-Mobile?node-id=1023-135
 */
export function ProfileAttribute({
  icon,
  title,
  value,
}: {
  icon: ProfileIconType;
  title: string;
  value?: string;
}) {
  if (!value) {
    return null;
  }
  return (
    <View style={styles.attribute}>
      <View style={styles.title}>
        <IconProfile type={icon} size={12} color={colors.lavender} />
        <AppText variant="Tags" style={styles.titleText}>
          {title}
        </AppText>
      </View>
      <AppText variant="Modal/Body" style={styles.value}>
        {value}
      </AppText>
    </View>
  );
}

const styles = StyleSheet.create({
  attribute: { width: u(150), gap: space['3xs'] },
  title: { flexDirection: 'row', alignItems: 'center' },
  titleText: { color: colors.lavender },
  value: { color: color.text.primary },
});
