import { applyRouted, onLinkTarget, targetOf } from '../src/app/links';
import { setCoreForTests } from '../src/core';
import { createMockCore } from '../src/core/mock';
import {
  ConfigParam,
  Effect,
  Navigation,
  AnalyticsEvent,
  StartKind,
} from '../src/core/native';

afterEach(() => setCoreForTests(undefined));

describe('deep link targets', () => {
  it('turns a teleport into the world to enter and a place link into its sheet', () => {
    const world = targetOf(
      new Navigation.Teleport({ realm: undefined, parcel: { x: 10, y: -4 } }),
    );
    expect(world).toMatchObject({ kind: 'world', place: { x: 10, y: -4 } });
    expect(targetOf(new Navigation.OpenPlace({ id: 'p1' }))).toEqual({
      kind: 'place',
      placeId: 'p1',
    });
    expect(targetOf(new Navigation.OpenEvent({ id: 'e1' }))).toEqual({
      kind: 'event',
      eventId: 'e1',
    });
    expect(targetOf(new Navigation.OpenDiscover())).toEqual({
      kind: 'discover',
    });
    expect(
      targetOf(new Navigation.JoinWorld({ realm: 'foo.dcl.eth' })),
    ).toMatchObject({ kind: 'world', place: { world: true } });
  });

  it('records the campaign token and the push open, and tells the shell where to go', () => {
    const core = createMockCore();
    const captured: string[] = [];
    core.captureCampaignToken = token => {
      captured.push(token);
      return true;
    };
    const tracked: string[] = [];
    core.track = name => {
      tracked.push(name);
      return Promise.resolve() as never;
    };
    setCoreForTests(core);

    const seen: string[] = [];
    const off = onLinkTarget(target => seen.push(target.kind));
    applyRouted({
      navigation: new Navigation.OpenDiscover(),
      effects: [
        new Effect.Config({
          param: new ConfigParam.CampaignToken({ token: 'summer' }),
        }),
        new Effect.Analytics({
          event: new AnalyticsEvent.PushOpened({
            campaignId: 'c',
            pushId: 'p',
            startKind: StartKind.Warm,
          }),
        }),
      ],
    });
    off();

    expect(captured).toEqual(['summer']);
    expect(tracked).toEqual(['Push Opened']);
    expect(seen).toEqual(['discover']);
  });
});
