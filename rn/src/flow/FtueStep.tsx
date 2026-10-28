import { useEffect, useRef, useState } from 'react';
import { core } from '../core';
import type { Place } from '../discover/api';
import { useApi } from '../discover/ApiContext';
import { landedFtuePlaces } from '../discover/ftue';
import { FtueScreen } from '../discover/FtueScreen';

/** The FTUE, or straight into its place when the list that landed holds exactly one (#3022): a
 * one-card carousel only asks to confirm the obvious. A list still in flight shows the carousel. */
export function FtueStep({
  username,
  onJumpIn,
  onFinish,
}: {
  username: string;
  onJumpIn?: (place: Place) => Promise<boolean>;
  onFinish: () => void;
}) {
  const api = useApi();
  const [single] = useState(() => {
    const places = landedFtuePlaces(api);
    return onJumpIn && places?.length === 1 ? places[0] : undefined;
  });
  const jumped = useRef(false);
  useEffect(() => {
    if (!single || !onJumpIn || jumped.current) {
      return;
    }
    jumped.current = true;
    onJumpIn(single)
      .then(booted => core().trackFtueBypass(single.id, booted))
      .catch(() => {});
  }, [single, onJumpIn]);

  if (single) {
    return null;
  }
  return (
    <FtueScreen
      username={username}
      skipLabel="FTUE_SKIP_AND_EXPLORE"
      onJump={place => (onJumpIn ? onJumpIn(place) : onFinish())}
      onSkip={onFinish}
    />
  );
}
