import { ReactNode } from 'react';
import { Image, Pressable, StyleSheet, View } from 'react-native';
import { AppText, colors, Icon, IconName, u } from '../ds';
import { t } from '../i18n';
import type { Person } from './types';

const VERIFIED = '#E8408F';
const GREEN = '#34CE76';
const AVATAR = u(30);

export function Avatar({
  uri,
  size = AVATAR,
  backdrop = '#D4E85A',
  square = false,
}: {
  uri: string;
  size?: number;
  backdrop?: string;
  square?: boolean;
}) {
  return (
    <View
      style={[
        styles.clip,
        {
          width: size,
          height: size,
          borderRadius: square ? u(4) : size / 2,
          backgroundColor: backdrop,
        },
      ]}
    >
      {uri ? (
        <Image source={{ uri }} style={styles.fill} resizeMode="cover" />
      ) : (
        <>
          <View
            style={[
              styles.head,
              { width: size * 0.42, top: size * 0.16, left: size * 0.29 },
            ]}
          />
          <View
            style={[
              styles.body,
              { width: size * 0.8, top: size * 0.6, left: size * 0.1 },
            ]}
          />
        </>
      )}
    </View>
  );
}

/** One row: avatar, coloured name with the verified badge, subtitle, and an action slot on the right. */
export function PersonRow({
  person,
  subtitle,
  dim = false,
  right,
}: {
  person: Person;
  subtitle?: string;
  dim?: boolean;
  right?: ReactNode;
}) {
  return (
    <View style={styles.row}>
      <Avatar uri={person.avatar} />
      <View style={styles.texts}>
        <View style={styles.nameRow}>
          <AppText
            variant="button"
            numberOfLines={1}
            style={[styles.name, { color: person.color }]}
          >
            {person.name}
          </AppText>
          {person.claimed && (
            <Icon name="seal" size={11} color={VERIFIED} strokeWidth={2.4} />
          )}
        </View>
        {!!subtitle && (
          <AppText
            variant="body"
            numberOfLines={1}
            style={[styles.subtitle, dim && styles.dim]}
          >
            {subtitle}
          </AppText>
        )}
      </View>
      {right}
    </View>
  );
}

export type SquareKind = 'jump' | 'accept' | 'decline' | 'cancel';

const KIND: Record<
  SquareKind,
  { icon: IconName; pressed: string; label: string }
> = {
  jump: { icon: 'jump', pressed: colors.ruby, label: 'MENU_JUMP_TO_FRIEND' },
  accept: { icon: 'check', pressed: GREEN, label: 'MENU_ACCEPT' },
  decline: { icon: 'close', pressed: colors.ruby, label: 'MENU_DECLINE' },
  cancel: { icon: 'close', pressed: colors.ruby, label: 'MENU_CANCEL_REQUEST' },
};

/** 27dp action square; `pressed` forces the pressed look (gallery) on top of the live press state. */
export function SquareButton({
  kind,
  name,
  pressed: forced = false,
  disabled = false,
  onPress,
}: {
  kind: SquareKind;
  name?: string;
  pressed?: boolean;
  disabled?: boolean;
  onPress: () => void;
}) {
  const { icon, pressed: tone, label } = KIND[kind];
  return (
    <Pressable
      accessibilityRole="button"
      accessibilityLabel={t(label as 'MENU_ACCEPT', { name: name ?? '' })}
      accessibilityState={{ disabled }}
      disabled={disabled}
      onPress={onPress}
      style={({ pressed }) => {
        const on = pressed || forced;
        const outlined = kind !== 'jump';
        return [
          styles.square,
          kind === 'jump'
            ? styles.jump
            : [styles.outlined, { borderColor: on ? tone : colors.snow }],
          disabled && styles.disabled,
          outlined ? null : on && styles.pressedFill,
        ];
      }}
    >
      {({ pressed }) => (
        <Icon
          name={icon}
          size={kind === 'jump' ? 16 : 18}
          color={kind !== 'jump' && (pressed || forced) ? tone : colors.snow}
          strokeWidth={2.2}
        />
      )}
    </Pressable>
  );
}

export function AddFriendButton({ onPress }: { onPress: () => void }) {
  return (
    <Pressable
      accessibilityRole="button"
      accessibilityLabel={t('MENU_SOCIAL_ADD_FRIEND')}
      onPress={onPress}
      style={({ pressed }) => [styles.add, pressed && styles.pressedFill]}
    >
      <Icon name="personPlus" size={16} color={colors.snow} strokeWidth={2} />
      <AppText variant="button" style={styles.addText}>
        {t('MENU_SOCIAL_ADD_FRIEND')}
      </AppText>
    </Pressable>
  );
}

const styles = StyleSheet.create({
  head: {
    position: 'absolute',
    aspectRatio: 1,
    borderRadius: 999,
    backgroundColor: '#E9B596',
  },
  body: {
    position: 'absolute',
    height: '60%',
    borderRadius: 999,
    backgroundColor: '#3C2A5A',
  },
  clip: { overflow: 'hidden' },
  fill: { width: '100%', height: '100%' },
  row: {
    height: u(54),
    flexDirection: 'row',
    alignItems: 'center',
    gap: u(9),
  },
  texts: { flex: 1 },
  nameRow: { flexDirection: 'row', alignItems: 'center', gap: u(5) },
  name: { fontSize: u(15), flexShrink: 1 },
  subtitle: {
    fontSize: u(12),
    color: 'rgba(255,255,255,0.8)',
    marginTop: u(1),
  },
  dim: { opacity: 0.5 },
  square: {
    width: u(27),
    height: u(27),
    borderRadius: u(4),
    alignItems: 'center',
    justifyContent: 'center',
  },
  jump: { backgroundColor: colors.ruby },
  outlined: { borderWidth: 1 },
  disabled: { opacity: 0.4 },
  pressedFill: { opacity: 0.85 },
  add: {
    width: u(115),
    height: u(27),
    borderRadius: u(4),
    backgroundColor: colors.ruby,
    flexDirection: 'row',
    alignItems: 'center',
    justifyContent: 'center',
    gap: u(5),
  },
  addText: { fontFamily: 'Inter-Bold', fontSize: u(13) },
});
