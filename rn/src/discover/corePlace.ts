import { core } from '../core';
import type { Place as CorePlace } from '../core/native';
import type { Place } from './api';

const parseParcel = (position: string): [number, number] => {
  const [x, y] = position.split(',').map(Number);
  return [isFinite(x) ? x : 0, isFinite(y) ? y : 0];
};

/** Maps a place of the typed core to the card view-model; hosts come from the core's URLs. */
export function toPlace(place: CorePlace): Place {
  const { mainRealm, worldsContentServer } = core().urls();
  const inWorld = place.world && place.worldName !== '';
  const [x, y] = place.world ? [0, 0] : parseParcel(place.basePosition);
  return {
    id: place.id,
    title: place.title,
    creator: place.creator,
    image: place.image,
    users: place.users,
    likeRate: place.likeRate ?? null,
    location: place.world ? place.worldName : place.basePosition,
    realm: inWorld
      ? `${worldsContentServer}${place.worldName.toLowerCase()}`
      : mainRealm,
    x,
    y,
    world: place.world,
    description: place.description || undefined,
    categories: place.categories.length > 0 ? place.categories : undefined,
  };
}
