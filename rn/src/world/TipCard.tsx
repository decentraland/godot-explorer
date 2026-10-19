import { StyleSheet, View } from 'react-native';
import { AppText, colors, fonts, Icon, u } from '../ds';
import { t } from '../i18n';
import type { Tip } from './loadingModel';

/** Bordered purple card with the tip's icon and "Tip: text". */
export function TipCard({ tip }: { tip: Tip }) {
  return (
    <View style={styles.card}>
      <Icon name={tip.icon} size={16} color={colors.lavender} strokeWidth={2} />
      <AppText variant="body" style={styles.text}>
        <AppText variant="body" style={[styles.text, styles.bold]}>
          {t('LOADING_TIP_LABEL')}{' '}
        </AppText>
        {t(tip.key)}
      </AppText>
    </View>
  );
}

const styles = StyleSheet.create({
  card: {
    width: u(300),
    flexDirection: 'row',
    alignItems: 'center',
    gap: u(8.5),
    paddingHorizontal: u(12.5),
    paddingVertical: u(10),
    borderRadius: u(2),
    borderWidth: 1,
    borderColor: colors.lavender,
    backgroundColor: 'rgba(70,15,111,0.7)',
  },
  text: { flex: 1, fontSize: u(10), lineHeight: u(12), color: colors.snow },
  bold: { fontFamily: fonts.semibold },
});
