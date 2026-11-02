import { ReactNode } from 'react';
import { BackpackView } from './BackpackView';
import { BackpackApi } from './api';
import { BackpackState } from './backpackState';
import type { SubRow } from './BackpackChrome';
import { EmptyCopy, getEmptyCopy } from './platformPolicy';
import { useBackpack } from './useBackpack';

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
};

export function BackpackScreen({
  footer,
  onBack,
  api,
  initial,
  emptyCopy,
  subRow,
}: BackpackScreenProps) {
  const controller = useBackpack({ api, initial });
  return (
    <BackpackView
      controller={controller}
      copy={emptyCopy ?? getEmptyCopy()}
      subRow={subRow}
      footer={footer}
      onBack={onBack}
    />
  );
}
