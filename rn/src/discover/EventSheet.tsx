import { Linking, Pressable, Share, StyleSheet, View } from 'react-native';
import { AppText, colors, Icon } from '../ds';
import { u } from '../ds/scale';
import { t } from '../i18n';
import { DiscoverEvent } from './api';
import { EventBadges } from './Cards';
import { calendarUrl, eventJumpLink, formatSchedule } from './eventTime';
import { SheetShell, SheetSnap } from './SheetShell';
import {
  ActionRow,
  Byline,
  Description,
  Divider,
  Hero,
  JumpButton,
  SquareButton,
  StatRow,
} from './SheetParts';
import { eventMeta } from './stats';

const COLLAPSED = 0.54;

export const addToCalendar = (
  event: DiscoverEvent,
  start = event.startsAt,
  end = event.endsAt,
) =>
  Linking.openURL(
    calendarUrl({
      title: event.title,
      description: event.description ?? '',
      location: event.place.location
        ? t('CALENDAR_EVENT_LOCATION', { x: event.place.x, y: event.place.y })
        : '',
      start,
      end: end ?? start,
    }),
  ).catch(() => undefined);

const shareEvent = (event: DiscoverEvent) =>
  Share.share({
    message: t('SHARE_EVENT_MESSAGE', {
      event: event.title,
      link: eventJumpLink(event.id),
    }),
  }).catch(() => undefined);

function RemindButton({ on, onPress }: { on: boolean; onPress: () => void }) {
  const label = t(on ? 'EVENTS_REMINDER_SET' : 'EVENTS_REMIND_ME');
  return (
    <Pressable
      accessibilityRole="button"
      accessibilityLabel={label}
      accessibilityState={{ selected: on }}
      onPress={onPress}
      style={[styles.remind, on && styles.remindOn]}
    >
      <Icon
        name={on ? 'check' : 'bell'}
        size={18}
        color={on ? colors.ruby : colors.snow}
        strokeWidth={2.4}
      />
      <AppText
        variant="button"
        style={[styles.remindText, on && { color: colors.ruby }]}
      >
        {label}
      </AppText>
    </Pressable>
  );
}

/** Event sheet: live events jump in, upcoming ones set a reminder. */
export function EventSheet({
  event,
  reminder,
  now,
  initial,
  onClose,
  onJumpIn,
  onReminder,
}: {
  event: DiscoverEvent;
  reminder: boolean;
  now: number;
  initial?: SheetSnap;
  onClose: () => void;
  onJumpIn: () => void;
  onReminder: (on: boolean) => void;
}) {
  const meta = eventMeta(event);
  const offset = new Date().getTimezoneOffset();
  return (
    <SheetShell
      fraction={COLLAPSED}
      initial={initial}
      onClose={onClose}
      footer={
        <ActionRow>
          <SquareButton
            icon="share"
            label={t('COMMON_SHARE')}
            onPress={() => shareEvent(event)}
          />
          <SquareButton
            icon="calendar"
            label={t('EVENT_ADD_TO_CALENDAR')}
            onPress={() => addToCalendar(event)}
          />
          {event.live ? (
            <JumpButton onPress={onJumpIn} />
          ) : (
            <RemindButton on={reminder} onPress={() => onReminder(!reminder)} />
          )}
        </ActionRow>
      }
    >
      {snap => (
        <View>
          <View style={styles.head}>
            <AppText
              variant="title"
              numberOfLines={snap === 'expanded' ? undefined : 2}
              style={styles.title}
            >
              {event.title}
            </AppText>
            <Byline creator={event.creator} />
          </View>
          <Hero image={event.image}>
            <EventBadges event={event} now={now} showFeatured />
          </Hero>
          <StatRow
            cells={meta}
            layout={meta.length === 3 ? 'thirds' : 'ends'}
          />
          {snap === 'expanded' ? (
            <View>
              <Divider />
              {event.description ? (
                <Description text={event.description} />
              ) : null}
              {event.schedule?.length ? <Divider /> : null}
              {event.schedule?.map(slot => (
                <View key={slot.start} style={styles.slot}>
                  <AppText variant="caption" style={styles.slotText}>
                    {formatSchedule(slot.start, slot.end, offset)}
                  </AppText>
                  <SquareButton
                    icon="calendar"
                    label={t('EVENT_ADD_TO_CALENDAR')}
                    onPress={() => addToCalendar(event, slot.start, slot.end)}
                  />
                </View>
              ))}
            </View>
          ) : null}
        </View>
      )}
    </SheetShell>
  );
}

const styles = StyleSheet.create({
  head: { marginTop: u(6.5) },
  title: { fontSize: u(20), lineHeight: u(26), fontFamily: 'Inter-Bold' },
  remind: {
    flex: 1,
    height: u(47),
    borderRadius: u(6),
    borderWidth: 1.5,
    borderColor: colors.snow,
    backgroundColor: '#4A1478',
    flexDirection: 'row',
    alignItems: 'center',
    justifyContent: 'center',
    gap: u(8),
  },
  remindOn: { borderColor: colors.ruby, backgroundColor: '#2A0A48' },
  remindText: { fontFamily: 'Inter-Bold', fontSize: u(15) },
  slot: {
    flexDirection: 'row',
    alignItems: 'center',
    justifyContent: 'space-between',
    marginBottom: u(10),
  },
  slotText: { fontSize: u(12), color: colors.pearl },
});
