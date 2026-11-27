import { useEffect, useMemo } from 'react';
import { useNav } from '../app/NavProvider';
import { registerScreen, ScreenProps } from '../app/registry';
import type { Place } from './api';
import { EventSheet } from './EventSheet';
import { FriendSheet } from './FriendSheet';
import {
  DEEPLINK,
  trackPlaceDetailClick,
  usePlaceDetailsView,
} from './metrics';
import { PlaceSheet, sharePlace } from './PlaceSheet';
import {
  useEventDetail,
  useFriendDetail,
  useJumpIn,
  usePlaceDetail,
} from './sheetHooks';

function PlaceSheetRoute({ params }: ScreenProps<'placeDetail'>) {
  const { pop } = useNav();
  const jump = useJumpIn();
  const { detail, failed, setFavorite, setVote } = usePlaceDetail(
    params.placeId,
  );
  const origin = params.origin ?? DEEPLINK;
  usePlaceDetailsView(detail !== null, detail?.place ?? null, origin);
  useEffect(() => (failed ? pop() : undefined), [failed, pop]);
  const press = (
    button: 'JUMP_IN' | 'SHARE',
    run: (place: Place) => unknown,
  ) => {
    if (detail) {
      trackPlaceDetailClick(button, detail.place.id, origin);
      run(detail.place);
    }
  };
  return (
    <PlaceSheet
      detail={detail}
      onClose={pop}
      onJumpIn={() => press('JUMP_IN', jump)}
      onShare={() => press('SHARE', sharePlace)}
      onFavorite={setFavorite}
      onVote={setVote}
    />
  );
}

function EventSheetRoute({ params }: ScreenProps<'eventDetail'>) {
  const { pop } = useNav();
  const jump = useJumpIn();
  const { detail, failed, setReminder } = useEventDetail(params.eventId);
  const now = useMemo(() => Date.now(), []);
  useEffect(() => (failed ? pop() : undefined), [failed, pop]);
  return detail ? (
    <EventSheet
      event={detail.event}
      reminder={detail.reminder}
      now={now}
      onClose={pop}
      onJumpIn={() => jump(detail.event.place)}
      onReminder={setReminder}
    />
  ) : null;
}

function FriendSheetRoute({ params }: ScreenProps<'friendDetail'>) {
  const { pop } = useNav();
  const jump = useJumpIn();
  const { friend, downloadMb, failed } = useFriendDetail(params.address);
  const origin = params.origin ?? DEEPLINK;
  usePlaceDetailsView(friend !== null, friend?.place ?? null, origin);
  useEffect(() => (failed ? pop() : undefined), [failed, pop]);
  return friend ? (
    <FriendSheet
      friend={friend}
      downloadMb={downloadMb}
      onClose={pop}
      onJumpIn={() => {
        if (friend.place) {
          trackPlaceDetailClick('JUMP_IN', friend.place.id, origin);
          jump(friend.place);
        }
      }}
    />
  ) : null;
}

registerScreen('placeDetail', PlaceSheetRoute);
registerScreen('eventDetail', EventSheetRoute);
registerScreen('friendDetail', FriendSheetRoute);
