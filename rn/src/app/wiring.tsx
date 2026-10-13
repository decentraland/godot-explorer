import { useEffect } from 'react';
import { BackHandler } from 'react-native';
import { BackpackScreen } from '../backpack/BackpackScreen';
import { onIntent } from '../hud/hudSignals';
import { openMenu } from '../menu';
import { useNav } from './NavProvider';
import { registerScreen } from './registry';
import '../discover/routes';
import '../menu/register';
import '../profile';
import '../settings';

function BackpackRoute() {
  const { pop } = useNav();
  return <BackpackScreen footer={null} onBack={pop} />;
}

registerScreen('backpack', () => <BackpackRoute />);

/** Routes HUD requests to the screens that own them, and lets Android back leave the world. */
export function useAppWiring(onLeave: () => void) {
  const { push, route } = useNav();
  const inWorld = route.name === 'world';

  useEffect(
    () =>
      onIntent(intent => {
        if (intent.type === 'open_menu') {
          openMenu();
        } else if (intent.type === 'open_profile') {
          push({ name: 'profile' });
        } else {
          push({ name: 'backpack' });
        }
      }),
    [push],
  );

  useEffect(() => {
    if (!inWorld) {
      return;
    }
    const sub = BackHandler.addEventListener('hardwareBackPress', () => {
      onLeave();
      return true;
    });
    return () => sub.remove();
  }, [inWorld, onLeave]);
}
