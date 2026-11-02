import { useEffect, useState } from 'react';
import { core } from '../core';
import { onCoreEvents } from '../core/events';
import type { SeasonalSkin } from '../core/native';

/** The active season, `undefined` while the remote flags load or when none is set. */
export function useSeasonalSkin(): SeasonalSkin | undefined {
  const [skin, setSkin] = useState(() => core().seasonalSkin());
  useEffect(() => {
    setSkin(core().seasonalSkin());
    return onCoreEvents({ flagsLoaded: () => setSkin(core().seasonalSkin()) });
  }, []);
  return skin;
}
