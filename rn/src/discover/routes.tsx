import { useEffect, useMemo } from 'react';
import { useNav } from '../app/NavProvider';
import { registerScreen, ScreenProps } from '../app/registry';
import { EventSheet } from './EventSheet';
import { FriendSheet } from './FriendSheet';
import { PlaceSheet } from './PlaceSheet';
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
  useEffect(() => (failed ? pop() : undefined), [failed, pop]);
  return (
    <PlaceSheet
      detail={detail}
      onClose={pop}
      onJumpIn={() => detail && jump(detail.place)}
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
  useEffect(() => (failed ? pop() : undefined), [failed, pop]);
  return friend ? (
    <FriendSheet
      friend={friend}
      downloadMb={downloadMb}
      onClose={pop}
      onJumpIn={() => friend.place && jump(friend.place)}
    />
  ) : null;
}

registerScreen('placeDetail', PlaceSheetRoute);
registerScreen('eventDetail', EventSheetRoute);
registerScreen('friendDetail', FriendSheetRoute);
