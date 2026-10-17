import type { Orientation } from '../GodotBridge';

export type PreviewRoute = {
  /** Manifest id, or null for the list. */
  id: string | null;
  orientation?: Orientation;
  state?: string;
};

const PREFIX = 'dclrn://preview';

/** Parses `dclrn://preview[/<id>][?orientation=landscape|portrait&state=..]`; null for any other url. */
export function parsePreviewUrl(url: string): PreviewRoute | null {
  if (
    url !== PREFIX &&
    !url.startsWith(`${PREFIX}/`) &&
    !url.startsWith(`${PREFIX}?`)
  ) {
    return null;
  }
  const [path, query = ''] = url.slice(PREFIX.length).split('?');
  const id = path.replace(/^\/+|\/+$/g, '');
  const route: PreviewRoute = { id: id === '' ? null : decodeURIComponent(id) };
  query.split('&').forEach(pair => {
    const [key, value = ''] = pair.split('=');
    if (
      key === 'orientation' &&
      (value === 'landscape' || value === 'portrait')
    ) {
      route.orientation = value;
    } else if (key === 'state' && value !== '') {
      route.state = decodeURIComponent(value);
    }
  });
  return route;
}

export function previewIdFromUrl(url: string): string | null {
  return parsePreviewUrl(url)?.id ?? null;
}
