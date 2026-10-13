import { Pressable, StyleSheet, View } from 'react-native';
import { AppText, colors, fonts, Icon } from '../ds';
import { u } from '../ds/scale';
import { t } from '../i18n';
import { ClaimedSeal } from './ClaimedSeal';
import { shortAddress } from './logic';
import type { Profile } from './types';

const SIZES = {
  portrait: { name: 21, tag: 16, address: 15 },
  landscape: { name: 19, tag: 15, address: 13 },
} as const;

/** Name + tag and the short address, each with a copy icon (the name's sits at the far right in portrait). */
export function ProfileHeader({
  profile,
  size,
  onCopyName,
  onCopyAddress,
}: {
  profile: Profile;
  size: keyof typeof SIZES;
  onCopyName: () => void;
  onCopyAddress: () => void;
}) {
  const font = SIZES[size];
  const portrait = size === 'portrait';
  const copyName = (
    <CopyButton label={t('PROFILE_COPY_NAME')} onPress={onCopyName} />
  );
  return (
    <View>
      <View style={styles.nameRow}>
        <AppText
          variant="heading"
          numberOfLines={1}
          style={[styles.name, { fontSize: u(font.name) }]}
        >
          {profile.name}
        </AppText>
        {profile.hasClaimedName ? (
          <View style={styles.seal}>
            <ClaimedSeal size={font.tag + 2} />
          </View>
        ) : (
          <AppText
            variant="body"
            style={[styles.tag, { fontSize: u(font.tag) }]}
          >{`#${profile.tag}`}</AppText>
        )}
        {portrait && <View style={styles.push} />}
        {portrait && copyName}
      </View>
      <View style={styles.addressRow}>
        <AppText
          variant="body"
          style={[styles.address, { fontSize: u(font.address) }]}
        >
          {shortAddress(profile.address)}
        </AppText>
        <CopyButton
          label={t('PROFILE_COPY_ADDRESS')}
          onPress={onCopyAddress}
          size={portrait ? 22 : 20}
        />
      </View>
    </View>
  );
}

function CopyButton({
  label,
  onPress,
  size = 26,
}: {
  label: string;
  onPress: () => void;
  size?: number;
}) {
  return (
    <Pressable
      accessibilityRole="button"
      accessibilityLabel={label}
      hitSlop={10}
      onPress={onPress}
      style={styles.copy}
    >
      <Icon name="copy" size={size} color={colors.snow} strokeWidth={2} />
    </Pressable>
  );
}

const styles = StyleSheet.create({
  nameRow: { flexDirection: 'row', alignItems: 'baseline' },
  name: { flexShrink: 1 },
  tag: { marginLeft: u(8), color: colors.silver, fontFamily: fonts.regular },
  seal: { marginLeft: u(6), alignSelf: 'center' },
  push: { flex: 1 },
  addressRow: { flexDirection: 'row', alignItems: 'center', marginTop: u(4) },
  address: { color: colors.lavender, fontFamily: fonts.semibold },
  copy: { marginLeft: u(10), alignSelf: 'center' },
});
