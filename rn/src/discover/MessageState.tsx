import { StyleSheet, View } from 'react-native';
import { SvgXml } from 'react-native-svg';
import { AppText, PrimaryButton } from '../ds';
import { u } from '../ds/scale';
import { t } from '../i18n';

const STROKE = '#C9A6DB';
const FRAME = `<rect x="1.5" y="1.5" width="67" height="67" rx="8" stroke="${STROKE}" stroke-width="3"/>`;
const GLYPHS = {
  search: `<svg viewBox="0 0 70 70" fill="none">${FRAME}<circle cx="34" cy="30" r="12" stroke="${STROKE}" stroke-width="3"/><path d="M43 39 L50 46" stroke="${STROKE}" stroke-width="3" stroke-linecap="round"/><path d="M20 58 H25 M31 58 H50" stroke="${STROKE}" stroke-width="3" stroke-linecap="round"/></svg>`,
  offline: `<svg viewBox="0 0 70 70" fill="none">${FRAME}<path d="M14 27 Q35 10 56 27 M22 35 Q35 25 48 35 M29 42 Q35 38 41 42" stroke="${STROKE}" stroke-width="3" stroke-linecap="round"/><circle cx="35" cy="48" r="3" fill="${STROKE}"/><path d="M16 14 L54 44" stroke="${STROKE}" stroke-width="3" stroke-linecap="round"/><path d="M19 60 H25 M31 60 H51" stroke="${STROKE}" stroke-width="3" stroke-linecap="round"/></svg>`,
};

/** Centred framed glyph, title and body (no results, search error, connection lost) with an optional retry. */
export function MessageState({
  glyph,
  title,
  body,
  onRetry,
}: {
  glyph: keyof typeof GLYPHS;
  title: string;
  body: string;
  onRetry?: () => void;
}) {
  return (
    <View style={styles.root}>
      <SvgXml xml={GLYPHS[glyph]} width={u(70)} height={u(70)} />
      <AppText variant="heading" style={styles.title}>
        {title}
      </AppText>
      <AppText variant="body" style={styles.body}>
        {body}
      </AppText>
      {onRetry ? (
        <View style={styles.action}>
          <PrimaryButton
            label={t('COMMON_TRY_AGAIN')}
            tone="secondary"
            onPress={onRetry}
          />
        </View>
      ) : null}
    </View>
  );
}

const styles = StyleSheet.create({
  root: {
    flex: 1,
    alignItems: 'center',
    paddingTop: u(160),
    paddingHorizontal: u(40),
  },
  title: {
    marginTop: u(24),
    fontSize: u(26),
    lineHeight: u(29),
    maxWidth: u(260),
    textAlign: 'center',
    color: '#D9B8E8',
  },
  body: {
    marginTop: u(8),
    fontSize: u(17),
    lineHeight: u(23),
    maxWidth: u(240),
    textAlign: 'center',
    color: '#CDB0DA',
  },
  action: { marginTop: u(20), minWidth: u(160) },
});
