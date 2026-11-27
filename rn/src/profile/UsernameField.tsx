import { InputUsername } from '@dcl/ui-native';
import { t } from '../i18n';
import { LIMITS, validateText } from './logic';
import type { UsernameTab } from './types';

export type UsernameVariant = 'tagged' | 'plain' | 'tabs';

/** Which Username UI applies: owners of minted names get the tabs; a claimed name without a names list has no tag. */
export function usernameVariant(
  names: readonly string[],
  hasClaimedName: boolean,
): UsernameVariant {
  if (names.length > 0) {
    return 'tabs';
  }
  return hasClaimedName ? 'plain' : 'tagged';
}

const ERRORS = {
  limit: 'INPUTS_CHARACTER_LIMIT_EXCEEDED',
  edgeSpaces: 'INPUTS_NO_EDGE_SPACES',
  url: 'INPUTS_ENTER_VALID_URL',
} as const;

export const usernameError = (value: string) => {
  const error = validateText(value, LIMITS.username, { noEdgeSpaces: true });
  return error ? t(ERRORS[error]) : undefined;
};

const TABS = { unique: 'Unique', nonUnique: 'Name' } as const;

/** Username input (`Input/Username`): plain, with the `#tag`, or UNIQUE NAME / NON-UNIQUE USERNAME tabs. */
export function UsernameField({
  variant,
  value,
  onChange,
  tag,
  names,
  tab,
  onTab,
  picked,
  onPick,
  disabled = false,
  defaultOpen = false,
  onOpenChange,
}: {
  variant: UsernameVariant;
  value: string;
  onChange: (value: string) => void;
  tag: string;
  names: readonly string[];
  tab: UsernameTab;
  onTab: (tab: UsernameTab) => void;
  picked: string;
  onPick: (name: string) => void;
  disabled?: boolean;
  defaultOpen?: boolean;
  onOpenChange?: (open: boolean) => void;
}) {
  return (
    <InputUsername
      label={t('PROFILE_USERNAME')}
      uniqueName={variant === 'tabs'}
      tab={TABS[tab]}
      onTabChange={next => onTab(next === 'Unique' ? 'unique' : 'nonUnique')}
      uniqueTabLabel={t('PROFILE_UNIQUE_NAME')}
      nameTabLabel={t('PROFILE_NON_UNIQUE_USERNAME')}
      names={names}
      picked={picked}
      onPick={onPick}
      value={value}
      onChangeText={onChange}
      suffix={variant === 'plain' ? undefined : `#${tag}`}
      placeholder={t('PROFILE_USERNAME_PLACEHOLDER')}
      maxLength={LIMITS.username}
      error={usernameError(value)}
      disabled={disabled}
      defaultOpen={defaultOpen}
      onOpenChange={onOpenChange}
    />
  );
}
