import { useEffect, useState } from 'react';
import { core } from '../core';
import { onCoreEvents } from '../core/events';
import type { AvatarState } from '../core/native';

/** The equipped avatar, null until the core has one; follows `avatarChanged`. */
export function useAvatar(): AvatarState | null {
  const [avatar, setAvatar] = useState<AvatarState | null>(
    () => core().avatarState() ?? null,
  );
  useEffect(() => {
    setAvatar(core().avatarState() ?? null);
    return onCoreEvents({ avatarChanged: setAvatar });
  }, []);
  return avatar;
}
