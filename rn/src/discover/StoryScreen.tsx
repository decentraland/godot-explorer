import { Pressable, StyleSheet, View } from 'react-native';
import { useSafeAreaInsets } from 'react-native-safe-area-context';
import { AppText, colors, Icon, LinearFill } from '../ds';
import { u } from '../ds/scale';
import { t } from '../i18n';
import { Place } from './api';
import { getFlag } from './flags';
import { Thumb } from './Thumb';

/**
 * Story viewer stub: a full-bleed place image with back and JUMP IN. Dark behind the `storyViewer`
 * flag until the story media source, entry point and paging exist.
 */
export function StoryScreen({
  place,
  onBack,
  onJump,
  enabled,
}: {
  place: Place;
  onBack: () => void;
  onJump: (place: Place) => void;
  enabled?: boolean;
}) {
  const insets = useSafeAreaInsets();
  if (!(enabled ?? getFlag('storyViewer'))) {
    return null;
  }
  return (
    <View style={styles.root}>
      <Thumb uri={place.image} width="100%" height="100%" radius={0} />
      <View style={styles.scrim} pointerEvents="none">
        <LinearFill
          angle={180}
          stops={[
            { offset: 0, color: '#000000', opacity: 0 },
            { offset: 1, color: '#000000', opacity: 0.8 },
          ]}
        />
      </View>
      <Pressable
        accessibilityRole="button"
        accessibilityLabel={t('STORY_BACK')}
        onPress={onBack}
        style={[styles.back, { top: insets.top + u(5) }]}
      >
        <Icon name="chevronLeft" size={18} strokeWidth={2.2} />
      </Pressable>
      <View style={[styles.info, { paddingBottom: insets.bottom + u(20) }]}>
        <AppText variant="heading" numberOfLines={2} style={styles.title}>
          {place.title}
        </AppText>
        <AppText variant="body" style={styles.by}>
          {t('COMMON_BY')}{' '}
          <AppText variant="body" style={styles.creator}>
            {place.creator}
          </AppText>
        </AppText>
        <Pressable
          accessibilityRole="button"
          onPress={() => onJump(place)}
          style={styles.jump}
        >
          <AppText variant="button" style={styles.jumpText}>
            {t('COMMON_JUMP_IN')}
          </AppText>
        </Pressable>
      </View>
    </View>
  );
}

const styles = StyleSheet.create({
  root: { ...StyleSheet.absoluteFill, backgroundColor: colors.shadow },
  scrim: { position: 'absolute', left: 0, right: 0, bottom: 0, height: '45%' },
  back: {
    position: 'absolute',
    left: u(24),
    width: u(30),
    height: u(30),
    borderRadius: u(4),
    alignItems: 'center',
    justifyContent: 'center',
    backgroundColor: 'rgba(0,0,0,0.5)',
  },
  info: { position: 'absolute', left: u(24), right: u(24), bottom: 0 },
  title: { fontSize: u(24), lineHeight: u(27) },
  by: { marginTop: u(6), fontSize: u(14), color: colors.lavender },
  creator: {
    fontSize: u(14),
    fontFamily: 'Inter-SemiBold',
    color: colors.snow,
  },
  jump: {
    marginTop: u(14),
    height: u(47),
    borderRadius: u(6),
    alignItems: 'center',
    justifyContent: 'center',
    backgroundColor: colors.ruby,
  },
  jumpText: { fontFamily: 'Inter-Bold', fontSize: u(14) },
});
