// Backpack previews, one per manifest id; see docs/rn-ui-spec/reports/backpack-{a,b}.md.
import { useMemo } from 'react';
import { BottomNav, u } from '@dcl/ui-native';
import { registerPreview } from '../gallery/registry';
import { BackpackScreen } from './BackpackScreen';
import { LANDSCAPE, PORTRAIT, Spec } from './gallerySpecs';
import { createMockApi } from './mockApi';

const noop = () => {};
/** The Figma frames draw the nav bar 180 px tall: 154 px of bar plus the phone's bottom safe area. */
const FIGMA_BOTTOM_INSET = u(13);

function Preview({ spec, landscape }: { spec: Spec; landscape: boolean }) {
  const api = useMemo(() => createMockApi(spec.api), [spec.api]);
  return (
    <BackpackScreen
      api={api}
      initial={spec.initial}
      emptyCopy={spec.copy ?? 'marketplace_link'}
      subRow={spec.subRow}
      credits={spec.credits ?? false}
      onBack={landscape ? noop : undefined}
      footer={
        landscape ? null : (
          <BottomNav
            active="backpack"
            bottomInset={FIGMA_BOTTOM_INSET}
            onChange={noop}
          />
        )
      }
    />
  );
}

const register = (
  rows: [string, string, Spec][],
  orientation: 'portrait' | 'landscape',
) =>
  rows.forEach(([id, title, spec]) =>
    registerPreview(id, {
      title: `Backpack: ${title}`,
      orientation,
      render: () => (
        <Preview spec={spec} landscape={orientation === 'landscape'} />
      ),
      settleMs: 1200,
    }),
  );

register(PORTRAIT, 'portrait');
register(LANDSCAPE, 'landscape');
