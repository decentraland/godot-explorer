import { useEffect } from 'react';
import { BackHandler } from 'react-native';
import { BackpackScreen } from '../backpack/BackpackScreen';
import { TutorialStep } from '../core/native';
import { replayTutorial, useFtue } from '../ftue/store';
import { EquipHint } from '../ftue/TutorialLayer';
import { onIntent } from '../hud/hudSignals';
import { t } from '../i18n';
import { openMenu } from '../menu';
import { modalApi } from '../modals/bridge';
import { useNav } from './NavProvider';
import { registerScreen } from './registry';
import '../discover/routes';
import '../menu/register';
import '../profile';
import '../settings';

const TUTORIAL_TAB = { tab: 'extras' as const };

function BackpackRoute() {
  const { pop } = useNav();
  const step = useFtue().run?.step;
  // The tutorial's equip step opens on Extras (hats, eyewear...).
  const tutorial =
    step === TutorialStep.Backpack || step === TutorialStep.Equip;
  return (
    <>
      <BackpackScreen
        footer={null}
        onBack={pop}
        initial={tutorial ? TUTORIAL_TAB : undefined}
      />
      <EquipHint />
    </>
  );
}

registerScreen('backpack', () => <BackpackRoute />);

/** Back from the world asks first, so a slip of the thumb does not drop the player out of it.
 * Anything open on top (the menu, the chat, a modal) handled the key before this ran. */
export function confirmLeave(onLeave: () => void): void {
  const modals = modalApi();
  if (!modals) {
    onLeave();
    return;
  }
  modals.show({
    key: 'leave-world',
    title: t('WORLD_LEAVE_TITLE'),
    body: t('WORLD_LEAVE_BODY'),
    buttons: [
      { label: t('COMMON_CANCEL'), tone: 'secondary' },
      { label: t('WORLD_LEAVE_CONFIRM'), onPress: onLeave },
    ],
  });
}

/** Routes HUD requests to the screens that own them, and lets Android back leave the world. */
export function useAppWiring(onLeave: () => void) {
  const { push, pop, route } = useNav();
  const inWorld = route.name === 'world';

  useEffect(
    () =>
      onIntent(intent => {
        if (intent.type === 'open_menu') {
          openMenu();
        } else if (intent.type === 'open_profile') {
          push({ name: 'profile' });
        } else if (intent.type === 'replay_tutorial') {
          if (!inWorld) {
            pop();
          }
          replayTutorial();
        } else {
          push({ name: 'backpack' });
        }
      }),
    [push, pop, inWorld],
  );

  useEffect(() => {
    if (!inWorld) {
      return;
    }
    const sub = BackHandler.addEventListener('hardwareBackPress', () => {
      confirmLeave(onLeave);
      return true;
    });
    return () => sub.remove();
  }, [inWorld, onLeave]);
}
