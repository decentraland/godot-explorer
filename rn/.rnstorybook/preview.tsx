import type { Preview } from '@storybook/react-native';
import { PreviewFrame } from '../src/gallery/PreviewFrame';
import type { StoryPreview } from '../src/gallery/stories';

const preview: Preview = {
  // The screens place themselves inside the safe area; Storybook's own wrapper would add bars.
  parameters: { noSafeArea: true },
  decorators: [
    (Story, { parameters }) => {
      const { id, orientation, settleMs } = parameters.preview as StoryPreview;
      return (
        <PreviewFrame id={id} orientation={orientation} settleMs={settleMs}>
          <Story />
        </PreviewFrame>
      );
    },
  ],
};

export default preview;
