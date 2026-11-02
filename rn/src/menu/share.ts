import { Share } from 'react-native';
import { t } from '../i18n';
import { sceneLink } from './api';
import type { SceneInfo } from './types';

export const sceneShareMessage = (scene: SceneInfo): string =>
  t('MENU_SHARE_SCENE', {
    scene: scene.title,
    link: sceneLink(scene.linkQuery),
  });

/** Opens the OS share sheet; a dismissed or failed sheet is not an error worth surfacing. */
export async function shareScene(scene: SceneInfo): Promise<void> {
  try {
    await Share.share({ message: sceneShareMessage(scene) });
  } catch {
    // The user cancelled or no share target exists.
  }
}

export const REPORT_CONTENT_URL =
  'https://docs.google.com/forms/d/e/1FAIpQLSdD31D0GKROyxmrvM-KVStqdhyqF430crjaTtpemEiAqCHQbg/viewform';
