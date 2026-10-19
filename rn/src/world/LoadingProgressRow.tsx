import { StyleSheet, View } from 'react-native';
import { AppText, colors, fonts, LinearFill, u } from '../ds';
import { t } from '../i18n';
import type { LoadingProgress } from '../hud/types';
import { percentLabel, progressLabel, progressValue } from './loadingModel';

const FILL = [
  { offset: 0, color: colors.purple350 },
  { offset: 1, color: '#A14BF3' },
];

/** "N/M resources at X mb/s", the percent, and the purple bar under them. */
export function LoadingProgressRow({
  progress,
}: {
  progress: LoadingProgress;
}) {
  const value = progressValue(progress.percent);
  return (
    <View>
      <View style={styles.labels}>
        <AppText variant="body" style={styles.label}>
          {t('LOADING_PROGRESS', progressLabel(progress))}
        </AppText>
        <AppText variant="body" style={[styles.label, styles.percent]}>
          {percentLabel(progress.percent)}
        </AppText>
      </View>
      <View
        accessibilityRole="progressbar"
        accessibilityValue={{ min: 0, max: 100, now: Math.round(value * 100) }}
        style={styles.track}
      >
        <View style={[styles.fill, { width: `${value * 100}%` }]}>
          <LinearFill stops={FILL} angle={90} />
        </View>
      </View>
    </View>
  );
}

const styles = StyleSheet.create({
  labels: {
    flexDirection: 'row',
    justifyContent: 'space-between',
    marginBottom: u(2),
  },
  label: { fontSize: u(10), lineHeight: u(12), color: colors.snow },
  percent: { fontFamily: fonts.bold },
  track: {
    height: u(7),
    borderRadius: u(3.5),
    backgroundColor: 'rgba(0,0,0,0.85)',
    overflow: 'hidden',
  },
  fill: { height: '100%', borderRadius: u(3.5), overflow: 'hidden' },
});
