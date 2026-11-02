import { AccountSection } from './AccountSection';
import { AudioSection } from './AudioSection';
import { DevToolsSection } from './DevToolsSection';
import { GameplaySection } from './GameplaySection';
import { GraphicsSection } from './GraphicsSection';
import { HelpSection } from './HelpSection';
import { LanguageSection } from './LanguageSection';
import type { SectionKey } from './sections';
import { StorageSection } from './StorageSection';
import type { SettingsModel } from './useSettings';

/** Which states a gallery preview wants on first render. */
export type PreviewState = {
  /** Which dropdown starts open: the profile or skybox list in Graphics, the only list elsewhere. */
  open?: 'profile' | 'skybox' | 'list';
  dialog?: 'signOut' | 'delete' | 'deleteFailed';
};

export function SectionBody({
  section,
  m,
  preview,
  onSignedOut,
}: {
  section: SectionKey;
  m: SettingsModel;
  preview?: PreviewState;
  onSignedOut?: () => void;
}) {
  switch (section) {
    case 'graphics':
      return <GraphicsSection m={m} open={preview?.open} />;
    case 'audio':
      return <AudioSection m={m} />;
    case 'gameplay':
      return <GameplaySection m={m} />;
    case 'language':
      return <LanguageSection m={m} defaultOpen={preview?.open === 'list'} />;
    case 'account':
      return (
        <AccountSection
          m={m}
          onSignedOut={onSignedOut}
          initialDialog={preview?.dialog}
        />
      );
    case 'help':
      return <HelpSection m={m} />;
    case 'storage':
      return <StorageSection m={m} defaultOpen={preview?.open === 'list'} />;
    case 'dev':
      return <DevToolsSection m={m} />;
  }
}
