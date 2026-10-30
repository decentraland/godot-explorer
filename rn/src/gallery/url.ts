import type { Orientation } from '../platform/orientation';

export type PreviewRoute = {
  /** Manifest id, or null for the list. */
  id: string | null;
  orientation?: Orientation;
  state?: string;
};

const PREFIX = 'dclrn://preview';

/**
 * Parses `dclrn://preview[/<id>][?orientation=landscape|portrait&state=..]`, or on web
 * `http(s)://host/?preview=<id>[&orientation=..&state=..]`; null for any other url.
 */
export function parsePreviewUrl(url: string): PreviewRoute | null {
  let id = '';
  let query: string;
  if (
    url === PREFIX ||
    url.startsWith(`${PREFIX}/`) ||
    url.startsWith(`${PREFIX}?`)
  ) {
    const [path, rest = ''] = url.slice(PREFIX.length).split('?');
    id = path.replace(/^\/+|\/+$/g, '');
    query = rest;
  } else if (/^https?:\/\/[^?#]*\?(.*&)?preview=/.test(url)) {
    query = url.split('#')[0].split('?')[1];
  } else {
    return null;
  }
  const route: PreviewRoute = { id: null };
  query.split('&').forEach(pair => {
    const [key, value = ''] = pair.split('=');
    if (key === 'preview') {
      id = value;
    } else if (
      key === 'orientation' &&
      (value === 'landscape' || value === 'portrait')
    ) {
      route.orientation = value;
    } else if (key === 'state' && value !== '') {
      route.state = decodeURIComponent(value);
    }
  });
  route.id = id === '' ? null : decodeURIComponent(id);
  return route;
}

export function previewIdFromUrl(url: string): string | null {
  return parsePreviewUrl(url)?.id ?? null;
}
