import { View } from 'react-native';
import { Dropdown, WarningBanner } from '../ds';
import { t } from '../i18n';
import type { CameraMode } from './keys';
import { Group, SectionCaption, ToggleRow } from './rows';
import type { SettingsModel } from './useSettings';

export function GameplaySection({ m }: { m: SettingsModel }) {
  const { values, env, set } = m;
  const masterOff = !values.hide_interface;
  return (
    <View>
      <SectionCaption title={t('SETTINGS_CHAT_SOCIAL')} />
      <Group>
        <ToggleRow
          label={t('SETTINGS_CLOSE_CHAT_AFTER_SENDING')}
          value={values.submit_message_closes_chat}
          onChange={v => set('submit_message_closes_chat', v)}
        />
      </Group>
      {env.inWorld && (
        <>
          <SectionCaption title={t('SETTINGS_CAMERA')} />
          <Group>
            <Dropdown
              variant="settings"
              label={t('SETTINGS_CAMERA_MODE')}
              value={values.camera_mode}
              disabled={env.cameraLocked}
              options={[
                { value: 'first', label: t('SETTINGS_CAMERA_FIRST_PERSON') },
                { value: 'third', label: t('SETTINGS_CAMERA_THIRD_PERSON') },
              ]}
              onChange={v => set('camera_mode', v as CameraMode)}
            />
            {env.cameraLocked && (
              <WarningBanner
                outlined
                message={t('SETTINGS_THE_CAMERA_MODE_IS_SET_BY')}
              />
            )}
          </Group>
        </>
      )}
      <SectionCaption title={t('SETTINGS_INTERFACE_CAPS')} />
      <Group>
        <ToggleRow
          label={t('SETTINGS_HIDE_PLAYER_NAMES')}
          value={values.hide_player_names}
          onChange={v => set('hide_player_names', v)}
        />
        <ToggleRow
          label={t('SETTINGS_HIDE_INTERFACE')}
          description={t('SETTINGS_HIDE_THE_ON_SCREEN_INTERFACE_TO')}
          value={values.hide_interface}
          onChange={v => set('hide_interface', v)}
        />
        <ToggleRow
          label={t('SETTINGS_HIDE_VIEW_PROFILE')}
          value={values.hide_view_profile && !masterOff}
          disabled={masterOff}
          onChange={v => set('hide_view_profile', v)}
        />
        <ToggleRow
          label={t('SETTINGS_HIDE_INTERACTION_PROMPTS')}
          value={values.hide_interaction_prompts && !masterOff}
          disabled={masterOff}
          onChange={v => set('hide_interaction_prompts', v)}
        />
      </Group>
    </View>
  );
}
