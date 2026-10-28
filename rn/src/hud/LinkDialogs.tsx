import { Dialog } from '../ds';
import { t } from '../i18n';
import type { Segment } from './chatModel';

/** Confirm step before a tapped chat link leaves the world or jumps elsewhere. */
export function LinkDialogs({
  pending,
  onCancel,
  onConfirm,
}: {
  pending: Segment | null;
  onCancel: () => void;
  onConfirm: (segment: Segment) => void;
}) {
  const external = pending?.kind === 'url';
  const location = pending?.value ?? '';
  return (
    <Dialog
      visible={pending !== null}
      title={
        external
          ? t('CHAT_LINK_OPEN_TITLE')
          : t('CHAT_TELEPORT_TITLE', { location })
      }
      body={
        external
          ? t('CHAT_LINK_OPEN_BODY', { url: location })
          : t('CHAT_TELEPORT_BODY')
      }
      onRequestClose={onCancel}
      buttons={[
        { label: t('CHAT_LINK_CANCEL'), tone: 'secondary', onPress: onCancel },
        {
          label: external ? t('CHAT_LINK_OPEN') : t('CHAT_TELEPORT_GO'),
          onPress: () => pending && onConfirm(pending),
        },
      ]}
    />
  );
}
