import { t } from '../i18n';
import { modalApi } from '../modals/bridge';
import type { BackpackApi } from './api';

/** Saves the avatar; a failure asks to try again, since the Backpack is already gone. */
export function saveOnLeave(api: BackpackApi): void {
  api.saveAvatar().catch(() =>
    modalApi()?.show({
      key: 'backpack-save-failed',
      title: t('BACKPACK_SAVE_FAILED'),
      body: t('BACKPACK_SAVE_FAILED_BODY'),
      buttons: [
        { label: t('COMMON_CANCEL'), tone: 'secondary' },
        { label: t('BACKPACK_TRY_AGAIN'), onPress: () => saveOnLeave(api) },
      ],
    }),
  );
}
