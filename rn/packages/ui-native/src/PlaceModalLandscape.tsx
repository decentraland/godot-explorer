import { Image, Pressable, StyleSheet, View } from 'react-native';
import { AppText } from './AppText';
import { Button } from './Button';
import { Icon } from './Icon';
import { t } from './i18n';
import { EventBadge, EventTimeKind, EventTimeTag } from './Tags';
import { color, radius, space } from './tokens';
import { u } from './scale';

export type PlaceModalAction = {
  /** `jumpIn` for a place or a live event; `remindMe` / `reminderSet` before an event starts. */
  kind: 'jumpIn' | 'remindMe' | 'reminderSet';
  label: string;
  onPress: () => void;
};

/** Landscape place / event detail: the image on the left (events: time tag and Featured), a panel
 * on the right with title, creator and one action.
 * Figma Modal/PlaceLandscape: https://www.figma.com/design/Jis8xv58qaDNy53OCGLeoh/Design-System-Mobile?node-id=1100-519
 */
export function PlaceModalLandscape({
  imageUri,
  title,
  creator,
  time,
  featured,
  action,
  onClose,
}: {
  imageUri?: string;
  title: string;
  creator: string;
  /** Events only: the time tag on the image. */
  time?: { kind: EventTimeKind; label: string; players?: number };
  /** Events only: the translated "Featured" label. */
  featured?: string;
  action: PlaceModalAction;
  onClose: () => void;
}) {
  return (
    <View style={styles.root}>
      <View style={styles.image}>
        {imageUri && (
          <Image source={{ uri: imageUri }} style={StyleSheet.absoluteFill} />
        )}
        {time && (
          <View style={styles.time}>
            <EventTimeTag {...time} />
          </View>
        )}
        {featured && (
          <View style={styles.featured}>
            <EventBadge type="featured" label={featured} />
          </View>
        )}
      </View>
      <View style={styles.panel}>
        <View style={styles.text}>
          <AppText variant="Modal/Title" style={styles.title}>
            {title}
          </AppText>
          <AppText variant="B1" style={styles.creator}>
            {creator}
          </AppText>
        </View>
        {action.kind === 'jumpIn' ? (
          <Button
            fullWidth
            label={action.label}
            trailingIcon="jumpIn"
            onPress={action.onPress}
          />
        ) : (
          <Button
            fullWidth
            tone="reminder"
            selected={action.kind === 'reminderSet'}
            icon={action.kind === 'reminderSet' ? 'check' : 'notification'}
            label={action.label}
            onPress={action.onPress}
          />
        )}
        <Pressable
          accessibilityRole="button"
          accessibilityLabel={t('APP_CLOSE')}
          onPress={onClose}
          hitSlop={u(8)}
          style={styles.close}
        >
          <Icon name="close" size={27} color={color.icon.default} />
        </Pressable>
      </View>
    </View>
  );
}

const styles = StyleSheet.create({
  root: { flexDirection: 'row', height: u(230) },
  image: {
    width: u(300),
    borderTopLeftRadius: radius.xl,
    borderBottomLeftRadius: radius.xl,
    overflow: 'hidden',
    backgroundColor: color.surface.overlay,
  },
  time: { position: 'absolute', top: space.md, left: space.md },
  featured: { position: 'absolute', top: space.md, right: u(15) },
  panel: {
    width: u(230),
    justifyContent: 'space-between',
    paddingTop: space['64'],
    paddingBottom: space['60'],
    paddingHorizontal: space['40'],
    backgroundColor: color.surface.modal,
    borderTopRightRadius: radius.xl,
    borderBottomRightRadius: radius.xl,
    overflow: 'hidden',
  },
  text: { gap: space.md },
  title: { color: color.text.primary },
  creator: { color: color.text.primary },
  close: { position: 'absolute', top: u(1.5), right: u(1.5) },
});
