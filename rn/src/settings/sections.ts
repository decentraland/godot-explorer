import type { I18nKey } from '../i18n';

export type SectionKey =
  | 'graphics'
  | 'audio'
  | 'gameplay'
  | 'language'
  | 'account'
  | 'help'
  | 'storage'
  | 'dev';

const SECTIONS: { key: SectionKey; label: I18nKey }[] = [
  { key: 'graphics', label: 'SETTINGS_GRAPHICS' },
  { key: 'audio', label: 'SETTINGS_AUDIO' },
  { key: 'gameplay', label: 'SETTINGS_GAMEPLAY' },
  { key: 'language', label: 'SETTINGS_LANGUAGE' },
  { key: 'account', label: 'SETTINGS_ACCOUNT' },
  { key: 'help', label: 'SETTINGS_HELP_SUPPORT' },
  { key: 'storage', label: 'SETTINGS_STORAGE' },
  { key: 'dev', label: 'SETTINGS_DEV_TOOLS' },
];

/** The seven sections, plus Dev Tools outside production. */
export const sectionsFor = (devBuild: boolean) =>
  SECTIONS.filter(s => s.key !== 'dev' || devBuild);

export const sectionLabel = (key: SectionKey): I18nKey =>
  SECTIONS.find(s => s.key === key)!.label;
