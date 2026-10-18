import type { I18nKey } from '../i18n';
import type { DiscoverEvent, Place } from './api';

export type Page<T> = { items: T[]; hasMore: boolean };

export type PlaceRail = 'popular' | 'new' | 'favorites' | 'mine';

export type Friend = {
  address: string;
  name: string;
  /** `#1234` of an unclaimed name. */
  tag?: string;
  /** Pastel avatar background. */
  color: string;
  /** Where they are right now. */
  place: Place | null;
};

export type Banner = {
  id: string;
  image: string;
  title: string;
  place: Place;
  /** `arrow`: square CTA that opens the sheet. `button`: JUMP IN straight from the banner. */
  cta: 'arrow' | 'button';
};

export type Category = {
  id: string;
  label: I18nKey;
  title: I18nKey;
  emoji: string;
};

export type VoteState = 'like' | 'dislike' | null;

/** A place plus what the signed-in user did with it. */
export type PlaceDetail = {
  place: Place;
  favorite: boolean;
  vote: VoteState;
  /** MB the scene may download, null when unknown (no warning strip). */
  downloadMb: number | null;
};

export type EventDetail = { event: DiscoverEvent; reminder: boolean };

export type SearchResults = {
  featured: Place[];
  events: DiscoverEvent[];
  mostActive: Place[];
  favorites: Place[];
};

export type Suggestion = {
  kind: 'history' | 'keyword' | 'coordinates';
  text: string;
};

/** Everything Discover reads or writes. The default is the public-API implementation; with the engine, a bridge one adds the signed calls. */
export interface DiscoverApi {
  banners(): Promise<Banner[]>;
  /** The places the first-run carousel offers, curated server-side with the `ftue` tag. */
  ftuePlaces(): Promise<Place[]>;
  friends(): Promise<Friend[]>;
  events(): Promise<DiscoverEvent[]>;
  places(rail: PlaceRail, offset: number): Promise<Page<Place>>;
  categoryPlaces(
    category: string,
    offset: number,
    search?: string,
  ): Promise<Page<Place>>;
  search(query: string): Promise<SearchResults>;
  suggest(query: string): Promise<Suggestion[]>;
  placeDetail(id: string): Promise<PlaceDetail>;
  eventDetail(id: string): Promise<EventDetail>;
  friend(address: string): Promise<Friend | null>;
  setFavorite(placeId: string, on: boolean): Promise<void>;
  setVote(placeId: string, vote: VoteState): Promise<void>;
  setReminder(eventId: string, on: boolean): Promise<void>;
}
