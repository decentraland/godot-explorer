import { Slider } from '../ds';
import { View } from 'react-native';
import { t } from '../i18n';
import { Group, SectionCaption } from './rows';
import type { SettingsModel } from './useSettings';

const SLIDERS = [
  ['audio_general_volume', 'SETTINGS_GENERAL'],
  ['audio_scene_volume', 'SETTINGS_SCENE'],
  ['audio_ui_volume', 'SETTINGS_UI_SFX'],
  ['audio_music_volume', 'SETTINGS_MUSIC_AMBIENT'],
  ['audio_avatar_and_emotes_volume', 'SETTINGS_AVATAR_EMOTE_SFX'],
] as const;

/** Applies live while dragging, persists on release. */
export function AudioSection({ m }: { m: SettingsModel }) {
  return (
    <View>
      <SectionCaption title={t('SETTINGS_VOLUME')} gap={18} />
      <Group gap={37}>
        {SLIDERS.map(([key, label]) => (
          <Slider
            variant="settings"
            key={key}
            label={t(label)}
            value={m.values[key]}
            onChange={v => m.set(key, v, false)}
            onChangeEnd={v => m.set(key, v)}
          />
        ))}
      </Group>
    </View>
  );
}
