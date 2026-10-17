import { useState } from 'react';
import { Pressable, StyleSheet, View } from 'react-native';
import {
  AppText,
  Dropdown,
  OutlinedButton,
  colors,
  Slider,
  TextField,
  u,
} from '../ds';
import { t } from '../i18n';
import { DEV_REALMS } from './links';
import { Group, SectionCaption, ToggleRow } from './rows';
import type { SettingsModel } from './useSettings';

/** Only mounted when `env.devBuild`; text-heavy and not in production. */
export function DevToolsSection({ m }: { m: SettingsModel }) {
  const { values, set, api } = m;
  const [url, setUrl] = useState(values.websocket_url);
  return (
    <View>
      <SectionCaption title={t('SETTINGS_DEV_TOOLS_CAPS')} />
      <Group>
        <ToggleRow
          label={t('SETTINGS_SCENE_LOGS')}
          value={values.scene_logs}
          onChange={v => set('scene_logs', v)}
        />
        <ToggleRow
          label={t('SETTINGS_RAYCAST_DEBUGGER')}
          value={values.raycast_debugger}
          onChange={v => set('raycast_debugger', v)}
        />
        <ToggleRow
          label={t('SETTINGS_SCENE_PROCESSING')}
          value={values.scene_processing}
          onChange={v => set('scene_processing', v)}
        />
        <Dropdown
          label={t('SETTINGS_REALM')}
          placeholder={t('SETTINGS_SELECT_ONE')}
          value={values.realm || null}
          options={DEV_REALMS.map(r => ({ value: r, label: r }))}
          onChange={v => set('realm', v)}
        />
        <View style={styles.urlRow}>
          <View style={styles.urlField}>
            <TextField
              label={t('SETTINGS_WEBSOCKET_URL')}
              value={url}
              autoCapitalize="none"
              keyboardType="url"
              onChangeText={setUrl}
            />
          </View>
          <Pressable
            accessibilityRole="button"
            accessibilityLabel={t('SETTINGS_CONNECT')}
            onPress={() => {
              set('websocket_url', url);
              api.devAction('connect_websocket');
            }}
            style={styles.connect}
          >
            <AppText variant="button" style={styles.connectText}>
              {t('SETTINGS_CONNECT')}
            </AppText>
          </Pressable>
        </View>
        <Slider
          label={t('SETTINGS_PROCESS_TICK_QUOTA')}
          value={values.process_tick_quota}
          onChange={v => set('process_tick_quota', v, false)}
          onChangeEnd={v => set('process_tick_quota', v)}
        />
        <OutlinedButton
          box
          label={t('SETTINGS_TEST_NOTIFICATIONS')}
          onPress={() => {
            api.devAction('test_notification');
          }}
        />
      </Group>
    </View>
  );
}

const styles = StyleSheet.create({
  urlRow: { flexDirection: 'row', alignItems: 'flex-end', gap: u(8) },
  urlField: { flex: 1 },
  connect: {
    width: u(85),
    height: u(34),
    borderRadius: u(5),
    alignItems: 'center',
    justifyContent: 'center',
    backgroundColor: colors.ruby,
    marginBottom: 0,
  },
  connectText: { fontSize: u(13) },
});
