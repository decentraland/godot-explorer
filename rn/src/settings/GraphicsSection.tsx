import { StyleSheet, View } from 'react-native';
import { Dropdown, DropdownOption, LinearFill, u, WarningBanner } from '../ds';
import { t } from '../i18n';
import { GRAPHIC_PROFILES, SKYBOX_TIMES } from './keys';
import { Group, SectionCaption, ToggleRow } from './rows';
import type { SettingsModel } from './useSettings';

const PROFILE_LABELS = {
  1: 'SETTINGS_GRAPHIC_PROFILE_LOW',
  2: 'SETTINGS_GRAPHIC_PROFILE_MEDIUM',
  3: 'SETTINGS_GRAPHIC_PROFILE_HIGH',
} as const;

const SKYBOX_LABELS = {
  86400: 'SETTINGS_SKYBOX_MIDNIGHT',
  64800: 'SETTINGS_SKYBOX_AFTERNOON',
  43200: 'SETTINGS_SKYBOX_MIDDAY',
  21600: 'SETTINGS_SKYBOX_MORNING',
} as const;

const SKY_STOPS: Record<number, [string, string]> = {
  86400: ['#0B1030', '#2A2F5E'],
  64800: ['#F2A35E', '#7B5E9B'],
  43200: ['#6FB8F2', '#DDF1FF'],
  21600: ['#B9D5F0', '#F6E7D0'],
};

export const profileOptions = (): DropdownOption[] =>
  GRAPHIC_PROFILES.map(p => ({
    value: String(p),
    label: t(PROFILE_LABELS[p]),
  }));

export const skyboxOptions = (): DropdownOption[] =>
  SKYBOX_TIMES.map(s => ({ value: String(s), label: t(SKYBOX_LABELS[s]) }));

/** [left %, width %, height dp] of the distant buildings in the stand-in. */
const SKYLINE = [
  [8, 10, 22],
  [20, 7, 34],
  [30, 12, 18],
  [48, 8, 40],
  [62, 14, 20],
  [80, 9, 30],
] as const;

/** Stand-in for the live scene screenshot: a sky gradient for the selected time of day. */
function SkyPreview({ time }: { time: number }) {
  const [top, bottom] = SKY_STOPS[time] ?? SKY_STOPS[43200];
  return (
    <View style={styles.preview} testID="skybox-preview">
      <LinearFill
        stops={[
          { offset: 0, color: top },
          { offset: 1, color: bottom },
        ]}
      />
      <View style={styles.skyline}>
        {SKYLINE.map(([x, w, h], i) => (
          <View
            key={i}
            style={[
              styles.block,
              { left: `${x}%`, width: `${w}%`, height: u(h) },
            ]}
          />
        ))}
      </View>
      <View style={styles.ground} />
    </View>
  );
}

export function GraphicsSection({
  m,
  open,
}: {
  m: SettingsModel;
  open?: 'profile' | 'skybox' | 'list';
}) {
  const { values, env, set } = m;
  const dynamicSky = values.dynamic_skybox && !env.skyboxLocked;
  const customSky =
    dynamicSky || env.skyboxLocked ? null : String(values.skybox_time);
  const showPreview = !dynamicSky && !env.skyboxLocked;

  return (
    <View>
      <SectionCaption title={t('SETTINGS_PERFORMANCE')} />
      <Group>
        <ToggleRow
          label={t('SETTINGS_DYNAMIC_GRAPHICS')}
          description={t(
            'SETTINGS_ADJUSTS_GRAPHICS_AUTOMATICALLY_FOR_OPTIMAL_PERFORMANCE',
          )}
          value={values.dynamic_graphics_enabled}
          onChange={v => set('dynamic_graphics_enabled', v)}
        />
        <Dropdown
          variant="settings"
          label={t('SETTINGS_GRAPHIC_PROFILES')}
          helper={t('SETTINGS_SETS_A_GRAPHICS_QUALITY_PRESET_FOR')}
          value={String(values.graphic_profile)}
          options={profileOptions()}
          disabled={values.dynamic_graphics_enabled}
          defaultOpen={open === 'profile'}
          onChange={v => set('graphic_profile', Number(v))}
        />
      </Group>
      <SectionCaption title={t('SETTINGS_VISUAL')} />
      <Group>
        <ToggleRow
          label={t('SETTINGS_DYNAMIC_SKYBOX')}
          description={t('SETTINGS_THE_SKY_CHANGES_OVER_TIME_AS')}
          value={dynamicSky}
          disabled={env.skyboxLocked}
          onChange={v => set('dynamic_skybox', v)}
        />
        <Dropdown
          variant="settings"
          label={t('SETTINGS_CUSTOM_SKYBOX')}
          placeholder={t('SETTINGS_SELECT')}
          value={customSky}
          options={skyboxOptions()}
          disabled={dynamicSky || env.skyboxLocked}
          direction={showPreview ? 'up' : 'down'}
          defaultOpen={open === 'skybox'}
          onChange={v => set('skybox_time', Number(v))}
        />
        {env.skyboxLocked && (
          <WarningBanner
            outlined
            message={t('SETTINGS_THE_SKYBOX_IS_SET_BY_THE')}
          />
        )}
        {showPreview && <SkyPreview time={values.skybox_time} />}
      </Group>
    </View>
  );
}

const styles = StyleSheet.create({
  preview: { height: u(175), overflow: 'hidden', borderRadius: u(2) },
  skyline: {
    position: 'absolute',
    left: 0,
    right: 0,
    bottom: u(52),
    height: u(40),
  },
  block: {
    position: 'absolute',
    bottom: 0,
    backgroundColor: 'rgba(60,50,80,0.55)',
  },
  ground: {
    position: 'absolute',
    left: 0,
    right: 0,
    bottom: 0,
    height: u(52),
    backgroundColor: 'rgba(235,232,240,0.9)',
  },
});
