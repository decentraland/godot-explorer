import { Pressable, StyleSheet, View } from 'react-native';
import { AppText, colors, Icon, LinearFill, RadialFill, u } from '../ds';
import type { DiscoverEvent, Place } from '../discover/api';
import { t } from '../i18n';
import { EventTags, Thumb } from './Cards';
import { UsersTag } from './Tags';

export type ModalTarget =
  | { kind: 'place'; place: Place }
  | { kind: 'event'; event: DiscoverEvent; featured: boolean };

type Cta = 'jump' | 'remind' | 'set' | 'signIn';

/** Live events and places jump in; upcoming events set a reminder (guests are asked to sign in). */
export function ctaFor(
  target: ModalTarget,
  reminderSet: boolean,
  guest: boolean,
): Cta {
  if (target.kind === 'place' || target.event.live) {
    return 'jump';
  }
  if (guest) {
    return 'signIn';
  }
  return reminderSet ? 'set' : 'remind';
}

function CtaButton({ cta, onPress }: { cta: Cta; onPress: () => void }) {
  const filled = cta === 'jump';
  const color = cta === 'set' ? colors.ruby : colors.snow;
  const label = {
    jump: t('MENU_JUMP_IN'),
    remind: t('MENU_REMIND_ME'),
    set: t('MENU_REMINDER_SET'),
    signIn: t('MENU_SIGN_IN_FOR_REMINDERS'),
  }[cta];
  const icon = { jump: 'jump', remind: 'bell', set: 'check', signIn: null }[
    cta
  ];
  return (
    <Pressable
      accessibilityRole="button"
      accessibilityLabel={label}
      accessibilityState={{ disabled: cta === 'signIn' }}
      disabled={cta === 'signIn'}
      onPress={onPress}
      style={({ pressed }) => [
        styles.cta,
        filled
          ? styles.ctaFilled
          : { borderColor: cta === 'set' ? colors.ruby : colors.snow },
        cta === 'signIn' && styles.ctaDisabled,
        pressed && styles.pressed,
      ]}
    >
      {icon === 'check' && <Icon name="check" size={14} color={color} />}
      {icon === 'bell' && (
        <Icon name="bell" size={14} color={color} strokeWidth={2} />
      )}
      <AppText variant="button" style={[styles.ctaText, { color }]}>
        {label}
      </AppText>
      {icon === 'jump' && (
        <Icon name="jump" size={14} color={color} strokeWidth={2} />
      )}
    </Pressable>
  );
}

export function PlaceModal({
  target,
  now,
  reminderSet,
  guest,
  onClose,
  onJumpIn,
  onToggleReminder,
}: {
  target: ModalTarget;
  now: number;
  reminderSet: boolean;
  guest: boolean;
  onClose: () => void;
  onJumpIn: (place: Place) => void;
  onToggleReminder: () => void;
}) {
  const isEvent = target.kind === 'event';
  const place = isEvent ? target.event.place : target.place;
  const title = isEvent ? target.event.title : target.place.title;
  const creator = isEvent ? target.event.creator : target.place.creator;
  const image = isEvent ? target.event.image : target.place.image;
  const cta = ctaFor(target, reminderSet, guest);

  return (
    <View style={styles.scrim}>
      <Pressable
        accessibilityLabel={t('MENU_CLOSE')}
        style={StyleSheet.absoluteFill}
        onPress={onClose}
      />
      <View style={styles.card} accessibilityViewIsModal>
        <View style={styles.image}>
          <Thumb uri={image} width={u(300)} height={u(230)} radius={0} />
          {isEvent ? (
            <EventTags
              event={target.event}
              featured={target.featured}
              now={now}
            />
          ) : (
            target.place.users > 0 && (
              <View style={styles.tags} pointerEvents="none">
                <UsersTag count={target.place.users} />
              </View>
            )
          )}
        </View>
        <View style={styles.info}>
          <LinearFill
            stops={[
              { offset: 0, color: '#4F1580' },
              { offset: 1, color: '#521A82' },
            ]}
          />
          <RadialFill
            cy={1}
            rx={0.8}
            ry={0.3}
            stops={[
              { offset: 0, color: '#8F2BAE', opacity: 0.9 },
              { offset: 1, color: '#4F1580', opacity: 0 },
            ]}
          />
          <Pressable
            accessibilityRole="button"
            accessibilityLabel={t('MENU_CLOSE')}
            hitSlop={u(8)}
            onPress={onClose}
            style={styles.close}
          >
            <Icon name="close" size={16} color={colors.snow} />
          </Pressable>
          <AppText variant="heading" numberOfLines={2} style={styles.title}>
            {title}
          </AppText>
          <AppText variant="body" numberOfLines={1} style={styles.by}>
            {t('MENU_BY_CREATOR_MODAL', { creator })}
          </AppText>
          <View style={styles.ctaSlot}>
            <CtaButton
              cta={cta}
              onPress={
                cta === 'jump' ? () => onJumpIn(place) : onToggleReminder
              }
            />
          </View>
        </View>
      </View>
    </View>
  );
}

const styles = StyleSheet.create({
  scrim: {
    ...StyleSheet.absoluteFill,
    alignItems: 'center',
    justifyContent: 'center',
    backgroundColor: 'rgba(22,21,24,0.62)',
  },
  card: {
    width: u(530),
    height: u(230),
    flexDirection: 'row',
    borderRadius: u(10),
    overflow: 'hidden',
  },
  image: { width: u(300), height: u(230), backgroundColor: '#D9D9D9' },
  tags: { position: 'absolute', left: u(8), right: u(8), top: u(8) },
  info: { flex: 1, padding: u(20), overflow: 'hidden' },
  close: { position: 'absolute', top: u(15), right: u(15) },
  title: {
    fontSize: u(20),
    lineHeight: u(26),
    marginTop: u(2),
    paddingRight: u(20),
  },
  by: { fontSize: u(14), color: 'rgba(255,255,255,0.85)', marginTop: u(8) },
  ctaSlot: { position: 'absolute', left: u(20), right: u(20), bottom: u(20) },
  cta: {
    height: u(37),
    borderRadius: u(7),
    borderWidth: 1,
    borderColor: 'transparent',
    flexDirection: 'row',
    alignItems: 'center',
    justifyContent: 'center',
    gap: u(6),
  },
  ctaFilled: { backgroundColor: colors.ruby },
  ctaDisabled: { opacity: 0.5 },
  ctaText: { fontFamily: 'Inter-Bold', fontSize: u(14) },
  pressed: { opacity: 0.85 },
});
