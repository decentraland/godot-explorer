import { ReactNode } from 'react';
import { BackpackView } from './BackpackView';
import { BackpackApi, getBackpackApi } from './api';
import { BackpackState } from './backpackState';
import type { SubRow } from './BackpackChrome';
import { EmptyCopy, getEmptyCopy, showsCredits } from './platformPolicy';
import { useBackpack } from './useBackpack';
import { useCredits } from './useCredits';

export type BackpackScreenProps = {
  /** Bar under the sheet: the app's bottom navigation, or a Continue button. */
  footer: ReactNode;
  /** The entry flow and the world menu need a back button; the tab shell does not. */
  onBack?: () => void;
  /** Swaps the data source (previews, tests); defaults to the active api. */
  api?: BackpackApi;
  initial?: Partial<BackpackState>;
  /** Empty-state hint, normally from the platform's store policy. */
  emptyCopy?: EmptyCopy;
  /** Sub-category row: thumbnail tiles (default) or the older text pills. */
  subRow?: SubRow;
  /** Credits badge over the avatar; defaults to the platform's store policy. */
  credits?: boolean;
  tools?: boolean;
};

export function BackpackScreen({
  footer,
  onBack,
  api,
  initial,
  emptyCopy,
  subRow,
  credits = showsCredits(),
  tools,
}: BackpackScreenProps) {
  const controller = useBackpack({ api, initial });
  const balance = useCredits(api ?? getBackpackApi(), credits && tools !== false);
  return (
    <BackpackView
      controller={controller}
      copy={emptyCopy ?? getEmptyCopy()}
      subRow={subRow}
      credits={balance}
      footer={footer}
      onBack={onBack}
      tools={tools}
    />
  );
}
