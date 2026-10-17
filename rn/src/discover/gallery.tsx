import '../hud/gallery';
import { ReactNode } from 'react';
import { NavProvider } from '../app/NavProvider';
import { BottomNav } from '../ds';
import { registerAlias, registerPreview } from '../gallery/registry';
import { GodotPlaceholder } from '../godot/GodotPlaceholder';
import { PlaceDetail } from './types';
import { ApiProvider } from './ApiContext';
import { CATEGORIES } from './catalog';
import { DiscoverBackground } from './DiscoverBackground';
import { DiscoverScreen } from './DiscoverScreen';
import { EventSheet } from './EventSheet';
import { FriendSheet } from './FriendSheet';
import { FtueScreen } from './FtueScreen';
import { ID } from './galleryIds';
import { createMockApi, MockOptions } from './mockApi';
import { EVENTS, FRIENDS, MOCK_NOW, PLACES, placeById } from './mockData';
import { PlaceSheet } from './PlaceSheet';
import type { SearchState } from './search';
import type { SheetSnap } from './SheetShell';
import { StoryScreen } from './StoryScreen';
import type { VoteState } from './types';
import type { Place } from './api';

const noop = () => {};
const place = (id: string): Place => placeById(id)!;
const PLAYED = [place('genesis'), place('wonder'), place('dejay')];
const nav = <BottomNav active="explore" bottomInset={0} onChange={noop} />;

function Frame({
  options,
  children,
}: {
  options?: MockOptions;
  children: ReactNode;
}) {
  return (
    <ApiProvider api={createMockApi(options)}>
      <NavProvider>{children}</NavProvider>
    </ApiProvider>
  );
}

function entry(title: string, render: () => ReactNode, settleMs = 1500) {
  return {
    title,
    orientation: 'portrait' as const,
    render: () => <>{render()}</>,
    settleMs,
  };
}

/** One preview registered under every manifest seq that shows the same state. */
function reg(
  seqs: number[],
  title: string,
  render: () => ReactNode,
  settleMs?: number,
) {
  seqs.forEach(seq => registerPreview(ID[seq], entry(title, render, settleMs)));
}

const home = (
  props: Partial<Parameters<typeof DiscoverScreen>[0]> = {},
  options?: MockOptions,
) => (
  <Frame options={options}>
    <DiscoverScreen
      footer={nav}
      onJumpIn={noop}
      played={PLAYED}
      now={MOCK_NOW}
      {...props}
    />
  </Frame>
);

reg([16, 44, 50, 61, 65, 72, 74, 80, 83, 87, 95, 98], 'Discover home', () =>
  home(),
);
reg([17, 78], 'Discover pushed', () =>
  home({ footer: undefined, onBack: noop }),
);
reg([77, 85], 'Discover pushed, loading', () =>
  home({ footer: undefined, onBack: noop }, { hang: true }),
);
reg([69], 'Discover loading', () => home({}, { hang: true }));
reg([71], 'Discover with broken images', () => home({}, { imageFail: true }));
reg([73], 'Discover rail loading more', () => home({}, { hasMore: true }));
reg(
  [75],
  'Discover scrolled, My Places loading',
  () => home({ startScrolled: true }, { hang: ['mine'] }),
  2500,
);
reg([86], 'Connection lost', () =>
  home(
    { footer: undefined, onBack: noop },
    { fail: ['banners', 'events', 'places'] },
  ),
);
reg([92], 'Discover, second banner', () => home({ bannerIndex: 1 }));
reg([99], 'Discover seasonal', () => home({ seasonal: true }));
reg([100], 'Seasonal background', () => <DiscoverBackground seasonal />);
reg([101], 'Seasonal overlay', () => (
  <DiscoverBackground seasonal tone="grey" />
));

const FTUE_PLACES = [place('genesis'), place('tower'), place('meta')];
reg([13], 'FTUE static', () => (
  <Frame>
    <FtueScreen
      username="UserName"
      places={[place('genesis')]}
      skipLabel="FTUE_SKIP_AND_EXPLORE"
      onJump={noop}
      onSkip={noop}
    />
  </Frame>
));
reg([14], 'FTUE carousel', () => (
  <Frame>
    <FtueScreen
      username="UserName"
      places={FTUE_PLACES}
      onJump={noop}
      onSkip={noop}
    />
  </Frame>
));
reg([15], 'FTUE carousel, second card', () => (
  <Frame>
    <FtueScreen
      username="UserName"
      places={FTUE_PLACES}
      initialIndex={1}
      onJump={noop}
      onSkip={noop}
    />
  </Frame>
));

const sheetOverHome = (sheet: ReactNode) => (
  <>
    {home()}
    {sheet}
  </>
);

const placeSheet = (
  id: string,
  over: {
    initial?: SheetSnap;
    favorite?: boolean;
    vote?: VoteState;
    downloadMb?: number | null;
    place?: Partial<Place>;
  } = {},
) => {
  const detail: PlaceDetail = {
    place: { ...place(id), ...over.place },
    favorite: over.favorite ?? false,
    vote: over.vote ?? null,
    downloadMb: over.downloadMb ?? null,
  };
  return sheetOverHome(
    <PlaceSheet
      detail={detail}
      initial={over.initial}
      onClose={noop}
      onJumpIn={noop}
      onFavorite={noop}
      onVote={noop}
    />,
  );
};
const expanded = { initial: 'expanded' as const };

reg([19], 'Place sheet, collapsed', () => placeSheet('meta'));
reg([20], 'Place sheet, expanded', () => placeSheet('meta', expanded));
reg([21], 'Place sheet, expanded favorite', () =>
  placeSheet('meta', { ...expanded, favorite: true }),
);
reg([23], 'Place sheet, expanded liked', () =>
  placeSheet('meta', { ...expanded, vote: 'like' }),
);
reg([24], 'Place sheet, expanded disliked', () =>
  placeSheet('meta', { ...expanded, vote: 'dislike' }),
);
reg([26], 'Place sheet, location and likes', () =>
  placeSheet('genesis-lite', { place: { location: '0,0' } }),
);
reg([27], 'World sheet, name and likes', () => placeSheet('world'));
reg([31], 'Place sheet, favorite', () =>
  placeSheet('meta', { favorite: true }),
);
reg([33], 'Place sheet, liked', () => placeSheet('meta', { vote: 'like' }));
reg([34], 'Place sheet, disliked', () =>
  placeSheet('meta', { vote: 'dislike' }),
);
reg([35], 'Place sheet, location only', () =>
  placeSheet('bare', { place: { location: '-0,-0' } }),
);
reg([36], 'World sheet, name only', () =>
  placeSheet('world', { place: { likeRate: null } }),
);
reg([82], 'Place sheet with download warning', () =>
  placeSheet('meta', { downloadMb: 120 }),
);
reg([88], 'Place sheet over banners', () => placeSheet('meta'));
reg([89], 'Place sheet, loading', () =>
  sheetOverHome(
    <PlaceSheet
      detail={null}
      onClose={noop}
      onJumpIn={noop}
      onFavorite={noop}
      onVote={noop}
    />,
  ),
);

const eventSheet = (
  id: string,
  over: {
    initial?: SheetSnap;
    reminder?: boolean;
    patch?: Partial<(typeof EVENTS)[0]>;
  } = {},
) => {
  const event = { ...EVENTS.find(e => e.id === id)!, ...over.patch };
  return sheetOverHome(
    <EventSheet
      event={event}
      reminder={over.reminder ?? false}
      now={MOCK_NOW}
      initial={over.initial}
      onClose={noop}
      onJumpIn={noop}
      onReminder={noop}
    />,
  );
};

reg([38, 81], 'Event sheet, live', () => eventSheet('chiri'));
reg([39, 42], 'Event sheet, expanded', () => eventSheet('chiri', expanded));
reg([41], 'Event sheet, in a world', () => eventSheet('world-event'));
reg([43], 'Event sheet, no location', () => eventSheet('nowhere'));
reg([45], 'Event sheet, reminder off', () => eventSheet('crypto'));
reg([46], 'Event sheet, reminder set', () =>
  eventSheet('crypto', { reminder: true }),
);

reg([84], 'Friend sheet', () =>
  sheetOverHome(
    <FriendSheet
      friend={FRIENDS[0]}
      downloadMb={120}
      onClose={noop}
      onJumpIn={noop}
    />,
  ),
);

const search = (startSearch: Partial<SearchState>, options?: MockOptions) =>
  home({ startSearch, footer: undefined }, options);

reg([51, 62, 66], 'Search, focused', () => search({}));
reg([52, 60], 'Search, typing', () =>
  search({ query: 'Ca', phase: 'suggest' }),
);
reg([53, 54], 'Search, suggestions', () =>
  search({ query: 'Casa', phase: 'suggest' }),
);
reg([55], 'Search, loading', () =>
  search({ query: 'Casa', phase: 'loading' }, { hang: true }),
);
reg([56], 'Search, results', () => search({ query: 'Casa', phase: 'loading' }));
reg([57], 'Search, error', () =>
  search({ query: 'Casa', phase: 'loading' }, { fail: ['search'] }),
);
reg([58], 'Search, no results', () =>
  search({ query: 'zzzz', phase: 'loading' }),
);
reg([59], 'Search, recents', () => search({ history: ['Casa'] }));
reg([63], 'Search, coordinates suggestion', () =>
  search({ query: '44,-48', phase: 'suggest' }),
);
reg([64], 'Search, coordinates results', () =>
  search({ query: '44,-48', phase: 'loading' }),
);
reg([67], 'Search, partial coordinates', () =>
  search({ query: '44,-', phase: 'suggest' }),
);
reg([68], 'Search, results in several rails', () =>
  search({ query: 'a', phase: 'loading' }),
);

reg([96], 'Category page', () =>
  home({ startCategory: CATEGORIES[2], footer: nav }),
);
reg([97], 'Category search', () =>
  home({
    startCategory: CATEGORIES[2],
    startCategorySearch: true,
    footer: undefined,
  }),
);

registerPreview(
  ID[93],
  entry('Story', () => (
    <Frame>
      <StoryScreen enabled place={PLACES[2]} onBack={noop} onJump={noop} />
    </Frame>
  )),
);
registerPreview(
  ID[94],
  entry('Story, short title', () => (
    <Frame>
      <StoryScreen
        enabled
        place={{ ...PLACES[4], title: 'Rage Parkour', creator: 'Creator Name' }}
        onBack={noop}
        onJump={noop}
      />
    </Frame>
  )),
);

const OS_SHARE = 'Native OS share sheet (Share.share) over this sheet';
const OS_CALENDAR = 'Native OS calendar/chooser intent over this sheet';
registerAlias(ID[22], ID[20], 'portrait', OS_SHARE);
registerAlias(ID[32], ID[19], 'portrait', OS_SHARE);
registerAlias(ID[47], ID[38], 'portrait', OS_SHARE);
[40, 48, 49].forEach(seq =>
  registerAlias(ID[seq], ID[39], 'portrait', OS_CALENDAR),
);
registerAlias(
  ID[76],
  'hud__hud__609-7021',
  'landscape',
  'In-world HUD; same screen as the HUD preview',
);
[25, 91].forEach(seq =>
  registerPreview(ID[seq], {
    title: 'Godot loading screen',
    orientation: 'portrait',
    notes:
      'Legacy Godot loading screen, captured rotated; the RN loading cover is loading__loading-1',
    render: () => <GodotPlaceholder label="Loading (Godot)" />,
  }),
);
