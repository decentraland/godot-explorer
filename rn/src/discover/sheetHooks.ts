import { useCallback, useEffect, useState } from 'react';
import { useNav } from '../app/NavProvider';
import { useToast } from '../ds';
import { t } from '../i18n';
import { syncEventReminder } from '../notifications/push';
import { maybeAskForReview } from '../notifications/review';
import { enterPlaceChecked } from '../modals/destination';
import { Place } from './api';
import { useApi } from './ApiContext';
import { eventJumpLink } from './eventTime';
import { recordPlayed } from './history';
import type { EventDetail, Friend, PlaceDetail, VoteState } from './types';

/** Enters the world at `place` from a sheet route (the Discover tab gets the same via its onJumpIn prop). */
export function useJumpIn(): (place: Place) => void {
  const { push } = useNav();
  const toast = useToast();
  return useCallback(
    place => {
      enterPlaceChecked(place)
        .then(entered => {
          if (entered) {
            recordPlayed(place);
            push({ name: 'world', params: { place } });
          }
        })
        .catch(() => toast(t('DISCOVER_ACTION_FAILED')));
    },
    [push, toast],
  );
}

/** Loads one item; `failed` once the api rejects so the sheet can close instead of spinning. */
function useLoaded<T>(load: () => Promise<T>, deps: unknown[]) {
  const [value, setValue] = useState<T | null>(null);
  const [failed, setFailed] = useState(false);
  useEffect(() => {
    let cancelled = false;
    load()
      .then(result => !cancelled && setValue(result))
      .catch(() => !cancelled && setFailed(true));
    return () => {
      cancelled = true;
    };
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, deps);
  return { value, setValue, failed };
}

export function usePlaceDetail(placeId: string) {
  const api = useApi();
  const { value, setValue, failed } = useLoaded<PlaceDetail>(
    () => api.placeDetail(placeId),
    [api, placeId],
  );

  const toast = useToast();
  // Optimistic: apply, then put the previous state back if the service refuses.
  const update = useCallback(
    (patch: Partial<PlaceDetail>, save: () => Promise<void>) => {
      if (!value) {
        return;
      }
      setValue({ ...value, ...patch });
      save().catch(() => {
        setValue(value);
        toast(t('DISCOVER_ACTION_FAILED'));
      });
    },
    [value, setValue, toast],
  );
  const setFavorite = useCallback(
    (on: boolean) =>
      update({ favorite: on }, () =>
        api.setFavorite(placeId, on).then(() => {
          if (on) {
            maybeAskForReview('place_favorited').catch(() => {});
          }
        }),
      ),
    [api, placeId, update],
  );
  const setVote = useCallback(
    (vote: VoteState) =>
      update({ vote }, () =>
        api.setVote(placeId, vote).then(() => {
          if (vote === 'like') {
            maybeAskForReview('place_upvoted').catch(() => {});
          }
        }),
      ),
    [api, placeId, update],
  );
  return { detail: value, failed, setFavorite, setVote };
}

export function useEventDetail(eventId: string) {
  const api = useApi();
  const { value, setValue, failed } = useLoaded<EventDetail>(
    () => api.eventDetail(eventId),
    [api, eventId],
  );
  const toast = useToast();
  const setReminder = useCallback(
    (on: boolean) => {
      if (!value) {
        return;
      }
      setValue({ ...value, reminder: on });
      api
        .setReminder(eventId, on)
        .then(() =>
          syncEventReminder(value.event, on, eventJumpLink(eventId)).catch(
            () => {},
          ),
        )
        .catch(() => {
          setValue(value);
          toast(t('DISCOVER_ACTION_FAILED'));
        });
    },
    [api, eventId, value, setValue, toast],
  );
  return { detail: value, failed, setReminder };
}

export function useFriendDetail(address: string) {
  const api = useApi();
  const friend = useLoaded<Friend | null>(
    () => api.friend(address),
    [api, address],
  );
  const placeId = friend.value?.place?.id;
  const place = useLoaded<PlaceDetail | null>(
    () => (placeId ? api.placeDetail(placeId) : Promise.resolve(null)),
    [api, placeId],
  );
  return {
    friend: friend.value,
    downloadMb: place.value?.downloadMb ?? null,
    failed: friend.failed,
  };
}
