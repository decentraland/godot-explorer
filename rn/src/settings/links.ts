import { core } from '../core';
import type { I18nKey } from '../i18n';

export type HelpLink = {
  label: I18nKey;
  url: string;
};

// Not an environment host, so the core's `Urls` does not carry it.
export const DISCORD_URL =
  'https://discord.com/channels/417796904760639509/1446513533893218465';

export const helpUrl = (): string => core().urls().help;

export const legalLinks = (): HelpLink[] => {
  const urls = core().urls();
  return [
    { label: 'SETTINGS_PRIVACY_POLICY', url: urls.privacyPolicy },
    { label: 'SETTINGS_CONTENT_POLICY', url: urls.contentPolicy },
    { label: 'SETTINGS_TERMS_OF_USE', url: urls.termsOfService },
  ];
};

/** Realms offered by the dev-only picker. */
export const devRealms = (): string[] => core().urls().devRealms;
