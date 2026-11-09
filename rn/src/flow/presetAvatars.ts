import { core } from '../core';
import { Control } from '../core/native';

const PRESET_COUNT = 12;

export type PresetAvatar = {
  id: string;
  thumbnail: string;
  bodyShape: string;
  wearables: string[];
  skin: string;
  hair: string;
  eyes: string;
};

type PeerAvatar = {
  bodyShape?: string;
  wearables?: string[];
  skin?: { color?: { r: number; g: number; b: number } };
  hair?: { color?: { r: number; g: number; b: number } };
  eyes?: { color?: { r: number; g: number; b: number } };
  snapshots?: { body?: string };
};

const hex = (c?: { r: number; g: number; b: number }): string =>
  c
    ? `#${[c.r, c.g, c.b]
        .map(v =>
          Math.round(v * 255)
            .toString(16)
            .padStart(2, '0'),
        )
        .join('')}`
    : '#000000';

/** The stock profiles ("default1".."default12") the lobby's preset carousel shows, same as
 * `godot/src/ui/components/atoms/controls/preset_avatar_card/preset_avatar_card.gd`. Never
 * throws: a single bad/unreachable preset resolves to null instead of failing the whole batch. */
async function fetchPreset(id: string): Promise<PresetAvatar | null> {
  try {
    const base = core().urls().peerLambdas;
    const response = await fetch(`${base}profiles/${id}`);
    if (!response.ok) {
      return null;
    }
    const json = await response.json();
    const avatar: PeerAvatar | undefined = json?.avatars?.[0]?.avatar;
    const body = avatar?.snapshots?.body;
    if (!avatar || !body) {
      return null;
    }
    const thumbnail = body.startsWith('http')
      ? body
      : `${json.baseUrl ?? core().urls().peerContent}contents/${body}`;
    return {
      id,
      thumbnail,
      bodyShape: avatar.bodyShape ?? '',
      wearables: avatar.wearables ?? [],
      skin: hex(avatar.skin?.color),
      hair: hex(avatar.hair?.color),
      eyes: hex(avatar.eyes?.color),
    };
  } catch {
    return null;
  }
}

/** Fetches the 12 stock presets in parallel; drops any that fail instead of failing the whole set. */
export async function fetchPresetAvatars(): Promise<PresetAvatar[]> {
  const ids = Array.from({ length: PRESET_COUNT }, (_, i) => `default${i + 1}`);
  const results = await Promise.all(ids.map(fetchPreset));
  return results.filter((p): p is PresetAvatar => p !== null);
}

/** The preset's body shape, wearables and colors, as one edit: the engine draws the result only. */
export function applyPresetAvatar(preset: PresetAvatar): void {
  core().control(
    new Control.SetLook({
      bodyShape: preset.bodyShape,
      wearables: preset.wearables,
      skin: preset.skin,
      hair: preset.hair,
      eyes: preset.eyes,
    }),
  );
}
