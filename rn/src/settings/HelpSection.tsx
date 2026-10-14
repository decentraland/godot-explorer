import { Linking, View } from 'react-native';
import { LinkRow, WarningBanner } from '../ds';
import { t } from '../i18n';
import { DISCORD_URL, helpUrl, legalLinks } from './links';
import { Group, SectionCaption } from './rows';
import type { SettingsModel } from './useSettings';

export function HelpSection({
  m,
  openUrl = url => Linking.openURL(url),
}: {
  m: SettingsModel;
  openUrl?: (url: string) => unknown;
}) {
  const { api, env } = m;
  return (
    <View>
      {env.deviceUnsupported && (
        <Group>
          <WarningBanner
            outlined
            message={t('MODAL_END_OF_DEVICE_SUPPORT_TITLE')}
          />
        </Group>
      )}
      <SectionCaption title={t('SETTINGS_SUPPORT_CAPS')} gap={1} />
      <Group gap={0} after={30}>
        <LinkRow
          height={42}
          inline
          label={t('SETTINGS_REPORT_BUG')}
          onPress={() => {
            api.openBugReport();
          }}
        />
        <LinkRow
          height={42}
          inline
          external
          label={t('SETTINGS_REPORT_CONTENT')}
          onPress={() => {
            api.openReportContent();
          }}
        />
        <LinkRow
          height={42}
          inline
          external
          label={t('SETTINGS_HELP')}
          onPress={() => {
            openUrl(helpUrl());
          }}
        />
      </Group>
      <SectionCaption title={t('SETTINGS_COMMUNITY_CAPS')} gap={1} />
      <Group gap={0} after={30}>
        <LinkRow
          height={42}
          inline
          external
          label={t('SETTINGS_DISCORD')}
          onPress={() => {
            openUrl(DISCORD_URL);
          }}
        />
      </Group>
      <SectionCaption title={t('SETTINGS_LEGAL_CAPS')} gap={1} />
      <Group gap={0} after={30}>
        {legalLinks().map(link => (
          <LinkRow
            height={42}
            key={link.url}
            inline
            external
            label={t(link.label)}
            onPress={() => {
              openUrl(link.url);
            }}
          />
        ))}
      </Group>
    </View>
  );
}
