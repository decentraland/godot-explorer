import { core } from '../core';
import { AccessVerdict, Control, ResolveFailure } from '../core/native';
import type { ResolvedDestination } from '../core/native';
import type { Place } from '../discover/api';
import { enterPlace } from '../world/enterPlace';
import { t } from '../i18n';
import { modalApi } from './bridge';

export type EntryVerdict = 'enter' | 'invalid' | 'private' | 'password';

/** Refusals that mean the place is not there; the rest (an empty parcel, a slow answer) are left to
 * the engine, which can still load it. */
const GONE: ResolveFailure[] = [
  ResolveFailure.InvalidInput,
  ResolveFailure.WorldNotFound,
  ResolveFailure.ParcelOutOfBounds,
  ResolveFailure.PreviewUnreachable,
];

export function entryVerdict(resolved: ResolvedDestination): EntryVerdict {
  if (resolved.access === AccessVerdict.NotAllowed) {
    return 'private';
  }
  if (resolved.access === AccessVerdict.NeedsPassword) {
    return 'password';
  }
  if (resolved.failure !== undefined && GONE.includes(resolved.failure)) {
    return 'invalid';
  }
  return 'enter';
}

/** What the core resolves a place from: the world name, else the parcel. */
export const destinationInput = (place: Place): string =>
  place.world ? place.location : `${place.x},${place.y}`;

/** The reason a wrong password was refused, as the dialog says it. */
export function passwordError(resolved: ResolvedDestination): string | null {
  if (resolved.ready) {
    return null;
  }
  if (resolved.failure === ResolveFailure.RateLimited) {
    return t('MODAL_WORLD_PASSWORD_TOO_MANY');
  }
  if (resolved.access === AccessVerdict.NeedsPassword) {
    return t('MODAL_WORLD_PASSWORD_WRONG');
  }
  return resolved.failure === undefined ? null : t('MODAL_WORLD_PASSWORD_BUSY');
}

/** Asks the core whether the player may enter, tells them when not, and asks for the password of
 * a protected world. Resolves true when the player may go. Fails open: if the core cannot say,
 * the engine decides. */
export async function mayEnter(place: Place): Promise<boolean> {
  const modals = modalApi();
  const input = destinationInput(place);
  let resolved: ResolvedDestination;
  try {
    resolved = await core().resolveDestination(input);
  } catch {
    return true;
  }
  const verdict = entryVerdict(resolved);
  if (verdict === 'enter' || !modals) {
    return true;
  }
  if (verdict === 'private') {
    modals.show({
      key: 'private-world',
      title: t('MODAL_PRIVATE_WORLD_TITLE', { world: place.title }),
      body: t('MODAL_PRIVATE_WORLD_BODY'),
      buttons: [{ label: t('MODAL_PRIVATE_WORLD_PRIMARY') }],
    });
    return false;
  }
  if (verdict === 'invalid') {
    modals.show({
      key: 'invalid-destination',
      title: t('MODAL_INVALID_DESTINATION_TITLE'),
      body: t('MODAL_INVALID_DESTINATION_BODY'),
      buttons: [{ label: t('MODAL_INVALID_DESTINATION_PRIMARY') }],
    });
    return false;
  }
  return new Promise<boolean>(resolve => {
    modals.askPassword({
      title: t('MODAL_WORLD_PASSWORD_TITLE'),
      subtitle: t('MODAL_WORLD_PASSWORD_SUBTITLE'),
      verify: async password => {
        const answer = await core().resolveDestinationWithSecret(
          input,
          password,
        );
        const error = passwordError(answer);
        if (error === null) {
          core().control(
            new Control.SetWorldSecret({
              realm: place.realm,
              secret: password,
            }),
          );
        }
        return error;
      },
      onCancel: () => resolve(false),
      onAccepted: () => resolve(true),
    });
  });
}

/** Enters `place` unless the core says no; true when the player went. */
export async function enterPlaceChecked(place: Place): Promise<boolean> {
  if (!(await mayEnter(place))) {
    return false;
  }
  enterPlace(place);
  return true;
}
