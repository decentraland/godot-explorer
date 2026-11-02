import { ComponentType, useEffect, useState } from 'react';
import { Linking } from 'react-native';

export interface PreviewRoute {
  id: string | null;
  orientation?: 'portrait' | 'landscape';
  state?: string;
}

interface Gallery {
  parsePreviewUrl(url: string): PreviewRoute | null;
  GalleryHost: ComponentType<PreviewRoute>;
}

// Required lazily so release bundles and jest do not depend on the dev-only harness.
function loadGallery(): Gallery | null {
  if (!__DEV__) {
    return null;
  }
  try {
    return require('../gallery') as Gallery;
  } catch {
    return null;
  }
}

export const gallery = loadGallery();

/** Dev only: the preview route from the launch URL or a later `url` event; `ready` once the launch URL is known. */
export function usePreviewRoute(): {
  route: PreviewRoute | null;
  ready: boolean;
} {
  const [route, setRoute] = useState<PreviewRoute | null>(null);
  const [ready, setReady] = useState(!gallery);

  useEffect(() => {
    if (!gallery) {
      return;
    }
    const apply = (url: string | null) =>
      setRoute(url ? gallery.parsePreviewUrl(url) : null);
    Linking.getInitialURL()
      .then(apply)
      .catch(() => apply(null))
      .finally(() => setReady(true));
    const sub = Linking.addEventListener('url', ({ url }) => apply(url));
    return () => sub.remove();
  }, []);

  return { route, ready };
}
