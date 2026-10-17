import { StyleSheet, View } from 'react-native';
import { AppText, colors, Dropdown, Tabs, TextField } from '../ds';
import { u } from '../ds/scale';
import { t } from '../i18n';
import { ClaimedSeal } from './ClaimedSeal';
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

/** Username input: plain, with the `#tag` and counter, or UNIQUE NAME / NON-UNIQUE USERNAME tabs. */
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
  const title = t('PROFILE_USERNAME');
  const tagged = variant !== 'plain';
  const showInput = variant !== 'tabs' || tab === 'nonUnique';
  return (
    <View>
      {variant === 'tabs' && (
        <>
          <AppText variant="body" style={styles.title}>
            {title}
          </AppText>
          <View style={styles.tabs}>
            <Tabs
              items={[
                { key: 'unique', label: t('PROFILE_UNIQUE_NAME') },
                { key: 'nonUnique', label: t('PROFILE_NON_UNIQUE_USERNAME') },
              ]}
              active={tab}
              onChange={onTab}
              gap={14}
              labelSize={10}
            />
          </View>
        </>
      )}
      {showInput ? (
        <TextField
          label={variant === 'tabs' ? '' : title}
          value={value}
          onChangeText={onChange}
          onClear={() => onChange('')}
          placeholder={t('PROFILE_USERNAME_PLACEHOLDER')}
          maxLength={LIMITS.username}
          showCounter={tagged}
          counterBelow
          error={usernameError(value)}
          disabled={disabled}
          autoCapitalize="none"
          suffix={
            tagged ? (
              <AppText variant="body" style={styles.tag}>{`#${tag}`}</AppText>
            ) : undefined
          }
        />
      ) : (
        <Dropdown
          label=""
          value={picked}
          options={names.map(name => ({
            value: name,
            label: name,
            iconNode: <ClaimedSeal />,
          }))}
          onChange={onPick}
          disabled={disabled}
          defaultOpen={defaultOpen}
          onOpenChange={onOpenChange}
        />
      )}
    </View>
  );
}

const styles = StyleSheet.create({
  title: { fontSize: u(15), fontFamily: 'Inter-Medium', color: colors.snow },
  tabs: { alignSelf: 'flex-start', marginTop: u(6), marginBottom: u(8) },
  tag: { marginHorizontal: u(8), color: colors.silver, fontSize: u(13) },
});
