import { useEffect, useState } from 'react';
import { BackHandler, StyleSheet, View } from 'react-native';
import { Button, colors, Dialog, u } from '@dcl/ui-native';
import type { BackpackApi } from '../backpack/api';
import { BackpackScreen } from '../backpack/BackpackScreen';
import { t } from '../i18n';

export type CustomizePrompt = 'finish' | 'discard';

/** The entry flow's backpack, opened on Face: DONE asks to finish the look, back asks to discard. */
export function AvatarCustomize({
  onDone,
  onDiscard,
  api,
  initialPrompt = null,
}: {
  onDone: () => void;
  onDiscard: () => void;
  api?: BackpackApi;
  initialPrompt?: CustomizePrompt | null;
}) {
  const [prompt, setPrompt] = useState<CustomizePrompt | null>(initialPrompt);
  const close = () => setPrompt(null);
  useEffect(() => {
    const subscription = BackHandler.addEventListener(
      'hardwareBackPress',
      () => {
        setPrompt('discard');
        return true;
      },
    );
    return () => subscription.remove();
  }, []);
  return (
    <>
      <BackpackScreen
        api={api}
        initial={{ tab: 'face' }}
        tools={false}
        onBack={() => setPrompt('discard')}
        footer={
          <View style={styles.footer}>
            <Button
              big
              fullWidth
              label={t('FLOW_AVATAR_DONE')}
              onPress={() => setPrompt('finish')}
            />
          </View>
        }
      />
      <Dialog
        visible={prompt === 'finish'}
        title={t('FLOW_FINISH_LOOK_TITLE')}
        body={t('FLOW_FINISH_LOOK_BODY')}
        buttons={[
          { label: t('COMMON_CANCEL'), tone: 'secondary', onPress: close },
          { label: t('FLOW_FINISH_LOOK_YES'), onPress: onDone },
        ]}
        onRequestClose={close}
      />
      <Dialog
        visible={prompt === 'discard'}
        title={t('FLOW_DISCARD_TITLE')}
        body={t('FLOW_DISCARD_BODY')}
        buttons={[
          { label: t('COMMON_CANCEL'), tone: 'secondary', onPress: close },
          { label: t('FLOW_DISCARD'), onPress: onDiscard },
        ]}
        onRequestClose={close}
      />
    </>
  );
}

const styles = StyleSheet.create({
  footer: {
    paddingTop: u(9),
    paddingBottom: u(43),
    paddingHorizontal: u(24),
    backgroundColor: colors.purple500,
  },
});
