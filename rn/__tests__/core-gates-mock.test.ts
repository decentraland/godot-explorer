import { setCoreForTests } from '../src/core';
import { createMockCore } from '../src/core/mock';
import { createMockGates } from '../src/core/mock/gates';
import {
  AccessVerdict,
  CampaignFallback,
  DestinationKind,
  Environment,
  ResolveFailure,
  SupportStatus,
  UpdateDecision,
} from '../src/core/native';
import { placeJumpLink, eventJumpLink } from '../src/discover/eventTime';
import { mainRealm, worldRealm } from '../src/discover/api';
import { linkQueryFor, sceneLink } from '../src/menu/api';
import { marketplaceUrl } from '../src/backpack/platformPolicy';
import { entitiesUrl } from '../src/backpack/catalog';

afterEach(() => setCoreForTests(undefined));

describe('mock gates: urls', () => {
  const gates = createMockGates();

  it('reports the production environment and every url as a full url', () => {
    expect(gates.environment()).toBe(Environment.Org);
    const { devRealms, ...rest } = gates.urls();
    for (const [name, value] of Object.entries(rest)) {
      expect(value).toMatch(/^(https?|wss):\/\//);
      expect(name).toBeTruthy();
    }
    expect(devRealms).toHaveLength(3);
  });

  it('builds the links the core builds', () => {
    expect(gates.marketplaceBrowseUrl('wearables')).toBe(
      'https://decentraland.org/shop/items?category=wearable',
    );
    expect(gates.marketplaceBrowseUrl('emotes')).toMatch(/category=emote$/);
    expect(gates.marketplaceBrowseUrl('other')).toMatch(/\/shop\/items$/);
    expect(gates.marketplaceItemUrl('0xabc', '7')).toBe(
      'https://decentraland.org/shop/item/0xabc/7',
    );
    expect(gates.jumpPositionUrl('-9,-9')).toMatch(
      /\/jump\/\?position=-9%2C-9$/,
    );
    expect(gates.jumpRealmUrl('a b.dcl.eth')).toMatch(/realm=a%20b\.dcl\.eth$/);
    expect(gates.eventUrl('x&y')).toMatch(/\/events\/event\/\?id=x%26y$/);
  });

  it('links a scene by position in Genesis City and by name in a world', () => {
    const { mainRealm: main, worldsContentServer } = gates.urls();
    expect(gates.sceneLinkUrl(main, 3, -4)).toBe(
      'https://mobile.dclexplorer.com/open?position=3,-4',
    );
    expect(gates.sceneLinkUrl(`${main}/`, 3, -4)).toMatch(/position=3,-4$/);
    expect(gates.sceneLinkUrl(`${worldsContentServer}foo.dcl.eth`, 0, 0)).toBe(
      'https://mobile.dclexplorer.com/open?realm=foo.dcl.eth',
    );
  });
});

describe('shell url helpers use the core', () => {
  it('derive every link from core().urls() and the link functions', () => {
    setCoreForTests(createMockCore());
    expect(mainRealm()).toBe('https://realm-provider-ea.decentraland.org/main');
    expect(worldRealm('Foo.DCL.eth')).toBe(
      'https://worlds-content-server.decentraland.org/world/foo.dcl.eth',
    );
    expect(linkQueryFor(mainRealm(), 1, 2)).toBe('position=1,2');
    expect(linkQueryFor(worldRealm('a.dcl.eth'), 0, 0)).toBe('realm=a.dcl.eth');
    expect(sceneLink('position=1,2')).toBe(
      'https://mobile.dclexplorer.com/open?position=1,2',
    );
    expect(placeJumpLink({ world: true, location: 'a.eth' } as never)).toMatch(
      /\/jump\/\?realm=a\.eth$/,
    );
    expect(eventJumpLink('e1')).toMatch(/\/events\/event\/\?id=e1$/);
    expect(marketplaceUrl()).toMatch(/\/items\?category=wearable$/);
    expect(entitiesUrl()).toBe(
      'https://peer.decentraland.org/content/entities/active',
    );
  });

  it('follow an environment change of the core', () => {
    const base = createMockCore();
    setCoreForTests({
      ...base,
      urls: () => ({ ...base.urls(), mainRealm: 'https://x.test/main' }),
      sceneLinkUrl: (realm, x, y) =>
        base.sceneLinkUrl(
          realm === 'https://x.test/main' ? base.urls().mainRealm : realm,
          x,
          y,
        ),
    });
    expect(mainRealm()).toBe('https://x.test/main');
    expect(linkQueryFor(mainRealm(), 5, 6)).toBe('position=5,6');
  });
});

describe('mock gates: boot gates', () => {
  const gates = createMockGates();

  it('lets the app proceed on a supported device with no campaign', async () => {
    expect((await gates.versionGate()).decision).toBe(UpdateDecision.Proceed);
    const support = await gates.deviceSupport({
      totalRamMb: 4096,
      model: '',
      socModel: '',
      boardPlatform: '',
      hardware: '',
    });
    expect(support.status).toBe(SupportStatus.Supported);
    expect(support.modal).toBeUndefined();
    expect(await gates.pendingCampaign()).toBeUndefined();
    expect(await gates.campaignResolution()).toMatchObject({
      token: '',
      campaign: undefined,
      fallback: CampaignFallback.NoToken,
    });
    expect(gates.captureCampaignToken('t')).toBe(false);
  });

  it('stores the accepted terms version per instance', () => {
    expect(gates.termsVersion()).toBe(1);
    expect(gates.termsAcceptedVersion()).toBe(0);
    gates.acceptTerms(1);
    expect(gates.termsAcceptedVersion()).toBe(1);
    expect(createMockGates().termsAcceptedVersion()).toBe(0);
  });
});

describe('mock gates: resolveDestination', () => {
  const gates = createMockGates();

  it.each([
    ['10,20', { x: 10, y: 20 }],
    [' -9 , -9 ', { x: -9, y: -9 }],
    ['(3,4)', { x: 3, y: 4 }],
    ['decentraland://open?position=-9,-9', { x: -9, y: -9 }],
    ['https://decentraland.org/jump/?position=10%2C20', { x: 10, y: 20 }],
  ])('resolves %p to a Genesis parcel', async (input, parcel) => {
    const resolved = await gates.resolveDestination(input);
    expect(resolved).toMatchObject({
      ready: true,
      kind: DestinationKind.Genesis,
      access: AccessVerdict.Allowed,
      parcel,
    });
    expect(resolved.destination).toEqual({ realm: undefined, parcel });
  });

  it.each([
    ['foo.dcl.eth', undefined],
    ['Foo.dcl.eth 5,6', { x: 5, y: 6 }],
    ['https://decentraland.org/jump/?realm=foo.dcl.eth', undefined],
    [
      'https://mobile.dclexplorer.com/open?realm=foo.dcl.eth&position=1,2',
      { x: 1, y: 2 },
    ],
  ])('resolves %p to the world foo.dcl.eth', async (input, parcel) => {
    const resolved = await gates.resolveDestination(input);
    expect(resolved).toMatchObject({
      ready: true,
      kind: DestinationKind.World,
      worldName: 'foo.dcl.eth',
      parcel,
    });
    expect(resolved.destination.realm).toBe(
      'https://worlds-content-server.decentraland.org/world/foo.dcl.eth',
    );
  });

  it('tells a preview server from another realm', async () => {
    const preview = await gates.resolveDestination('http://localhost:8000 2,2');
    expect(preview).toMatchObject({
      kind: DestinationKind.Preview,
      parcel: { x: 2, y: 2 },
    });
    expect(preview.destination.realm).toBe('http://localhost:8000');
    expect(
      (await gates.resolveDestination('https://realm.example.com/main')).kind,
    ).toBe(DestinationKind.CustomRealm);
    expect(
      (
        await gates.resolveDestination(
          'decentraland://open?preview=http://10.0.0.5:8000',
        )
      ).kind,
    ).toBe(DestinationKind.Preview);
  });

  it.each([
    '',
    '   ',
    'hello',
    'a,b',
    'decentraland://open',
    'two words.eth x',
  ])('refuses %p as invalid input', async input => {
    expect(await gates.resolveDestination(input)).toMatchObject({
      ready: false,
      failure: ResolveFailure.InvalidInput,
    });
  });

  it('refuses a parcel outside Genesis City', async () => {
    expect(await gates.resolveDestination('151,0')).toMatchObject({
      ready: false,
      failure: ResolveFailure.ParcelOutOfBounds,
    });
  });

  it('resolves with a secret like without one', async () => {
    expect((await gates.resolveDestinationWithSecret('1,1', 's')).ready).toBe(
      true,
    );
  });
});
