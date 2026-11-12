import { Linking, StyleSheet, View } from 'react-native';
import { AppText, Button, colors, u } from '@dcl/ui-native';
import { FlowFrame } from './FlowFrame';
import { t } from '../i18n';

const STORE_URL = 'market://details?id=org.decentraland.godotexplorer';

/** Figma UPDATE available (1546:14269 with LATER, 1546:14186 forced): the version gate's screen. */
export function UpdateAvailable({
  required,
  onLater,
}: {
  required: boolean;
  onLater: () => void;
}) {
  return (
    <FlowFrame edgeToEdge>
      <View style={styles.text}>
        <AppText variant="heading" style={styles.title}>
          {t('FLOW_UPDATE_TITLE')}
        </AppText>
        <AppText variant="body" textBreakStrategy="simple" style={styles.copy}>
          {t('FLOW_UPDATE_COPY')}
        </AppText>
      </View>
      <View style={styles.buttons}>
        <Button
          big
          fullWidth
          label={t('FLOW_UPDATE')}
          onPress={() => Linking.openURL(STORE_URL).catch(() => {})}
        />
        {!required && (
          <Button
            big
            tone="secondary"
            fullWidth
            label={t('FLOW_UPDATE_LATER')}
            onPress={onLater}
          />
        )}
      </View>
    </FlowFrame>
  );
}

const styles = StyleSheet.create({
  text: { position: 'absolute', left: u(32), right: u(32), top: u(197) },
  title: { fontSize: u(36), lineHeight: u(43.5), textAlign: 'center' },
  copy: {
    marginTop: u(6),
    fontSize: u(20),
    lineHeight: u(24),
    textAlign: 'center',
    color: colors.snow,
  },
  buttons: {
    position: 'absolute',
    left: u(32),
    right: u(32),
    top: u(494),
    gap: u(19),
  },
});
