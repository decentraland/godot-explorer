import { core } from '../core';
import {
  AnalyticsEvent_Tags,
  ConfigParam_Tags,
  Effect_Tags,
  Navigation_Tags,
} from '../core/native';
import type { Effect, Navigation, Routed } from '../core/native';
import { mainRealm } from '../discover/api';
import type { Place } from '../discover/api';
import { trackEvent } from '../analytics';

/** Where a deep link wants the UI to go. */
export type LinkTarget =
  | { kind: 'world'; place: Place }
  | { kind: 'place'; placeId: string }
  | { kind: 'event'; eventId: string }
  | { kind: 'discover' };

type Handler = (target: LinkTarget) => void;
const handlers = new Set<Handler>();

/** Subscribes the shell (it owns the navigator) to link navigation; returns the unsubscribe. */
export function onLinkTarget(handler: Handler): () => void {
  handlers.add(handler);
  return () => {
    handlers.delete(handler);
  };
}

/** The world a link enters, as the minimal place the world route needs. */
function worldPlace(realm: string | undefined, x: number, y: number): Place {
  const url = realm ?? mainRealm();
  return {
    id: `link:${url}:${x},${y}`,
    title: realm ?? `${x},${y}`,
    creator: '',
    image: '',
    users: 0,
    likeRate: null,
    location: realm ?? `${x},${y}`,
    realm: url,
    x,
    y,
    world: realm !== undefined,
  };
}

export function targetOf(navigation: Navigation): LinkTarget {
  switch (navigation.tag) {
    case Navigation_Tags.Teleport: {
      const { realm, parcel } = navigation.inner;
      return { kind: 'world', place: worldPlace(realm, parcel.x, parcel.y) };
    }
    case Navigation_Tags.JoinWorld:
      return { kind: 'world', place: worldPlace(navigation.inner.realm, 0, 0) };
    case Navigation_Tags.OpenPlace:
      return { kind: 'place', placeId: navigation.inner.id };
    case Navigation_Tags.OpenEvent:
      return { kind: 'event', eventId: navigation.inner.id };
    default:
      return { kind: 'discover' };
  }
}

function applyEffect(effect: Effect): void {
  if (effect.tag === Effect_Tags.Config) {
    const { param } = effect.inner;
    if (param.tag === ConfigParam_Tags.CampaignToken) {
      core().captureCampaignToken(param.inner.token);
    }
    return;
  }
  if (effect.tag === Effect_Tags.Analytics) {
    const { event } = effect.inner;
    if (event.tag === AnalyticsEvent_Tags.PushOpened) {
      trackEvent('Push Opened', {
        campaign_id: event.inner.campaignId,
        push_id: event.inner.pushId,
        start_kind: String(event.inner.startKind),
      });
    }
  }
}

/** What the shell does with a routed deep link: records its effects and navigates. */
export function applyRouted(routed: Routed): void {
  routed.effects.forEach(applyEffect);
  if (routed.navigation) {
    const target = targetOf(routed.navigation);
    handlers.forEach(handler => handler(target));
  }
}
