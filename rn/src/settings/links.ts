import type { I18nKey } from '../i18n';

export type HelpLink = {
  label: I18nKey;
  url: string;
};

export const HELP_URL = 'https://decentraland.org/help';
export const DISCORD_URL =
  'https://discord.com/channels/417796904760639509/1446513533893218465';
export const PRIVACY_URL = 'https://decentraland.org/privacy/';
export const CONTENT_URL = 'https://decentraland.org/content/';
export const TERMS_URL = 'https://decentraland.org/terms/';

export const LEGAL_LINKS: HelpLink[] = [
  { label: 'SETTINGS_PRIVACY_POLICY', url: PRIVACY_URL },
  { label: 'SETTINGS_CONTENT_POLICY', url: CONTENT_URL },
  { label: 'SETTINGS_TERMS_OF_USE', url: TERMS_URL },
];

/** Realms offered by the dev-only picker. */
export const DEV_REALMS = [
  'main',
  'https://sdk-team-cdn.decentraland.org/ipfs/goerli-plaza-main-latest',
  'https://worlds-content-server.decentraland.org/world/olavra.dcl.eth',
];
