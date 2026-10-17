import { core } from '../core';
import { Control } from '../core/native';
import type { Destination } from '../core/native';
import { mainRealm } from '../discover/api';
import type { Place } from '../discover/api';

export function destinationOf(place: Place): Destination {
  return {
    realm: place.realm === mainRealm() ? undefined : place.realm,
    parcel: { x: place.x, y: place.y },
  };
}

/** Teleports within the realm the player stands in; anywhere else it replaces the destination. */
export function enterPlace(place: Place): void {
  const state = core().state();
  if (state.tag === 'InWorld' && state.inner.realm === place.realm) {
    core().control(
      new Control.Teleport({ parcel: { x: place.x, y: place.y } }),
    );
    return;
  }
  core().enterWorld(destinationOf(place));
}
