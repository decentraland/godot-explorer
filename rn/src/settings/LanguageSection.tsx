import { Dropdown } from '../ds';
import { t } from '../i18n';
import { LOCALES } from './keys';
import type { SettingsModel } from './useSettings';

const NAMES = {
  en: 'SETTINGS_LANGUAGE_NAME_EN',
  es: 'SETTINGS_LANGUAGE_NAME_ES',
  pt_BR: 'SETTINGS_LANGUAGE_NAME_PT_BR',
} as const;

/** `defaultOpen` renders the open-list state for the gallery. */
export function LanguageSection({
  m,
  defaultOpen = false,
}: {
  m: SettingsModel;
  defaultOpen?: boolean;
}) {
  return (
    <Dropdown
      variant="settings"
      label={t('SETTINGS_APP_LANGUAGE')}
      helper={t('SETTINGS_LANGUAGE_HELPER')}
      value={m.values.locale}
      options={LOCALES.map(l => ({ value: l, label: t(NAMES[l]) }))}
      defaultOpen={defaultOpen}
      onChange={v => m.set('locale', v)}
    />
  );
}
