import { useEffect, useState } from 'react';
import { Linking } from 'react-native';
import { parsePreviewUrl, PreviewRoute } from './url';

/** The `dclrn://preview...` route the app was opened with (or re-opened with), else null. */
export function usePreviewRoute(): PreviewRoute | null {
  const [route, setRoute] = useState<PreviewRoute | null>(null);
  useEffect(() => {
    let alive = true;
    Linking.getInitialURL().then(url => {
      if (alive && url) {
        setRoute(parsePreviewUrl(url));
      }
    });
    const sub = Linking.addEventListener('url', ({ url }) => {
      const next = parsePreviewUrl(url);
      if (next) {
        setRoute(next);
      }
    });
    return () => {
      alive = false;
      sub.remove();
    };
  }, []);
  return route;
}
