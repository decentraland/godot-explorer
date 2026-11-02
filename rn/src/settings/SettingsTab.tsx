import { useNav } from '../app/NavProvider';
import type { ScreenProps } from '../app/registry';
import { SettingsScreen } from './SettingsScreen';

/** The registered 'settings' tab body; landscape back leaves Settings for Explore. */
export function SettingsTab({ footer }: ScreenProps<'settings'>) {
  const { setTab } = useNav();
  return <SettingsScreen footer={footer} onClose={() => setTab('explore')} />;
}
